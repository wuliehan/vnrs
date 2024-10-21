use crate::vnrs::trader::{
    constant::{Direction, Interval, Offset},
    object::{BarData, OrderData, TickData, TradeData},
};
use chrono::{DateTime, Duration, Local, NaiveDateTime};
use std::{
    collections::HashMap,
    ffi::{c_char, CString, OsStr, OsString},
    sync::{Arc, OnceLock},
};

use super::{backtesting::BacktestingEngine, template::CtaTemplate};

pub const APP_NAME: &'static str = "CtaStrategy";
pub const STOPORDER_PREFIX: &'static str = "STOP";

#[derive(Clone, Copy)]
pub enum StopOrderStatus {
    WAITING,
    CANCELLED,
    TRIGGERED,
}

impl Default for StopOrderStatus {
    fn default() -> Self {
        StopOrderStatus::WAITING
    }
}

pub enum EngineType {
    LIVE,
    BACKTESTING,
}
impl Default for EngineType {
    fn default() -> Self {
        EngineType::LIVE
    }
}

#[derive(PartialEq, Eq)]
pub enum BacktestingMode {
    BAR = 1,
    TICK = 2,
}

impl Default for BacktestingMode {
    fn default() -> Self {
        BacktestingMode::BAR
    }
}

#[derive(Default, Clone)]
pub struct StopOrder {
    pub vt_symbol: String,
    pub direction: Direction,
    pub offset: Offset,
    pub price: f64,
    pub volume: f64,
    pub stop_orderid: String,
    pub strategy_name: String,
    pub datetime: NaiveDateTime,
    pub lock: bool,
    pub net: bool,
    pub vt_orderids: Vec<String>,
    pub status: StopOrderStatus,
}

pub const EVENT_CTA_LOG: &'static str = "eCtaLog";
pub const EVENT_CTA_STRATEGY: &'static str = "eCtaStrategy";
pub const EVENT_CTA_STOPORDER: &'static str = "eCtaStopOrder";

pub static INTERVAL_DELTA_MAP: OnceLock<HashMap<Interval, Duration>> = OnceLock::new();

pub fn get_interval_delta_map() -> &'static HashMap<Interval, Duration> {
    INTERVAL_DELTA_MAP.get_or_init(|| {
        vec![
            (Interval::TICK, Duration::milliseconds(1)),
            (Interval::MINUTE, Duration::minutes(1)),
            (Interval::HOUR, Duration::hours(1)),
            (Interval::DAILY, Duration::days(1)),
        ]
        .into_iter()
        .collect()
    })
}

#[derive(Default)]
pub struct ExternClass {
    pub filename: OsString,
    lib: Option<libloading::Library>,
    pub func_new: Option<
        libloading::Symbol<
            'static,
            extern "C" fn(
                cta_engine: *const VTable,
                strategy_name: *const c_char,
                vt_symbol: *const c_char,
                setting: *const c_char,
            ) -> *mut CtaTemplate,
        >,
    >,
    pub func_on_init: Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, usize)>>,
    pub func_on_start: Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate)>>,
    pub func_on_stop: Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate)>>,
    pub func_on_tick:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const TickData)>>,
    pub func_on_bar:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const BarData)>>,
    pub func_on_order:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const OrderData)>>,
    pub func_on_trade:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const TradeData)>>,
    pub func_on_stop_order:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const StopOrder)>>,
    pub func_get_inited_mut:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate) -> *mut bool>>,
    pub func_get_trading_mut:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate) -> *mut bool>>,
    pub func_get_pos_mut:
        Option<libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate) -> *mut f64>>,
}

impl ExternClass {
    pub fn new<P: AsRef<OsStr>>(filename: P) -> Self {
        unsafe {
            let the_lib = libloading::Library::new(filename.as_ref().to_owned()).unwrap();
            let func_new = std::mem::transmute::<
                libloading::Symbol<
                    '_,
                    unsafe extern "C" fn(
                        cta_engine: *const VTable,
                        strategy_name: *const c_char,
                        vt_symbol: *const c_char,
                        setting: *const c_char,
                    ) -> *mut CtaTemplate,
                >,
                libloading::Symbol<
                    'static,
                    extern "C" fn(
                        cta_engine: *const VTable,
                        strategy_name: *const c_char,
                        vt_symbol: *const c_char,
                        setting: *const c_char,
                    ) -> *mut CtaTemplate,
                >,
            >(the_lib.get(b"abi_new").unwrap());
            let func_on_init = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate, usize)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, usize)>,
            >(the_lib.get(b"abi_on_init").unwrap());
            let func_on_start = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate)>,
            >(the_lib.get(b"abi_on_start").unwrap());
            let func_on_stop = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate)>,
            >(the_lib.get(b"abi_on_stop").unwrap());
            let func_on_tick = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate, *const TickData)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const TickData)>,
            >(the_lib.get(b"abi_on_tick").unwrap());
            let func_on_bar = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate, *const BarData)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const BarData)>,
            >(the_lib.get(b"abi_on_bar").unwrap());
            let func_on_order = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate, *const OrderData)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const OrderData)>,
            >(the_lib.get(b"abi_on_order").unwrap());
            let func_on_trade = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate, *const TradeData)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const TradeData)>,
            >(the_lib.get(b"abi_on_trade").unwrap());
            let func_on_stop_order = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate, *const StopOrder)>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate, *const StopOrder)>,
            >(the_lib.get(b"abi_on_stop_order").unwrap());
            let func_get_inited_mut = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate) -> *mut bool>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate) -> *mut bool>,
            >(the_lib.get(b"abi_get_inited_mut").unwrap());
            let func_get_trading_mut = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate) -> *mut bool>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate) -> *mut bool>,
            >(the_lib.get(b"abi_get_trading_mut").unwrap());
            let func_get_pos_mut = std::mem::transmute::<
                libloading::Symbol<'_, unsafe extern "C" fn(*mut CtaTemplate) -> *mut f64>,
                libloading::Symbol<'static, extern "C" fn(*mut CtaTemplate) -> *mut f64>,
            >(the_lib.get(b"abi_get_pos_mut").unwrap());

            ExternClass {
                filename: filename.as_ref().to_owned(),
                lib: Some(the_lib),
                func_new: Some(func_new),
                func_on_init: Some(func_on_init),
                func_on_start: Some(func_on_start),
                func_on_stop: Some(func_on_stop),
                func_on_tick: Some(func_on_tick),
                func_on_bar: Some(func_on_bar),
                func_on_order: Some(func_on_order),
                func_on_trade: Some(func_on_trade),
                func_on_stop_order: Some(func_on_stop_order),
                func_get_inited_mut: Some(func_get_inited_mut),
                func_get_trading_mut: Some(func_get_trading_mut),
                func_get_pos_mut: Some(func_get_pos_mut),
            }
        }
    }
}

impl Drop for ExternClass {
    fn drop(&mut self) {
        println!("class dropped");
    }
}

#[derive(Default)]
pub struct ExternInstance {
    class: Arc<ExternClass>,
    instance: Option<*mut CtaTemplate>,
    pub strategy_name: String,
}

impl ExternInstance {
    pub fn new(
        class: Arc<ExternClass>,
        cta_engine: *const VTable,
        strategy_name: String,
        vt_symbol: &str,
        setting: &str,
    ) -> Self {
        let inst;
        inst = class.func_new.as_ref().unwrap()(
            cta_engine,
            CString::new(strategy_name.clone()).unwrap().as_ptr(),
            CString::new(vt_symbol).unwrap().as_ptr(),
            CString::new(setting).unwrap().as_ptr(),
        );
        ExternInstance {
            class: class.clone(),
            instance: Some(inst),
            strategy_name: strategy_name,
        }
    }

    pub fn on_init(&self, cta_engine_ptr: usize) {
        self.class.func_on_init.as_ref().unwrap()(self.instance.unwrap(), cta_engine_ptr)
    }

    pub fn on_start(&self) {
        self.class.func_on_start.as_ref().unwrap()(self.instance.unwrap())
    }

    pub fn on_stop(&self) {
        self.class.func_on_stop.as_ref().unwrap()(self.instance.unwrap())
    }

    pub fn on_tick(&self, tick: &TickData) {
        self.class.func_on_tick.as_ref().unwrap()(self.instance.unwrap(), tick)
    }

    pub fn on_bar(&self, bar: &BarData) {
        self.class.func_on_bar.as_ref().unwrap()(self.instance.unwrap(), bar)
    }

    pub fn on_order(&self, order: &OrderData) {
        self.class.func_on_order.as_ref().unwrap()(self.instance.unwrap(), order)
    }

    pub fn on_trade(&self, trade: &TradeData) {
        self.class.func_on_trade.as_ref().unwrap()(self.instance.unwrap(), trade)
    }

    pub fn on_stop_order(&self, stop_order: &StopOrder) {
        self.class.func_on_stop_order.as_ref().unwrap()(self.instance.unwrap(), stop_order)
    }

    pub fn get_inited_mut(&self) -> &mut bool {
        unsafe { &mut *self.class.func_get_inited_mut.as_ref().unwrap()(self.instance.unwrap()) }
    }

    pub fn get_trading_mut(&self) -> &mut bool {
        unsafe { &mut *self.class.func_get_trading_mut.as_ref().unwrap()(self.instance.unwrap()) }
    }

    pub fn get_pos_mut(&self) -> &mut f64 {
        unsafe { &mut *self.class.func_get_pos_mut.as_ref().unwrap()(self.instance.unwrap()) }
    }
}

impl Drop for ExternInstance {
    fn drop(&mut self) {
        if self.class.lib.is_none() {
            return;
        }
        unsafe {
            let func_drop: libloading::Symbol<unsafe extern "C" fn(*mut CtaTemplate)> =
                self.class.lib.as_ref().unwrap().get(b"abi_drop").unwrap();
            func_drop(self.instance.unwrap());
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VTable {
    pub abi_load_bar: extern "C" fn(
        usize,
        *const c_char,
        i64,
        Interval,
        // Callable,
        bool,
    ) -> *mut Vec<BarData>,
    pub abi_drop_vec_bar_data: extern "C" fn(vec: *mut Vec<BarData>),
    pub abi_send_order: extern "C" fn(
        usize,
        *mut CtaTemplate,
        Direction,
        Offset,
        f64,
        f64,
        bool,
        bool,
        bool,
    ) -> *mut Vec<String>,
    pub abi_drop_vec_string: extern "C" fn(vec: *mut Vec<String>),
    pub abi_cancel_all: extern "C" fn(this: usize, strategy: *mut CtaTemplate),
}
