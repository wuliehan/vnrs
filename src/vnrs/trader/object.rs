/*！Basic data structure used for general trading function in the trading platform.*/
use chrono::NaiveDateTime;
use log::Level;
use std::{
    collections::{HashMap, HashSet},
    ffi::{c_char, CString},
    sync::{Mutex, OnceLock},
};

use super::constant::{
    Direction, Exchange, Interval, Offset, OptionType, OrderType, Product, Status,
};

pub static ACTIVE_STATUSES: OnceLock<HashSet<Status>> = OnceLock::new();
pub fn get_active_statuses() -> &'static HashSet<Status> {
    ACTIVE_STATUSES.get_or_init(|| {
        vec![Status::SUBMITTING, Status::NOTTRADED, Status::PARTTRADED]
            .into_iter()
            .collect()
    })
}

#[derive(Debug, Default)]

pub struct TickData {
    pub gateway_name: &'static str,

    pub symbol: String,
    pub exchange: Exchange,
    pub datetime: NaiveDateTime,

    pub name: String,
    pub volume: f64,
    pub turnover: f64,
    pub open_interest: f64,
    pub last_price: f64,
    pub last_volume: f64,
    pub limit_up: f64,
    pub limit_down: f64,

    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub pre_close: f64,

    pub bid_price_1: f64,
    pub bid_price_2: f64,
    pub bid_price_3: f64,
    pub bid_price_4: f64,
    pub bid_price_5: f64,

    pub ask_price_1: f64,
    pub ask_price_2: f64,
    pub ask_price_3: f64,
    pub ask_price_4: f64,
    pub ask_price_5: f64,

    pub bid_volume_1: f64,
    pub bid_volume_2: f64,
    pub bid_volume_3: f64,
    pub bid_volume_4: f64,
    pub bid_volume_5: f64,

    pub ask_volume_1: f64,
    pub ask_volume_2: f64,
    pub ask_volume_3: f64,
    pub ask_volume_4: f64,
    pub ask_volume_5: f64,

    localtime: NaiveDateTime,
}
impl TickData {
    pub fn vt_symbol(&self) -> String {
        format!("{}.{}", self.symbol, self.exchange.to_string())
    }
}

#[derive(Debug, Default, Clone)]
pub struct BarData {
    pub gateway_name: &'static str,

    pub symbol: String,
    pub exchange: Exchange,
    pub datetime: NaiveDateTime,

    pub interval: Interval,
    pub volume: f64,
    pub turnover: f64,
    pub open_interest: f64,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
}

impl BarData {
    pub fn vt_symbol(&self) -> String {
        format!("{}.{}", self.symbol, self.exchange.to_string())
    }
}

#[derive(Debug)]
pub enum MixData {
    TickData(TickData),
    BarData(BarData),
}

#[derive(Debug, Default, Clone)]
pub struct OrderData {
    pub gateway_name: &'static str,

    pub symbol: String,
    pub exchange: Exchange,
    pub orderid: String,

    pub type_: OrderType,
    pub direction: Direction,
    pub offset: Offset,
    pub price: f64,
    pub volume: f64,
    pub traded: f64,
    pub status: Status,
    pub datetime: NaiveDateTime,
    pub reference: String,

}

impl OrderData {
    pub fn vt_symbol(&self) -> String {
        format!("{}.{}", self.symbol, self.exchange.to_string())
    }

    pub fn vt_orderid(&self) -> String {
        format!("{}.{}", self.gateway_name, self.orderid)
    }

    pub fn is_active(&self) -> bool {
        get_active_statuses().contains(&self.status)
    }

    // fn create_cancel_request(&self) -> CancelRequest {
    //     CancelRequest {
    //         orderid: self.orderid,
    //         symbol: self.symbol,
    //         exchange: self.exchange,
    //     }
    // }
}

#[derive(Debug, Clone)]
pub struct TradeData {
    pub gateway_name: &'static str,

    pub symbol: String,
    pub exchange: Exchange,
    pub orderid: String,
    pub tradeid: String,
    pub direction: Direction,

    pub offset: Offset,
    pub price: f64,
    pub volume: f64,
    pub datetime: NaiveDateTime,
}

impl TradeData {
    pub fn vt_symbol(&self) -> String {
        format!("{}.{}", self.symbol, self.exchange.to_string())
    }

    pub fn vt_orderid(&self) -> String {
        format!("{}.{}", self.gateway_name, self.orderid)
    }

    pub fn vt_tradeid(&self) -> String {
        format!("{}.{}", self.gateway_name, self.tradeid)
    }
}

pub struct PositionData {
    symbol: String,
    exchange: Exchange,
    direction: Direction,

    volume: f64,
    frozen: f64,
    price: f64,
    pnl: f64,
    yd_volume: f64,
}
//     def __post_init__(self) -> None:
//         """"""
//         self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"
//         self.vt_positionid: String, = f"{self.gateway_name}.{self.vt_symbol}.{self.direction.value}"

pub struct AccountData {
    accountid: String,

    balance: f64,
    frozen: f64,
}
//     def __post_init__(self) -> None:
//         """"""
//         self.available: float = self.balance - self.frozen
//         self.vt_accountid: String, = f"{self.gateway_name}.{self.accountid}"

pub struct LogData {
    msg: String,
    level: Level,
}

//     def __post_init__(self) -> None:
//         """"""
//         self.time: datetime = datetime.now()

pub struct ContractData {
    symbol: String,
    exchange: Exchange,
    name: String,
    product: Product,
    size: f64,
    pricetick: f64,

    min_volume: f64,      // minimum trading volume of the contract
    stop_supported: bool, // whether server supports stop order
    net_position: bool,   // whether gateway uses net position volume
    history_data: bool,   // whether gateway provides bar history data

    option_strike: f64,
    option_underlying: String, // vt_symbol of underlying contract
    option_type: OptionType,
    option_listed: NaiveDateTime,
    option_expiry: NaiveDateTime,
    option_portfolio: String,
    option_index: String, // for identifying options with same strike price
}

//     def __post_init__(self) -> None:
//         """"""
//         self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"

// @dataclass
// class QuoteData(BaseData):
//     """
//     Quote data contains information for tracking lastest status
//     of a specific quote.
//     """

//     symbol: String,
//     exchange: Exchange
//     quoteid: String,

//     bid_price: f64,.0
//     bid_volume: int = 0
//     ask_price: f64,.0
//     ask_volume: int = 0
//     bid_offset: Offset = Offset.NONE
//     ask_offset: Offset = Offset.NONE
//     status: Status = Status.SUBMITTING
//     datetime: datetime = None
//     reference: String, = ""

//     def __post_init__(self) -> None:
//         """"""
//         self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"
//         self.vt_quoteid: String, = f"{self.gateway_name}.{self.quoteid}"

//     def is_active(self) -> bool:
//         """
//         Check if the quote is active.
//         """
//         return self.status in ACTIVE_STATUSES

//     def create_cancel_request(self) -> "CancelRequest":
//         """
//         Create cancel request object from quote.
//         """
//         req: CancelRequest = CancelRequest(
//             orderid=self.quoteid, symbol=self.symbol, exchange=self.exchange
//         )
//         return req

// @dataclass
// class SubscribeRequest:
//     """
//     Request sending to specific gateway for subscribing tick data update.
//     """

//     symbol: String,
//     exchange: Exchange

//     def __post_init__(self) -> None:
//         """"""
//         self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"

// @dataclass
// class OrderRequest:
//     """
//     Request sending to specific gateway for creating a new order.
//     """

//     symbol: String,
//     exchange: Exchange
//     direction: Direction
//     type: OrderType
//     volume: float
//     price: f64,
//     offset: Offset = Offset.NONE
//     reference: String, = ""

//     def __post_init__(self) -> None:
//         """"""
//         self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"

//     def create_order_data(self, orderid: String,, gateway_name: String,) -> OrderData:
//         """
//         Create order data from request.
//         """
//         order: OrderData = OrderData(
//             symbol=self.symbol,
//             exchange=self.exchange,
//             orderid=orderid,
//             type=self.type,
//             direction=self.direction,
//             offset=self.offset,
//             price=self.price,
//             volume=self.volume,
//             reference=self.reference,
//             gateway_name=gateway_name,
//         )
//         return order

pub struct CancelRequest {
    orderid: String,
    symbol: String,
    exchange: Exchange,
}
impl CancelRequest {
    fn __post_init__(self) {
        // self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"
    }
}

// @dataclass
// class HistoryRequest:
//     """
//     Request sending to specific gateway for querying history data.
//     """

//     symbol: String,
//     exchange: Exchange
//     start: datetime
//     end: datetime = None
//     interval: Interval = None

//     def __post_init__(self) -> None:
//         """"""
//         self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"

// @dataclass
// class QuoteRequest:
//     """
//     Request sending to specific gateway for creating a new quote.
//     """

//     symbol: String,
//     exchange: Exchange
//     bid_price: float
//     bid_volume: int
//     ask_price: float
//     ask_volume: int
//     bid_offset: Offset = Offset.NONE
//     ask_offset: Offset = Offset.NONE
//     reference: String, = ""

//     def __post_init__(self) -> None:
//         """"""
//         self.vt_symbol: String, = f"{self.symbol}.{self.exchange.value}"

//     def create_quote_data(self, quoteid: String,, gateway_name: String,) -> QuoteData:
//         """
//         Create quote data from request.
//         """
//         quote: QuoteData = QuoteData(
//             symbol=self.symbol,
//             exchange=self.exchange,
//             quoteid=quoteid,
//             bid_price=self.bid_price,
//             bid_volume=self.bid_volume,
//             ask_price=self.ask_price,
//             ask_volume=self.ask_volume,
//             bid_offset=self.bid_offset,
//             ask_offset=self.ask_offset,
//             reference=self.reference,
//             gateway_name=gateway_name,
//         )
//         return quote
