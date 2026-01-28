use anyhow::Result;

use crate::data::types::Candle;
use crate::strategy::types::Action as StrategyAction;
use crate::portfolio::Portfolio;

/// Common interface for backtesting custom strategies.
pub trait BacktestStrategy: Send {
    /// Number of candles needed to warm up internal state/indicators.
    fn warmup(&self) -> usize;

    /// Update internal state during warmup (no trading allowed).
    fn on_warmup(&mut self, candle: &Candle) -> Result<()>;

    /// Evaluate strategy on a new candle and optionally return an action.
    fn on_candle(
        &mut self,
        candle: &Candle,
        portfolio: &Portfolio,
    ) -> Result<Option<StrategyAction>>;
}
