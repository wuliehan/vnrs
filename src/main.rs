use std::{
    ffi::{c_char, CStr, CString},
    ptr,
    sync::Arc, time::Instant,
};

use ::vnrs::vnrs_ctastrategy::{
    base::{BacktestingMode, ExternClass},
    template::{CtaEngineTable, CtaTemplate},
};
use ::vnrs::{vnrs::trader::constant::Interval, vnrs_ctastrategy::backtesting::BacktestingEngine};
use chrono::NaiveDateTime;

pub mod vnrs;
pub mod vnrs_ctastrategy;

pub extern "C" fn api_print_log(msg: *const c_char) {
    unsafe {
        println!("{}", CStr::from_ptr(msg).to_owned().into_string().unwrap());
    }
}

fn main() {
    /*
    let cstring: CString;
    unsafe {
        let lib = libloading::Library::new("double_ma_strategy").unwrap();
        let func: libloading::Symbol<unsafe extern "C" fn() -> *const c_char> =
            lib.get(b"abi_variables").unwrap();
        cstring = CStr::from_ptr(func()).to_owned();
    }
    let author = cstring.into_string().unwrap();
    println!("{}", author);

    unsafe {
        let lib = libloading::Library::new("double_ma_strategy").unwrap();
        let func_new: libloading::Symbol<
            unsafe extern "C" fn(
                cta_engine: usize,
                strategy_name: *const c_char,
                vt_symbol: *const c_char,
                setting: *const c_char,
            ) -> *mut CtaTemplate,
        > = lib.get(b"abi_new").unwrap();
        let ptr_table = [api_print_log as usize];
        let s: *mut CtaTemplate = func_new(
            &ptr_table[0] as *const usize as usize,
            CString::new("testStrategy").unwrap().as_ptr(),
            CString::new("IF888.LOCAL").unwrap().as_ptr(),
            CString::new("").unwrap().as_ptr(),
        );

        let func_on_init: libloading::Symbol<unsafe extern "C" fn(*mut CtaTemplate)> =
            lib.get(b"abi_on_init").unwrap();
        func_on_init(s);

        let func_drop: libloading::Symbol<unsafe extern "C" fn(*mut CtaTemplate)> =
            lib.get(b"abi_drop").unwrap();
        func_drop(s);
    }

    */
    let mut engine = BacktestingEngine::new();
    // engine.set_parameters(
    //     "000905.LOCAL",
    //     Interval::DAILY,
    //     NaiveDateTime::parse_from_str("1999-12-24 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
    //     NaiveDateTime::parse_from_str("2024-6-4 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
    //     2.5e-5,
    //     0.2,
    //     300.0,
    //     0.2,
    //     1000000.0,
    //     BacktestingMode::BAR,
    //     0.0,
    //     240,
    //     120,
    // );
    engine.set_parameters(
        "ETH.LOCAL",
        Interval::MINUTE,
        NaiveDateTime::parse_from_str("2020-1-22 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        NaiveDateTime::parse_from_str("2020-12-31 23:59:59", "%Y-%m-%d %H:%M:%S").unwrap(),
        2.5e-5,
        0.05,
        1.0,
        0.01,
        1000000.0,
        BacktestingMode::BAR,
        0.0,
        240,
        120,
    );
    engine.add_strategy(
        Arc::new(ExternClass::new("double_ma_strategy")),
        "fast_window:10,slow_window:20",
    );
    engine.load_data();
    let beg=Instant::now();
    engine.run_backtesting();
    let dur=Instant::now()-beg;
    engine.calculate_result();
    engine.calculate_statistics(None, true);
    eprintln!("{:?}",dur);
}
