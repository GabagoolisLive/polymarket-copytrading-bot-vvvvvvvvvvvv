//! Polymarket CLOB client and order execution.

mod client;
mod orders;

pub use client::create_clob_client;
pub use orders::post_order;
