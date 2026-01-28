use anyhow::{Context, Result};
use csv::ReaderBuilder;
use serde::Deserialize;
use std::path::Path;

use crate::data::types::Candle;
use crate::strategy::types::Action as StrategyAction;
use crate::portfolio::Portfolio;
use crate::strategy::BacktestStrategy;

#[derive(Debug, Clone, Deserialize)]
struct SignalRow {
    pub time_open: u64,
    pub position: f64,
}

/// Strategy that follows precomputed target positions from a CSV file.
///
/// CSV format: time_open,position
pub struct SignalStrategy {
    signals: Vec<SignalRow>,
    idx: usize,
    current_target: f64,
}

impl SignalStrategy {
    pub fn from_csv(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)
            .with_context(|| format!("Failed to read signals CSV: {}", path.display()))?;

        let mut signals: Vec<SignalRow> = Vec::new();
        for result in reader.deserialize() {
            let row: SignalRow = result.context("Invalid signal row")?;
            signals.push(row);
        }
        signals.sort_by_key(|row| row.time_open);

        Ok(Self {
            signals,
            idx: 0,
            current_target: 0.0,
        })
    }

    fn update_target(&mut self, candle: &Candle) {
        while self.idx < self.signals.len() && self.signals[self.idx].time_open <= candle.time_open {
            self.current_target = self.signals[self.idx].position;
            self.idx += 1;
        }
    }
}

impl BacktestStrategy for SignalStrategy {
    fn warmup(&self) -> usize {
        0
    }

    fn on_warmup(&mut self, _candle: &Candle) -> Result<()> {
        Ok(())
    }

    fn on_candle(&mut self, candle: &Candle, portfolio: &Portfolio) -> Result<Option<StrategyAction>> {
        self.update_target(candle);

        let position_size = portfolio.get_position(&candle.coin);
        let is_flat = position_size.abs() < 1e-10;

        if self.current_target > 0.0 {
            if is_flat || position_size < 0.0 {
                return Ok(Some(StrategyAction::Buy { size_pct: 100.0 }));
            }
        } else if self.current_target < 0.0 {
            if is_flat || position_size > 0.0 {
                return Ok(Some(StrategyAction::Sell { size_pct: 100.0 }));
            }
        } else if !is_flat {
            return Ok(Some(StrategyAction::Close));
        }

        Ok(None)
    }
}
