use std::collections::HashMap;
use anyhow::Result;

use crate::data::types::Candle;
use crate::indicators2::impls::*;

pub trait IndicatorEvaluator: Send + Sync {
    fn warmup(&self) -> usize;
    fn update(&mut self, candle: &Candle) -> Result<()>;
    fn value(&self, output: &str) -> Result<f64>;
    fn reset(&mut self);
}

pub struct IndicatorRegistry;

impl IndicatorRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn get_lookback(&self, indicator_type: &str, params: &HashMap<String, f64>) -> Result<usize> {
        match indicator_type.to_uppercase().as_str() {
            "SMA" => {
                let length = params.get("length").copied().unwrap_or(20.0) as usize;
                Ok(length)
            }
            "EMA" => {
                let length = params.get("length").copied().unwrap_or(20.0) as usize;
                Ok(length * 3) // Safe warmup
            }
            "WMA" => {
                let length = params.get("length").copied().unwrap_or(20.0) as usize;
                Ok(length)
            }
            "RSI" => {
                let length = params.get("length").copied().unwrap_or(14.0) as usize;
                Ok(length + 1)
            }
            "MACD" => {
                let slow = params.get("slow").copied().unwrap_or(26.0) as usize;
                let signal = params.get("signal").copied().unwrap_or(9.0) as usize;
                Ok(slow + signal * 3)
            }
            "BBANDS" => {
                let length = params.get("length").copied().unwrap_or(20.0) as usize;
                Ok(length)
            }
            "STOCH" | "STOCHASTIC" => {
                let k_period = params.get("k_period").or(params.get("period")).copied().unwrap_or(14.0) as usize;
                let d_period = params.get("d_period").or(params.get("smooth")).copied().unwrap_or(3.0) as usize;
                Ok(k_period + d_period)
            }
            "ATR" => {
                let period = params.get("period").or(params.get("length")).copied().unwrap_or(14.0) as usize;
                Ok(period + 1)
            }
            "ADX" => {
                let period = params.get("period").or(params.get("length")).copied().unwrap_or(14.0) as usize;
                Ok(period * 2) // ADX needs more warmup
            }
            "OBV" => {
                Ok(1) // OBV starts immediately
            }
            _ => anyhow::bail!("Unknown indicator type: {}", indicator_type),
        }
    }
}

pub fn create_indicator(
    indicator_type: &str,
    params: &HashMap<String, f64>,
) -> Result<Box<dyn IndicatorEvaluator>> {
    match indicator_type.to_uppercase().as_str() {
        "SMA" => {
            let length = params.get("length").copied().unwrap_or(20.0) as usize;
            let source = params.get("source")
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or_else(|| "close".to_string());
            Ok(Box::new(SmaIndicator::new(length, source)?))
        }
        "EMA" => {
            let length = params.get("length").copied().unwrap_or(20.0) as usize;
            let source = params.get("source")
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or_else(|| "close".to_string());
            Ok(Box::new(EmaIndicator::new(length, source)?))
        }
        "WMA" => {
            let length = params.get("length").copied().unwrap_or(20.0) as usize;
            let source = params.get("source")
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or_else(|| "close".to_string());
            Ok(Box::new(WmaIndicator::new(length, source)?))
        }
        "RSI" => {
            let length = params.get("length").copied().unwrap_or(14.0) as usize;
            let source = params.get("source")
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or_else(|| "close".to_string());
            Ok(Box::new(RsiIndicator::new(length, source)?))
        }
        "MACD" => {
            let fast = params.get("fast").copied().unwrap_or(12.0) as usize;
            let slow = params.get("slow").copied().unwrap_or(26.0) as usize;
            let signal = params.get("signal").copied().unwrap_or(9.0) as usize;
            let source = params.get("source")
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or_else(|| "close".to_string());
            Ok(Box::new(MacdIndicator::new(fast, slow, signal, source)?))
        }
        "BBANDS" | "BB" => {
            let length = params.get("length").copied().unwrap_or(20.0) as usize;
            let std_dev = params.get("std").or(params.get("std_dev")).copied().unwrap_or(2.0);
            let source = params.get("source")
                .and_then(|v| v.to_string().parse().ok())
                .unwrap_or_else(|| "close".to_string());
            Ok(Box::new(BBandsIndicator::new(length, std_dev, source)?))
        }
        "STOCH" | "STOCHASTIC" => {
            let k_period = params.get("k_period").or(params.get("period")).copied().unwrap_or(14.0) as usize;
            let k_smooth = params.get("k_smooth").copied().unwrap_or(1.0) as usize;
            let d_period = params.get("d_period").or(params.get("smooth")).copied().unwrap_or(3.0) as usize;
            Ok(Box::new(StochIndicator::new(k_period, k_smooth, d_period)?))
        }
        "ATR" => {
            let period = params.get("period").or(params.get("length")).copied().unwrap_or(14.0) as usize;
            Ok(Box::new(AtrIndicator::new(period)?))
        }
        "ADX" => {
            let period = params.get("period").or(params.get("length")).copied().unwrap_or(14.0) as usize;
            Ok(Box::new(AdxIndicator::new(period)?))
        }
        "OBV" => {
            Ok(Box::new(ObvIndicator::new()?))
        }
        _ => anyhow::bail!("Unknown indicator type: {}", indicator_type),
    }
}

