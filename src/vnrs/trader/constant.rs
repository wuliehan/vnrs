/*!General constant enums used in the trading platform. */
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Direction {
    NONE,
    LONG,
    SHORT,
    NET,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::NONE
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum Offset {
    NONE,
    OPEN,
    CLOSE,
    CLOSETODAY,
    CLOSEYESTERDAY,
}

impl Default for Offset {
    fn default() -> Self {
        Offset::NONE
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    SUBMITTING,
    NOTTRADED,
    PARTTRADED,
    ALLTRADED,
    CANCELLED,
    REJECTED,
}

impl Default for Status {
    fn default() -> Self {
        Self::SUBMITTING
    }
}

pub enum Product {
    EQUITY,
    FUTURES,
    OPTION,
    INDEX,
    FOREX,
    SPOT,
    ETF,
    BOND,
    WARRANT,
    SPREAD,
    FUND,
    CFD,
    SWAP,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderType {
    LIMIT,
    MARKET,
    STOP,
    FAK,
    FOK,
    RFQ,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::LIMIT
    }
}

pub enum OptionType {
    CALL,
    PUT,
}

#[derive(Debug, Clone, Copy, EnumString, Display)]
pub enum Exchange {
    // Chinese
    CFFEX, // China Financial Futures Exchange
    SHFE,  // Shanghai Futures Exchange
    CZCE,  // Zhengzhou Commodity Exchange
    DCE,   // Dalian Commodity Exchange
    INE,   // Shanghai International Energy Exchange
    GFEX,  // Guangzhou Futures Exchange
    SSE,   // Shanghai Stock Exchange
    SZSE,  // Shenzhen Stock Exchange
    BSE,   // Beijing Stock Exchange
    SHHK,  // Shanghai-HK Stock Connect
    SZHK,  // Shenzhen-HK Stock Connect
    SGE,   // Shanghai Gold Exchange
    WXE,   // Wuxi Steel Exchange
    CFETS, // CFETS Bond Market Maker Trading System
    XBOND, // CFETS X-Bond Anonymous Trading System

    // Global
    SMART,    // Smart Router for US stocks
    NYSE,     // New York Stock Exchnage
    NASDAQ,   // Nasdaq Exchange
    ARCA,     // ARCA Exchange
    EDGEA,    // Direct Edge Exchange
    ISLAND,   // Nasdaq Island ECN
    BATS,     // Bats Global Markets
    IEX,      // The Investors Exchange
    AMEX,     // American Stock Exchange
    TSE,      // Toronto Stock Exchange
    NYMEX,    // New York Mercantile Exchange
    COMEX,    // COMEX of CME
    GLOBEX,   // Globex of CME
    IDEALPRO, // Forex ECN of Interactive Brokers
    CME,      // Chicago Mercantile Exchange
    ICE,      // Intercontinental Exchange
    SEHK,     // Stock Exchange of Hong Kong
    HKFE,     // Hong Kong Futures Exchange
    SGX,      // Singapore Global Exchange
    CBOT,     // Chicago Board of Trade
    CBOE,     // Chicago Board Options Exchange
    CFE,      // CBOE Futures Exchange
    DME,      // Dubai Mercantile Exchange
    EUREX,    // Eurex Exchange
    APEX,     // Asia Pacific Exchange
    LME,      // London Metal Exchange
    BMD,      // Bursa Malaysia Derivatives
    TOCOM,    // Tokyo Commodity Exchange
    EUNX,     // Euronext Exchange
    KRX,      // Korean Exchange
    OTC,      // OTC Product (Forex/CFD/Pink Sheet Equity)
    IBKRATS,  // Paper Trading Exchange of IB
    OKX,

    // Special Function
    LOCAL, // For local generated data
}

impl Default for Exchange {
    fn default() -> Self {
        Exchange::LOCAL
    }
}

pub enum Currency {
    USD,
    HKD,
    CNY,
    CAD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum Interval {
    NONE,
    MINUTE,
    HOUR,
    DAILY,
    WEEKLY,
    TICK,
}
impl Default for Interval {
    fn default() -> Self {
        Interval::NONE
    }
}
