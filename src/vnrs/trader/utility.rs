use std::sync::OnceLock;

use chrono::Timelike;
use libloading;
use rust_decimal::prelude::*;

use crate::vnrs::trader::constant::Exchange;

use super::object::{BarData, MixData, TickData};

///:return: (symbol, exchange)
pub fn extract_vt_symbol(vt_symbol: &str) -> (String, Exchange) {
    let vec_str: Vec<&str> = vt_symbol.rsplitn(2, ".").collect();
    let (symbol, exchange_str) = (vec_str[1], vec_str[0]);
    return (
        symbol.to_string(),
        Exchange::from_str(exchange_str).unwrap(),
    );
}

///Round price to price tick value.

pub fn round_to(value: f64, target: f64) -> f64 {
    let value: Decimal = Decimal::from_str(&value.to_string()).unwrap();
    let target: Decimal = Decimal::from_str(&target.to_string()).unwrap();
    ((value / target).round() * target)
        .to_string()
        .parse()
        .unwrap()
}

#[derive(Debug)]
pub struct BarGenerator {
    // bar: Option<BarData>,
    on_bar: fn(usize, &BarData),
    // interval: Interval,
    // interval_count: i64,

    // hour_bar: Option<BarData>,
    // daily_bar: Option<BarData>,

    // window: i64,
    // window_bar: Option<BarData>,
    // on_window_bar: Callable = on_window_bar

    // last_tick: Option<TickData>,

    // daily_end: time = daily_end
}

impl BarGenerator {
    pub fn new(on_bar: fn(usize, &BarData)) -> Self {
        BarGenerator { on_bar }
    }

    // fn update_tick(self, tick: TickData){
    //     let new_minute = false;

    //     // Filter tick data with 0 last price
    //     if !tick.last_price{
    //         return
    //     }

    //     if self.bar.is_none(){
    //         new_minute = true
    //     }
    //     else if
    //         (self.bar.unwrap().datetime.minute() != tick.datetime.minute())
    //         || (self.bar.unwrap().datetime.hour() != tick.datetime.hour())
    //     {
    //         self.bar.unwrap().datetime = self.bar.unwrap().datetime.replace(
    //             second=0, microsecond=0
    //         )
    //         self.on_bar(self.bar);

    //         new_minute = true
    //     }
    //     if new_minute:
    //         self.bar = BarData(
    //             symbol=tick.symbol,
    //             exchange=tick.exchange,
    //             interval=Interval.MINUTE,
    //             datetime=tick.datetime,
    //             gateway_name=tick.gateway_name,
    //             open_price=tick.last_price,
    //             high_price=tick.last_price,
    //             low_price=tick.last_price,
    //             close_price=tick.last_price,
    //             open_interest=tick.open_interest
    //         )
    //     else:
    //         self.bar.high_price = max(self.bar.high_price, tick.last_price)
    //         if tick.high_price > self.last_tick.high_price:
    //             self.bar.high_price = max(self.bar.high_price, tick.high_price)

    //         self.bar.low_price = min(self.bar.low_price, tick.last_price)
    //         if tick.low_price < self.last_tick.low_price:
    //             self.bar.low_price = min(self.bar.low_price, tick.low_price)

    //         self.bar.close_price = tick.last_price
    //         self.bar.open_interest = tick.open_interest
    //         self.bar.datetime = tick.datetime

    //     if self.last_tick:
    //         volume_change: float = tick.volume - self.last_tick.volume
    //         self.bar.volume += max(volume_change, 0)

    //         turnover_change: float = tick.turnover - self.last_tick.turnover
    //         self.bar.turnover += max(turnover_change, 0)

    //     self.last_tick = tick
}

struct TaLib {
    pub sma: libloading::Symbol<'static, unsafe extern "C" fn(*const f64, i32, i32, *mut f64)>,
}

static TALIB_DYLIB: OnceLock<libloading::Library> = OnceLock::new();
fn get_talib_dylib() -> &'static libloading::Library {
    TALIB_DYLIB.get_or_init(|| unsafe { libloading::Library::new("TALIBDYLIB").unwrap() })
}
static TALIB: OnceLock<TaLib> = OnceLock::new();
fn get_talib() -> &'static TaLib {
    TALIB.get_or_init(|| unsafe {
        let sma: libloading::Symbol<'static, unsafe extern "C" fn(*const f64, i32, i32, *mut f64)> =
            get_talib_dylib().get(b"sma").unwrap();

        TaLib { sma }
    })
}

#[derive(Debug)]
pub struct ArrayManager {
    pub count: usize,
    pub size: usize,
    pub inited: bool,

    pub open_array: Vec<f64>,
    pub high_array: Vec<f64>,
    pub low_array: Vec<f64>,
    pub close_array: Vec<f64>,
    pub volume_array: Vec<f64>,
    pub turnover_array: Vec<f64>,
    pub open_interest_array: Vec<f64>,
}

impl ArrayManager {
    pub fn new(size: usize) -> Self {
        ArrayManager {
            count: 0,
            size,
            inited: false,
            open_array: vec![0f64; size],
            high_array: vec![0f64; size],
            low_array: vec![0f64; size],
            close_array: vec![0f64; size],
            volume_array: vec![0f64; size],
            turnover_array: vec![0f64; size],
            open_interest_array: vec![0f64; size],
        }
    }

    pub fn update_bar(&mut self, bar: &BarData) {
        self.count += 1;
        if (!self.inited) && self.count >= self.size {
            self.inited = true;
        }
        self.open_array.remove(0);
        self.high_array.remove(0);
        self.low_array.remove(0);
        self.close_array.remove(0);
        self.volume_array.remove(0);
        self.turnover_array.remove(0);
        self.open_interest_array.remove(0);

        self.open_array.push(bar.open_price);
        self.high_array.push(bar.high_price);
        self.low_array.push(bar.low_price);
        self.close_array.push(bar.close_price);
        self.volume_array.push(bar.volume);
        self.turnover_array.push(bar.turnover);
        self.open_interest_array.push(bar.open_interest);
    }

    pub fn sma_array(&mut self, n: i64) -> Vec<f64> {
        unsafe {
            let mut ret = Vec::new();
            ret.resize(self.close_array.len(), 0f64);
            (get_talib().sma)(
                self.close_array.as_ptr(),
                self.close_array.len() as i32,
                n as i32,
                ret.as_mut_ptr(),
            );
            ret
        }
    }
}
