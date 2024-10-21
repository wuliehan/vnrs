use super::setting::{get_settings, SETTINGS};
use env_logger::builder;
use log::{self};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio;

use chrono::NaiveDateTime;

use super::constant::{Exchange, Interval};
use super::object::BarData;

pub static DBMAP: Mutex<GlobalDBMap> = Mutex::new(GlobalDBMap::new());

pub struct GlobalDBMap {
    sqlite: Option<Arc<SqliteDatabase>>,
}

impl GlobalDBMap {
    pub const fn new() -> Self {
        GlobalDBMap { sqlite: None }
    }
}

pub trait BaseDatabase {
    fn load_bar_data(
        &self,
        symbol: &str,
        exchange: Exchange,
        interval: Interval,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Vec<BarData>;
}

pub fn get_database() -> Arc<dyn BaseDatabase> {
    // Read database related global setting
    let database_name = get_settings()["database.name"].clone();
    match database_name.as_str() {
        "sqlite" => {
            if DBMAP.lock().unwrap().sqlite.is_some() {
                return DBMAP.lock().unwrap().sqlite.as_ref().unwrap().clone();
            } else {
                DBMAP.lock().unwrap().sqlite = Some(Arc::new(
                    SqliteDatabase::connect(&get_settings()["database.database"]).unwrap(),
                ));
                return DBMAP.lock().unwrap().sqlite.as_ref().unwrap().clone();
            }
        }
        _ => {
            unreachable!("unsupported Database")
        }
    }
}

pub struct SqliteDatabase {
    pool: SqlitePool,
    rt: tokio::runtime::Runtime,
}

impl SqliteDatabase {
    pub fn connect(url: &str) -> Result<SqliteDatabase, Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let pool = rt.block_on(SqlitePool::connect("database.db"))?;
        Ok(SqliteDatabase { pool, rt })
    }
}

impl BaseDatabase for SqliteDatabase {
    fn load_bar_data(
        &self,
        symbol: &str,
        exchange: Exchange,
        interval: Interval,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Vec<BarData> {
        let interval_str = match interval {
            Interval::DAILY => "d",
            Interval::MINUTE => "1m",
            _ => {
                unreachable!("invaild interval!");
            }
        };

        let s = self.rt.block_on(
            sqlx::query("SELECT symbol,exchange,datetime,interval,volume,turnover,open_interest,open_price,high_price,low_price,close_price FROM dbbardata WHERE symbol=? and exchange=? and interval=? and datetime>=? and datetime<=? ORDER BY datetime")
                    .bind(symbol).bind(exchange.to_string()).bind(interval_str).bind(start).bind(end)
                    .fetch_all(&self.pool)).unwrap();
        let mut bars = Vec::new();
        for db_bar in s.iter() {
            bars.push(BarData {
                symbol: db_bar.get::<String, usize>(0),
                exchange: Exchange::from_str(&db_bar.get::<String, usize>(1)).unwrap(),
                datetime: db_bar.get::<NaiveDateTime, usize>(2),
                interval: match db_bar.get::<&str, usize>(3) {
                    "d" => Interval::DAILY,
                    "1m" => Interval::MINUTE,
                    _ => {
                        unreachable!("invalid interval")
                    }
                },
                volume: db_bar.get::<f64, usize>(4),
                turnover: db_bar.get::<f64, usize>(5),
                open_interest: db_bar.get::<f64, usize>(6),
                open_price: db_bar.get::<f64, usize>(7),
                high_price: db_bar.get::<f64, usize>(8),
                low_price: db_bar.get::<f64, usize>(9),
                close_price: db_bar.get::<f64, usize>(10),
                gateway_name: "DB",
            });
        }
        bars
    }
}
