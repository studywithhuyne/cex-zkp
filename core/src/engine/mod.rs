// Engine module: in-memory order book matching logic (BTreeMap-based, sync, no I/O)

mod order_book;
mod types;

pub use order_book::OrderBook;
pub use types::{Order, Side, Trade};
