use chrono;
use chrono::{Datelike, Days, Local, NaiveDate, NaiveDateTime, TimeDelta};
use polars::lazy::dsl::{col, lit, when};
use polars::prelude::*;
use std::any::Any;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use strum::EnumString;

use super::base::{
    get_interval_delta_map, BacktestingMode, EngineType, ExternClass, ExternInstance, StopOrder,
    StopOrderStatus, VTable, INTERVAL_DELTA_MAP, STOPORDER_PREFIX,
};
use super::template::CtaTemplate;
use crate::vnrs::trader::constant::{Direction, Exchange, Interval, Offset, Status};
use crate::vnrs::trader::database::get_database;
use crate::vnrs::trader::object::{BarData, MixData, OrderData, TickData, TradeData};
use crate::vnrs::trader::utility::{extract_vt_symbol, round_to};

#[derive(Default)]
pub struct BacktestingEngine {
    engine_type: EngineType,
    gateway_name: &'static str,

    vt_symbol: String,
    symbol: String,
    exchange: Exchange,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub rate: f64,
    pub slippage: f64,
    pub size: f64,
    pub pricetick: f64,
    pub capital: f64,
    risk_free: f64,
    annual_days: i64,
    half_life: i64,
    mode: BacktestingMode,

    strategy_class: Arc<ExternClass>,
    strategy: ExternInstance,
    tick: TickData,
    bar: BarData,
    datetime: NaiveDateTime,

    pub interval: Interval,
    days: i32,
    //     callback: Callable = None
    history_data: Arc<RwLock<Vec<MixData>>>,
    stop_order_count: i64,
    stop_orders: HashMap<String, Rc<RefCell<StopOrder>>>,
    active_stop_orders: HashMap<String, Rc<RefCell<StopOrder>>>,

    limit_order_count: i64,
    limit_orders: HashMap<String, Rc<RefCell<OrderData>>>,
    active_limit_orders: HashMap<String, Rc<RefCell<OrderData>>>,

    trade_count: i64,
    trades: HashMap<String, Rc<RefCell<TradeData>>>,

    logs: Vec<String>,
    daily_results: HashMap<NaiveDate, DailyResult>,
    daily_df: Option<Rc<RefCell<DataFrame>>>,
    v_table: Option<VTable>,
}

impl BacktestingEngine {
    pub fn new() -> Self {
        let mut this = BacktestingEngine {
            engine_type: EngineType::BACKTESTING,
            gateway_name: "BACKTESTING",
            v_table: None,
            ..Default::default()
        };
        this.v_table = Some(VTable {
            abi_load_bar: BacktestingEngine::abi_load_bar,
            abi_drop_vec_bar_data: BacktestingEngine::abi_drop_vec_bar_data,
            abi_send_order: BacktestingEngine::abi_send_order,
            abi_drop_vec_string: BacktestingEngine::abi_drop_vec_string,
            abi_cancel_all: BacktestingEngine::abi_cancel_all,
        });
        eprintln!("this p:{:p}", &this);
        this
    }
    pub fn set_parameters(
        &mut self,
        vt_symbol: &str,
        interval: Interval,
        start: NaiveDateTime,
        end: NaiveDateTime,
        rate: f64,
        slippage: f64,
        size: f64,
        pricetick: f64,
        capital: f64,
        mode: BacktestingMode,
        risk_free: f64,
        annual_days: i64,
        half_life: i64,
    ) {
        self.vt_symbol = vt_symbol.to_string();
        let v: Vec<&str> = vt_symbol.split(".").collect();
        self.symbol = v[0].to_string();
        self.exchange = Exchange::from_str(v[1]).unwrap();
        self.interval = interval;
        self.start = start;
        self.end = end;
        self.rate = rate;
        self.slippage = slippage;
        self.size = size;
        self.pricetick = pricetick;
        self.capital = capital;
        self.mode = mode;
        self.risk_free = risk_free;
        self.annual_days = annual_days;
        self.half_life = half_life;
    }

    fn clear_data(&mut self) {
        // self.strategy = None;
        self.tick = TickData::default();
        self.bar = BarData::default();
        self.datetime = NaiveDateTime::default();

        self.stop_order_count = 0;
        self.stop_orders.clear();
        self.active_stop_orders.clear();

        self.limit_order_count = 0;
        self.limit_orders.clear();
        self.active_limit_orders.clear();

        self.trade_count = 0;
        self.trades.clear();

        self.logs.clear();
        self.daily_results.clear();
    }

    pub fn add_strategy(&mut self, strategy_class: Arc<ExternClass>, setting: &str) {
        self.strategy_class = strategy_class.clone();
        let strategy_name = strategy_class
            .clone()
            .filename
            .clone()
            .into_string()
            .unwrap();
        self.strategy = ExternInstance::new(
            self.strategy_class.clone(),
            (self.v_table.as_ref().unwrap() as *const VTable as usize) as *const VTable,
            strategy_name,
            &self.vt_symbol,
            setting,
        );
    }

    pub fn load_data(&mut self) {
        self.output("开始加载历史数据");
        if self.end == NaiveDateTime::default() {
            self.end = Local::now().naive_local();
        }
        if self.start >= self.end {
            self.output("起始日期必须小于结束日期");
            return;
        }
        self.history_data.write().unwrap().clear(); // Clear previously loaded history data

        // Load 30 days of data each time and allow for progress update
        let total_days = (self.end - self.start).num_days();
        let progress_days = (total_days / 10).max(1);
        let progress_delta = TimeDelta::days(progress_days);
        let interval_delta = get_interval_delta_map()
            .get(&self.interval)
            .unwrap()
            .clone();

        let mut start = self.start;
        let mut end = self.start + progress_delta;
        let mut progress: f64 = 0.0;

        while start < self.end {
            let progress_bar = "#".repeat((progress * 10.0 + 1.0) as usize);
            self.output(
                format!(
                    "加载进度：{progress_bar} [{progress:.0}%]",
                    progress_bar = progress_bar,
                    progress = progress * 100.0
                )
                .as_str(),
            );

            end = end.min(self.end); // Make sure end time stays within set range

            if self.mode == BacktestingMode::BAR {
                let data: Vec<BarData> =
                    load_bar_data(&self.symbol, self.exchange, self.interval, start, end);
                self.history_data
                    .write()
                    .unwrap()
                    .extend(data.into_iter().map(|bar_data| MixData::BarData(bar_data)));
            }
            //     else:
            //         data: List[TickData] = load_tick_data(
            //             self.symbol,
            //             self.exchange,
            //             start,
            //             end
            //         )

            progress += progress_days as f64 / total_days as f64;
            progress = progress.min(1.0);

            start = end + interval_delta;
            end += progress_delta
        }

        self.output(
            format!(
                "历史数据加载完成，数据量：{}",
                self.history_data.read().unwrap().len()
            )
            .as_str(),
        );
    }

    pub fn run_backtesting(&mut self) {
        let func: fn(&mut BacktestingEngine, &MixData);
        if self.mode == BacktestingMode::BAR {
            func = BacktestingEngine::new_bar;
        } else {
            func = BacktestingEngine::new_tick;
        }
        self.strategy
            .on_init(self as *const BacktestingEngine as usize);
        *self.strategy.get_inited_mut() = true;
        self.output("策略初始化完成");

        self.strategy.on_start();
        *self.strategy.get_trading_mut() = true;
        self.output("开始回放历史数据");

        let total_size: usize = self.history_data.read().unwrap().len();
        let batch_size: usize = (total_size / 10).max(1);

        let cloned_history_data = self.history_data.clone();
        let ref_vec_history_data = cloned_history_data.read().unwrap();
        for (ix, i) in (0..total_size).step_by(batch_size).enumerate() {
            let batch_data;
            if i + batch_size >= self.history_data.read().unwrap().len() {
                batch_data = &ref_vec_history_data[i..];
            } else {
                batch_data = &ref_vec_history_data[i..i + batch_size];
            }
            let this = self as *const BacktestingEngine as *mut BacktestingEngine;
            for data in batch_data {
                func(self, data);
            }
            let progress = (ix as f64 / 10.0).min(1.0);
            let progress_bar = "=".repeat(ix + 1);
            self.output(&format!(
                "回放进度：{} [{:.0}%]",
                progress_bar,
                progress * 100.0
            ));
        }
        self.strategy.on_stop();
        self.output("历史数据回放结束");
        eprintln!("{}", self.trade_count);
    }

    pub fn calculate_result(&mut self) -> Rc<RefCell<DataFrame>> {
        self.output("开始计算逐日盯市盈亏");

        if self.trades.len() == 0 {
            self.output("回测成交记录为空");
        }

        // Add trade data into daily reuslt.
        for trade in self.trades.values() {
            let d = trade.borrow().datetime.date();
            let daily_result = self.daily_results.get_mut(&d).unwrap();
            daily_result.add_trade(trade.clone())
        }

        // Calculate daily result by iteration.
        let mut pre_close = 0.0;
        let mut start_pos = 0.0;

        let mut sorted: Vec<&mut DailyResult> = self.daily_results.values_mut().collect();
        sorted.sort_by_key(|item| item.date);
        for daily_result in sorted {
            daily_result.calculate_pnl(pre_close, start_pos, self.size, self.rate, self.slippage);

            pre_close = daily_result.close_price;
            start_pos = daily_result.end_pos;
        }

        // Generate dataframe
        let mut date: Vec<NaiveDate> = Vec::new();
        let mut close_price = Vec::new();
        let mut pre_close = Vec::new();
        let mut trade_count = Vec::new();
        let mut start_pos = Vec::new();
        let mut end_pos = Vec::new();
        let mut turnover = Vec::new();
        let mut commission = Vec::new();
        let mut slippage = Vec::new();
        let mut trading_pnl = Vec::new();
        let mut holding_pnl = Vec::new();
        let mut total_pnl = Vec::new();
        let mut net_pnl = Vec::new();
        for daily_result in self.daily_results.values() {
            date.push(daily_result.date);
            close_price.push(daily_result.close_price);
            pre_close.push(daily_result.pre_close);
            trade_count.push(daily_result.trade_count);
            start_pos.push(daily_result.start_pos);
            end_pos.push(daily_result.end_pos);
            turnover.push(daily_result.turnover);
            commission.push(daily_result.commission);
            slippage.push(daily_result.slippage);
            trading_pnl.push(daily_result.trading_pnl);
            holding_pnl.push(daily_result.holding_pnl);
            total_pnl.push(daily_result.total_pnl);
            net_pnl.push(daily_result.net_pnl);
        }
        self.daily_df = Some(Rc::new(RefCell::new(df!(
            "date"=>&date,"close_price"=>&close_price,"pre_close"=>&pre_close,"trade_count"=>&trade_count,
            "start_pos"=>&start_pos,"end_pos"=>&end_pos,"turnover"=>&turnover,"commission"=>&commission,
            "slippage"=>&slippage,"trading_pnl"=>&trading_pnl,"holding_pnl"=>&holding_pnl,
            "total_pnl"=>&total_pnl,"net_pnl"=>&net_pnl
        ).unwrap())));
        (*self.daily_df.clone().unwrap())
            .borrow_mut()
            .sort_in_place(["date"], Default::default())
            .unwrap();

        self.output("逐日盯市盈亏计算完成");
        self.daily_df.clone().unwrap()
    }

    pub fn calculate_statistics(&mut self, mut df: Option<Rc<RefCell<DataFrame>>>, output: bool) {
        self.output("开始计算策略统计指标");

        // Check DataFrame input exterior
        if df.is_none() {
            df = self.daily_df.clone();
        }

        // Init all statistics default value
        let mut start_date = NaiveDate::default();
        let mut end_date = NaiveDate::default();
        let mut total_days: i64 = 0;
        let mut profit_days: i64 = 0;
        let mut loss_days: i64 = 0;
        let mut end_balance: f64 = 0.0;
        let mut max_drawdown: f64 = 0.0;
        let mut max_ddpercent: f64 = 0.0;
        let mut max_drawdown_duration: i64 = 0;
        let mut total_net_pnl: f64 = 0.0;
        let mut daily_net_pnl: f64 = 0.0;
        let mut total_commission: f64 = 0.0;
        let mut daily_commission: f64 = 0.0;
        let mut total_slippage: f64 = 0.0;
        let mut daily_slippage: f64 = 0.0;
        let mut total_turnover: f64 = 0.0;
        let mut daily_turnover: f64 = 0.0;
        let mut total_trade_count: i64 = 0;
        let mut daily_trade_count: f64 = 0.0;
        let mut total_return: f64 = 0.0;
        let mut annual_return: f64 = 0.0;
        let mut daily_return: f64 = 0.0;
        let mut return_std: f64 = 0.0;
        let mut sharpe_ratio: f64 = 0.0;
        let mut ewm_sharpe: f64 = 0.0;
        let mut return_drawdown_ratio: f64 = 0.0;

        // Check if balance is always positive
        let positive_balance: bool = false;
        let mut dfo: DataFrame;

        if !df.is_none() {
            // Calculate balance related time series data
            let cloned_df = df.clone().unwrap();
            let refmut_df = (*cloned_df).borrow_mut();
            dfo = refmut_df
                .clone()
                .lazy()
                .with_column(col("net_pnl").alias("balance").cum_sum(false) + lit(self.capital))
                .collect()
                .unwrap();

            // When balance falls below 0, set daily return to 0
            dfo = dfo
                .clone()
                .lazy()
                .with_column(
                    col("balance")
                        .alias("pre_balance")
                        .shift_and_fill(1, self.capital),
                )
                .collect()
                .unwrap();
            dfo = dfo
                .clone()
                .lazy()
                .with_column(col("balance").alias("x") / col("pre_balance"))
                .collect()
                .unwrap();
            let x: Vec<f64> = dfo["x"].f64().unwrap().into_no_null_iter().collect();
            let x: Vec<f64> = x
                .iter()
                .map(|x| if *x < 0.0 { 0.0 } else { f64::ln(*x) })
                .collect();
            dfo.with_column(Series::new("return", &x)).unwrap();

            let balance: Vec<f64> = dfo["balance"].f64().unwrap().into_no_null_iter().collect();
            let mut highlevel = Vec::new();
            let mut drawdown = Vec::new();
            let mut ddpercent = Vec::new();
            let mut max = 0f64;
            balance.iter().for_each(|x| {
                max = max.max(*x);
                highlevel.push(max);
                let dd = *x - max;
                drawdown.push(dd);
                ddpercent.push(dd / max * 100f64);
            });
            dfo.with_column(Series::new("highlevel", &highlevel))
                .unwrap();
            dfo.with_column(Series::new("drawdown", &drawdown)).unwrap();
            dfo.with_column(Series::new("ddpercent", &ddpercent))
                .unwrap();

            // All balance value needs to be positive
            let positive_balance = balance.iter().all(|x| *x > 0f64);
            if !positive_balance {
                self.output("回测中出现爆仓（资金小于等于0），无法计算策略统计指标");
            }

            // Calculate statistics value
            if positive_balance {
                // Calculate statistics value
                let dates: Vec<i32> = dfo["date"].date().unwrap().into_no_null_iter().collect();
                start_date = NaiveDate::from_ymd_opt(1970, 1, 1)
                    .unwrap()
                    .checked_add_days(Days::new(*dates.first().unwrap() as u64))
                    .unwrap();
                end_date = NaiveDate::from_ymd_opt(1970, 1, 1)
                    .unwrap()
                    .checked_add_days(Days::new(*dates.last().unwrap() as u64))
                    .unwrap();

                total_days = dfo.height() as i64;
                profit_days = dfo
                    .clone()
                    .lazy()
                    .filter(col("net_pnl").gt(0))
                    .collect()
                    .unwrap()
                    .height() as i64;
                loss_days = dfo
                    .clone()
                    .lazy()
                    .filter(col("net_pnl").lt(0))
                    .collect()
                    .unwrap()
                    .height() as i64;

                end_balance = dfo["balance"].f64().unwrap().last().unwrap();
                max_drawdown = dfo["drawdown"].f64().unwrap().min().unwrap();
                max_ddpercent = dfo["ddpercent"].f64().unwrap().min().unwrap();

                let max_drawdown_end_idx = dfo["drawdown"].arg_min();
                let max_drawdown_end = max_drawdown_end_idx.map(|idx| {
                    NaiveDate::from_ymd_opt(1970, 1, 1)
                        .unwrap()
                        .checked_add_days(Days::new(dates[idx] as u64))
                        .unwrap()
                });

                if max_drawdown_end.is_some() {
                    let before_max_drawdown = dfo
                        .clone()
                        .lazy()
                        .filter(col("date").lt_eq(lit(max_drawdown_end.unwrap())))
                        .collect()
                        .unwrap();
                    let max_drawdown_start_idx = before_max_drawdown["balance"].arg_max().unwrap();
                    let max_drawdown_start = NaiveDate::from_ymd_opt(1970, 1, 1)
                        .unwrap()
                        .checked_add_days(Days::new(dates[max_drawdown_start_idx] as u64))
                        .unwrap();
                    max_drawdown_duration =
                        ((max_drawdown_end.unwrap() - max_drawdown_start) as TimeDelta).num_days();
                } else {
                    max_drawdown_duration = 0;
                }

                total_net_pnl = dfo["net_pnl"].sum().unwrap();
                daily_net_pnl = total_net_pnl / total_days as f64;

                total_commission = dfo["commission"].sum().unwrap();
                daily_commission = total_commission / total_days as f64;

                total_slippage = dfo["slippage"].sum().unwrap();
                daily_slippage = total_slippage / total_days as f64;

                total_turnover = dfo["turnover"].sum().unwrap();
                daily_turnover = total_turnover / total_days as f64;

                total_trade_count = dfo["trade_count"].sum().unwrap();
                daily_trade_count = total_trade_count as f64 / total_days as f64;

                total_return = (end_balance / self.capital - 1.0) * 100.0;
                annual_return = total_return / (total_days as f64) * self.annual_days as f64;
                daily_return = dfo["return"].mean().unwrap() * 100.0;
                return_std = dfo["return"].std(0).unwrap() * 100.0;

                if return_std != 0.0 {
                    let daily_risk_free = self.risk_free / f64::sqrt(self.annual_days as f64);
                    sharpe_ratio = (daily_return - daily_risk_free) / return_std
                        * f64::sqrt(self.annual_days as f64);
                }

                //     ewm_window: ExponentialMovingWindow = df["return"].ewm(halflife=self.half_life)
                //     ewm_mean: Series = ewm_window.mean() * 100
                //     ewm_std: Series = ewm_window.std() * 100
                //     ewm_sharpe: float = ((ewm_mean - daily_risk_free) / ewm_std)[-1] * np.sqrt(self.annual_days)
                // else:
                //     sharpe_ratio: float = 0
                //     ewm_sharpe: float = 0

                if max_ddpercent != 0.0 {
                    return_drawdown_ratio = -total_return / max_ddpercent;
                } else {
                    return_drawdown_ratio = 0.0;
                }
            }
        }
        // Output
        if output {
            self.output(&"-".repeat(30));
            self.output(&format!("首个交易日：\t{}", start_date));
            self.output(&format!("最后交易日：\t{}", end_date));

            self.output(&format!("总交易日：\t{}", total_days));
            self.output(&format!("盈利交易日：\t{}", profit_days));
            self.output(&format!("亏损交易日：\t{}", loss_days));

            self.output(&format!("起始资金：\t{:.2}", self.capital));
            self.output(&format!("结束资金：\t{:.2}", end_balance));

            self.output(&format!("总收益率：\t{:.2}%", total_return));
            self.output(&format!("年化收益：\t{:.2}%", annual_return));
            self.output(&format!("最大回撤: \t{:.2}", max_drawdown));
            self.output(&format!("百分比最大回撤: {:.2}%", max_ddpercent));
            self.output(&format!("最长回撤天数: \t{}", max_drawdown_duration));

            self.output(&format!("总盈亏：\t{:.2}", total_net_pnl));
            self.output(&format!("总手续费：\t{:.2}", total_commission));
            self.output(&format!("总滑点：\t{:.2}", total_slippage));
            self.output(&format!("总成交金额：\t{:.2}", total_turnover));
            self.output(&format!("总成交笔数：\t{}", total_trade_count));

            self.output(&format!("日均盈亏：\t{:.2}", daily_net_pnl));
            self.output(&format!("日均手续费：\t{:.2}", daily_commission));
            self.output(&format!("日均滑点：\t{:.2}", daily_slippage));
            self.output(&format!("日均成交金额：\t{:.2}", daily_turnover));
            self.output(&format!("日均成交笔数：\t{}", daily_trade_count));

            self.output(&format!("日均收益率：\t{:.2}%", daily_return));
            self.output(&format!("收益标准差：\t{:.2}%", return_std));
            self.output(&format!("Sharpe Ratio：\t{:.2}", sharpe_ratio));
            // self.output(&format!("EWM Sharpe：\t{:.2}", ewm_sharpe));
            self.output(&format!("收益回撤比：\t{:.2}", return_drawdown_ratio));
        }
    }

    fn update_daily_close(&mut self, price: f64) {
        let d = self.datetime.date();

        self.daily_results
            .entry(d)
            .and_modify(|e| e.close_price = price)
            .or_insert(DailyResult::new(d, price));
    }

    fn new_bar(&mut self, bar: &MixData) {
        if let MixData::BarData(bar) = bar {
            self.bar = bar.clone();
            self.datetime = self.bar.datetime;

            self.cross_limit_order();
            self.cross_stop_order();
            self.strategy.on_bar(bar);

            self.update_daily_close(self.bar.close_price);
        }
    }

    fn new_tick(&mut self, tick: &MixData) {}

    fn cross_limit_order(&mut self) {
        let long_cross_price;
        let short_cross_price;
        let long_best_price;
        let short_best_price;
        if self.mode == BacktestingMode::BAR {
            long_cross_price = self.bar.low_price;
            short_cross_price = self.bar.high_price;
            long_best_price = self.bar.open_price;
            short_best_price = self.bar.open_price;
        } else {
            long_cross_price = self.tick.ask_price_1;
            short_cross_price = self.tick.bid_price_1;
            long_best_price = long_cross_price;
            short_best_price = short_cross_price;
        }

        let value_list: Vec<Rc<RefCell<OrderData>>> = self
            .active_limit_orders
            .values()
            .map(|v| v.clone())
            .collect();
        for order in value_list {
            let mut order = (*order).borrow_mut();
            // Push order update with status "not traded" (pending).
            if order.status == Status::SUBMITTING {
                order.status = Status::NOTTRADED;
                self.strategy.on_order(&order);
            }

            // Check whether limit orders can be filled.
            let long_cross: bool = order.direction == Direction::LONG
                && order.price >= long_cross_price
                && long_cross_price > 0.0;

            let short_cross: bool = order.direction == Direction::SHORT
                && order.price <= short_cross_price
                && short_cross_price > 0.0;

            if !long_cross && !short_cross {
                continue;
            }

            // Push order udpate with status "all traded" (filled).
            order.traded = order.volume;
            order.status = Status::ALLTRADED;
            self.strategy.on_order(&order);

            if self.active_limit_orders.contains_key(&order.vt_orderid()) {
                self.active_limit_orders.remove(&order.vt_orderid());
            }

            // Push trade update
            self.trade_count += 1;

            let trade_price;
            let pos_change;
            if long_cross {
                trade_price = order.price.min(long_best_price);
                pos_change = order.volume;
            } else {
                trade_price = order.price.max(short_best_price);
                pos_change = -order.volume;
            }

            let trade = Rc::new(RefCell::new(TradeData {
                symbol: order.symbol.to_string(),
                exchange: order.exchange,
                orderid: order.orderid.to_string(),
                tradeid: self.trade_count.to_string(),
                direction: order.direction,
                offset: order.offset,
                price: trade_price,
                volume: order.volume,
                datetime: self.datetime,
                gateway_name: self.gateway_name,
            }));

            *self.strategy.get_pos_mut() += pos_change;
            self.strategy.on_trade(&trade.borrow());

            self.trades
                .insert(trade.borrow().vt_tradeid(), trade.clone());
        }
    }

    fn cross_stop_order(&mut self) {
        let long_cross_price;
        let short_cross_price;
        let long_best_price;
        let short_best_price;
        if self.mode == BacktestingMode::BAR {
            long_cross_price = self.bar.high_price;
            short_cross_price = self.bar.low_price;
            long_best_price = self.bar.open_price;
            short_best_price = self.bar.open_price;
        } else {
            long_cross_price = self.tick.last_price;
            short_cross_price = self.tick.last_price;
            long_best_price = long_cross_price;
            short_best_price = short_cross_price;
        }

        let value_list: Vec<Rc<RefCell<StopOrder>>> = self
            .active_stop_orders
            .values()
            .map(|v| v.clone())
            .collect();
        for stop_order in value_list {
            let mut stop_order = (*stop_order).borrow_mut();
            // Check whether stop order can be triggered.
            let long_cross: bool =
                stop_order.direction == Direction::LONG && stop_order.price <= long_cross_price;

            let short_cross: bool =
                stop_order.direction == Direction::SHORT && stop_order.price >= short_cross_price;

            if !long_cross && !short_cross {
                continue;
            }

            // Create order data.
            self.limit_order_count += 1;

            let order = Rc::new(RefCell::new(OrderData {
                symbol: self.symbol.to_string(),
                exchange: self.exchange,
                orderid: self.limit_order_count.to_string(),
                direction: stop_order.direction,
                offset: stop_order.offset,
                price: stop_order.price,
                volume: stop_order.volume,
                traded: stop_order.volume,
                status: Status::ALLTRADED,
                gateway_name: self.gateway_name,
                datetime: self.datetime,
                ..Default::default()
            }));

            self.limit_orders
                .insert(order.borrow().vt_orderid(), order.clone());

            // Create trade data.
            let trade_price;
            let pos_change;
            if long_cross {
                trade_price = stop_order.price.max(long_best_price);
                pos_change = order.borrow().volume;
            } else {
                trade_price = stop_order.price.min(short_best_price);
                pos_change = -order.borrow().volume;
            }

            self.trade_count += 1;

            let trade = Rc::new(RefCell::new(TradeData {
                symbol: order.borrow().symbol.to_string(),
                exchange: order.borrow().exchange,
                orderid: order.borrow().orderid.clone(),
                tradeid: self.trade_count.to_string(),
                direction: order.borrow().direction,
                offset: order.borrow().offset,
                price: trade_price,
                volume: order.borrow().volume,
                datetime: self.datetime,
                gateway_name: self.gateway_name,
            }));

            self.trades
                .insert(trade.borrow().vt_tradeid(), trade.clone());

            // Update stop order.
            stop_order.vt_orderids.push(order.borrow().vt_orderid());
            stop_order.status = StopOrderStatus::TRIGGERED;

            if self
                .active_stop_orders
                .contains_key(&stop_order.stop_orderid)
            {
                self.active_stop_orders.remove(&stop_order.stop_orderid);
            }

            // Push update to strategy.
            self.strategy.on_stop_order(&stop_order);
            self.strategy.on_order(&order.borrow());

            *self.strategy.get_pos_mut() += pos_change;
            self.strategy.on_trade(&trade.borrow());
        }
    }

    fn load_bar(
        &mut self,
        vt_symbol: &str,
        days: i64,
        interval: Interval,
        // callback: Callable,
        use_database: bool,
    ) -> Vec<BarData> {
        let init_end = self.start - get_interval_delta_map()[&interval];
        let init_start = self.start.checked_sub_days(Days::new(days as u64)).unwrap();

        let (symbol, exchange) = extract_vt_symbol(vt_symbol);

        let bars: Vec<BarData> = load_bar_data(&symbol, exchange, interval, init_start, init_end);

        return bars;
    }

    fn load_tick(&mut self, vt_symbol: &str, days: i64) -> Vec<TickData> {
        vec![]
    }

    fn send_order(
        &mut self,
        strategy: *mut CtaTemplate,
        direction: Direction,
        offset: Offset,
        price: f64,
        volume: f64,
        stop: bool,
        lock: bool,
        net: bool,
    ) -> Vec<String> {
        let price: f64 = round_to(price, self.pricetick);
        let vt_orderid;
        if stop {
            vt_orderid = self.send_stop_order(direction, offset, price, volume);
        } else {
            vt_orderid = self.send_limit_order(direction, offset, price, volume);
        }
        vec![vt_orderid]
    }

    fn send_stop_order(
        &mut self,
        direction: Direction,
        offset: Offset,
        price: f64,
        volume: f64,
    ) -> String {
        self.stop_order_count += 1;

        let stop_order = Rc::new(RefCell::new(StopOrder {
            vt_symbol: self.vt_symbol.to_string(),
            direction: direction,
            offset: offset,
            price: price,
            volume: volume,
            datetime: self.datetime,
            stop_orderid: format!("{}.{}", STOPORDER_PREFIX, self.stop_order_count),
            strategy_name: self.strategy.strategy_name.clone(),
            ..Default::default()
        }));

        self.active_stop_orders
            .insert(stop_order.borrow().stop_orderid.clone(), stop_order.clone());
        self.stop_orders
            .insert(stop_order.borrow().stop_orderid.clone(), stop_order.clone());

        let ret = stop_order.borrow().stop_orderid.clone();
        ret
    }

    fn send_limit_order(
        &mut self,
        direction: Direction,
        offset: Offset,
        price: f64,
        volume: f64,
    ) -> String {
        self.limit_order_count += 1;

        let order = Rc::new(RefCell::new(OrderData {
            symbol: self.symbol.to_string(),
            exchange: self.exchange,
            orderid: self.limit_order_count.to_string(),
            direction: direction,
            offset: offset,
            price: price,
            volume: volume,
            status: Status::SUBMITTING,
            gateway_name: self.gateway_name,
            datetime: self.datetime,
            ..Default::default()
        }));

        self.active_limit_orders
            .insert(order.borrow().vt_orderid(), order.clone());
        self.limit_orders
            .insert(order.borrow().vt_orderid(), order.clone());

        let ret = order.borrow().vt_orderid();
        ret
    }

    ///Cancel order by vt_orderid.
    fn cancel_order(&mut self, strategy: *mut CtaTemplate, vt_orderid: String) {
        if vt_orderid.starts_with(STOPORDER_PREFIX) {
            self.cancel_stop_order(strategy, vt_orderid);
        } else {
            self.cancel_limit_order(strategy, vt_orderid);
        }
    }

    fn cancel_stop_order(&mut self, strategy: *mut CtaTemplate, vt_orderid: String) {
        if !self.active_stop_orders.contains_key(&vt_orderid) {
            return;
        }
        let stop_order = self.active_stop_orders.remove(&vt_orderid).unwrap();

        (*stop_order).borrow_mut().status = StopOrderStatus::CANCELLED;
        self.strategy.on_stop_order(&stop_order.borrow());
    }

    fn cancel_limit_order(&mut self, strategy: *mut CtaTemplate, vt_orderid: String) {
        if !self.active_limit_orders.contains_key(&vt_orderid) {
            return;
        }
        let order = self.active_limit_orders.remove(&vt_orderid).unwrap();

        (*order).borrow_mut().status = Status::CANCELLED;
        self.strategy.on_order(&order.borrow());
    }

    ///Cancel all orders, both limit and stop.
    fn cancel_all(&mut self, strategy: *mut CtaTemplate) {
        let vt_orderids: Vec<String> = self.active_limit_orders.keys().map(|k| k.clone()).collect();
        for vt_orderid in vt_orderids {
            self.cancel_limit_order(strategy, vt_orderid);
        }

        let stop_orderids: Vec<String> =
            self.active_stop_orders.keys().map(|k| k.clone()).collect();
        for vt_orderid in stop_orderids {
            self.cancel_stop_order(strategy, vt_orderid);
        }
    }

    fn write_log(&mut self, msg: &str) {
        let msg = format!("{}\t{}", self.datetime, msg);
        self.logs.push(msg);
    }

    fn output(&self, msg: &str) {
        println!("{datetime}\t{msg}", datetime = Local::now(), msg = msg);
    }

    pub extern "C" fn abi_load_bar(
        this: usize,
        vt_symbol: *const c_char,
        days: i64,
        interval: Interval,
        // callback: Callable,
        use_database: bool,
    ) -> *mut Vec<BarData> {
        unsafe {
            let s = CStr::from_ptr(vt_symbol).to_owned().into_string().unwrap();
            Box::into_raw(Box::new(
                std::mem::transmute::<usize, &mut BacktestingEngine>(this).load_bar(
                    &s,
                    days,
                    interval,
                    use_database,
                ),
            ))
        }
    }

    pub extern "C" fn abi_drop_vec_bar_data(vec: *mut Vec<BarData>) {
        drop(unsafe { Box::from_raw(vec) });
    }

    pub extern "C" fn abi_send_order(
        this: usize,
        strategy: *mut CtaTemplate,
        direction: Direction,
        offset: Offset,
        price: f64,
        volume: f64,
        stop: bool,
        lock: bool,
        net: bool,
    ) -> *mut Vec<String> {
        unsafe {
            Box::into_raw(Box::new(
                std::mem::transmute::<usize, &mut BacktestingEngine>(this)
                    .send_order(strategy, direction, offset, price, volume, stop, lock, net),
            ))
        }
    }

    pub extern "C" fn abi_drop_vec_string(vec: *mut Vec<String>) {
        drop(unsafe { Box::from_raw(vec) });
    }

    pub extern "C" fn abi_cancel_all(this: usize, strategy: *mut CtaTemplate) {
        unsafe {
            std::mem::transmute::<usize, &mut BacktestingEngine>(this).cancel_all(strategy);
        }
    }
}

#[derive(Default)]
struct DailyResult {
    date: NaiveDate,
    close_price: f64,
    pre_close: f64,

    trades: Vec<Rc<RefCell<TradeData>>>,
    trade_count: i64,

    start_pos: f64,
    end_pos: f64,

    turnover: f64,
    commission: f64,
    slippage: f64,

    trading_pnl: f64,
    holding_pnl: f64,
    total_pnl: f64,
    net_pnl: f64,
}

impl DailyResult {
    pub fn new(date: NaiveDate, close_price: f64) -> Self {
        DailyResult {
            date,
            close_price,
            ..Default::default()
        }
    }

    pub fn add_trade(&mut self, trade: Rc<RefCell<TradeData>>) {
        self.trades.push(trade)
    }

    fn calculate_pnl(
        &mut self,
        pre_close: f64,
        start_pos: f64,
        size: f64,
        rate: f64,
        slippage: f64,
    ) {
        // If no pre_close provided on the first day,
        // use value 1 to avoid zero division error
        if pre_close != 0.0 {
            self.pre_close = pre_close;
        } else {
            self.pre_close = 1.0;
        }

        // Holding pnl is the pnl from holding position at day start
        self.start_pos = start_pos;
        self.end_pos = start_pos;

        self.holding_pnl = self.start_pos * (self.close_price - self.pre_close) * size;

        // Trading pnl is the pnl from new trade during the day
        self.trade_count = self.trades.len() as i64;

        for trade in &self.trades {
            let pos_change;
            if trade.borrow().direction == Direction::LONG {
                pos_change = trade.borrow().volume;
            } else {
                pos_change = -trade.borrow().volume;
            }

            self.end_pos += pos_change;

            let turnover = trade.borrow().volume * size * trade.borrow().price;
            self.trading_pnl += pos_change * (self.close_price - trade.borrow().price) * size;
            self.slippage += trade.borrow().volume * size * slippage;

            self.turnover += turnover;
            self.commission += turnover * rate;
        }

        // Net pnl takes account of commission and slippage cost
        self.total_pnl = self.trading_pnl + self.holding_pnl;
        self.net_pnl = self.total_pnl - self.commission - self.slippage;
    }
}

fn load_bar_data(
    symbol: &str,
    exchange: Exchange,
    interval: Interval,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Vec<BarData> {
    let db = get_database();

    return db.load_bar_data(symbol, exchange, interval, start, end);
}
