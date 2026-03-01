//! Configuration: environment loading and copy strategy.

mod env;
mod strategy;

pub use env::{is_valid_ethereum_address, parse_user_addresses, EnvConfig};
pub use strategy::{
    calculate_order_size, get_trade_multiplier, parse_tiered_multipliers, CopyStrategy,
    CopyStrategyConfig, MultiplierTier, OrderSizeCalculation,
};
