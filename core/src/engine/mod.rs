// Engine module: in-memory order book matching logic (BTreeMap-based, sync, no I/O)

mod error;
mod order_book;
mod types;

pub use error::EngineError;
pub use order_book::OrderBook;
pub use types::{Order, Side, Trade};
