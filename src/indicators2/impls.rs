use crate::data::types::Candle;
use crate::indicators2::registry::IndicatorEvaluator;
use crate::indicators2::utils::{get_price, MinMaxDeque, RingBuffer, RollingStdDev};
use anyhow::Result;

// SMA - Simple Moving Average
pub struct SmaIndicator {
    divider: f64, // 1.0 / length (precomputed)
    value: f64,   // Current SMA value
    buffer: RingBuffer,
    length: usize,
    source: String,
}

impl SmaIndicator {
    pub fn new(length: usize, source: String) -> Result<Self> {
        Ok(Self {
            divider: 1.0 / length as f64,
            value: 0.0, // Will be initialized with first price
            buffer: RingBuffer::new(length),
            length,
            source,
        })
    }
}

impl IndicatorEvaluator for SmaIndicator {
    fn warmup(&self) -> usize {
        self.length
    }

    #[inline]
    fn update(&mut self, candle: &Candle) -> Result<()> {
        let price = get_price(candle, &self.source);
        let prev_value = self.buffer.push(price);

        // Incremental update: value += (new - old) / length
        // This is O(1) instead of recalculating mean
        // Use fused multiply-add for better precision and performance
        self.value = (price - prev_value).mul_add(self.divider, self.value);
        Ok(())
    }

    #[inline]
    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "value" => Ok(self.value),
            _ => anyhow::bail!("Unknown SMA output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.buffer = RingBuffer::new(self.length);
        self.value = 0.0;
    }
}

// EMA - Exponential Moving Average
pub struct EmaIndicator {
    alpha: f64,
    value: f64,
    length: usize,
    source: String,
    initialized: bool,
}

impl EmaIndicator {
    pub fn new(length: usize, source: String) -> Result<Self> {
        let alpha = 2.0 / (length as f64 + 1.0);
        Ok(Self {
            alpha,
            value: 0.0,
            length,
            source,
            initialized: false,
        })
    }
}

impl IndicatorEvaluator for EmaIndicator {
    fn warmup(&self) -> usize {
        self.length * 3
    }

    #[inline]
    fn update(&mut self, candle: &Candle) -> Result<()> {
        let price = get_price(candle, &self.source);
        if !self.initialized {
            self.value = price;
            self.initialized = true;
        } else {
            // Use fused multiply-add for better performance: (price - value) * alpha + value
            self.value = (price - self.value).mul_add(self.alpha, self.value);
        }
        Ok(())
    }

    #[inline]
    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "value" => Ok(self.value),
            _ => anyhow::bail!("Unknown EMA output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
    }
}

// WMA - Weighted Moving Average (O(1) implementation)
pub struct WmaIndicator {
    invert_sum: f64,   // 1.0 / sum_of_weights
    float_length: f64, // length as f64
    total: f64,        // Sum of values in window (for rolling update)
    numerator: f64,    // Weighted sum (for rolling update)
    buffer: RingBuffer,
    length: usize,
    source: String,
}

impl WmaIndicator {
    pub fn new(length: usize, source: String) -> Result<Self> {
        let length2 = length;
        let sum = ((length2 * (length2 + 1)) / 2) as f64;
        let float_length = length as f64;
        // Initialize with first value (will be set during first update)
        Ok(Self {
            invert_sum: 1.0 / sum,
            float_length,
            total: 0.0,
            numerator: 0.0,
            buffer: RingBuffer::new(length),
            length,
            source,
        })
    }
}

impl IndicatorEvaluator for WmaIndicator {
    fn warmup(&self) -> usize {
        self.length
    }

    #[inline]
    fn update(&mut self, candle: &Candle) -> Result<()> {
        let price = get_price(candle, &self.source);
        let prev_value = self.buffer.push(price);

        // O(1) incremental update using rolling numerator and total
        // numerator += length * price + total
        // total += prev_value - price
        self.numerator += self.float_length.mul_add(price, self.total);
        self.total += prev_value - price;
        Ok(())
    }

    #[inline]
    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "value" => Ok(self.numerator * self.invert_sum),
            _ => anyhow::bail!("Unknown WMA output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.buffer = RingBuffer::new(self.length);
        self.total = 0.0;
        self.numerator = 0.0;
    }
}

// RSI - Relative Strength Index (Wilder's smoothing)
pub struct RsiIndicator {
    period: usize,
    source: String,
    avg_gain: f64,
    avg_loss: f64,
    prev_price: Option<f64>,
    rsi: f64,
    initialized: bool,
}

impl RsiIndicator {
    pub fn new(period: usize, source: String) -> Result<Self> {
        Ok(Self {
            period,
            source,
            avg_gain: 0.0,
            avg_loss: 0.0,
            prev_price: None,
            rsi: 50.0,
            initialized: false,
        })
    }
}

impl IndicatorEvaluator for RsiIndicator {
    fn warmup(&self) -> usize {
        self.period + 1
    }

    fn update(&mut self, candle: &Candle) -> Result<()> {
        let price = get_price(candle, &self.source);

        if let Some(prev) = self.prev_price {
            let change = price - prev;
            let gain = change.max(0.0);
            let loss = (-change).max(0.0);

            if !self.initialized {
                // Initial average
                self.avg_gain = gain;
                self.avg_loss = loss;
                self.initialized = true;
            } else {
                // Wilder's smoothing: new_avg = (old_avg * (n-1) + new_value) / n
                self.avg_gain =
                    (self.avg_gain * (self.period - 1) as f64 + gain) / self.period as f64;
                self.avg_loss =
                    (self.avg_loss * (self.period - 1) as f64 + loss) / self.period as f64;
            }

            if self.avg_loss != 0.0 {
                let rs = self.avg_gain / self.avg_loss;
                self.rsi = 100.0 - (100.0 / (1.0 + rs));
            } else {
                self.rsi = 100.0;
            }
        }

        self.prev_price = Some(price);
        Ok(())
    }

    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "value" => Ok(self.rsi),
            _ => anyhow::bail!("Unknown RSI output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.prev_price = None;
        self.rsi = 50.0;
        self.initialized = false;
    }
}

// MACD - Moving Average Convergence Divergence
pub struct MacdIndicator {
    fast_ema: EmaIndicator,
    slow_ema: EmaIndicator,
    signal_ema: EmaIndicator,
    macd: f64,
    signal: f64,
    histogram: f64,
}

impl MacdIndicator {
    pub fn new(fast: usize, slow: usize, signal: usize, source: String) -> Result<Self> {
        Ok(Self {
            fast_ema: EmaIndicator::new(fast, source.clone())?,
            slow_ema: EmaIndicator::new(slow, source.clone())?,
            signal_ema: EmaIndicator::new(signal, source)?,
            macd: 0.0,
            signal: 0.0,
            histogram: 0.0,
        })
    }
}

impl IndicatorEvaluator for MacdIndicator {
    fn warmup(&self) -> usize {
        self.slow_ema.warmup() + self.signal_ema.warmup()
    }

    fn update(&mut self, candle: &Candle) -> Result<()> {
        self.fast_ema.update(candle)?;
        self.slow_ema.update(candle)?;

        let fast_value = self.fast_ema.value("value")?;
        let slow_value = self.slow_ema.value("value")?;
        self.macd = fast_value - slow_value;

        // Update signal EMA with MACD value
        // We need to update signal EMA with the MACD value, so we create a temporary candle
        let macd_candle = Candle {
            time_open: candle.time_open,
            time_close: candle.time_close,
            coin: candle.coin.clone(),
            interval: candle.interval.clone(),
            open: self.macd,
            close: self.macd,
            high: self.macd,
            low: self.macd,
            volume: candle.volume,
            num_trades: candle.num_trades,
        };
        self.signal_ema.update(&macd_candle)?;
        self.signal = self.signal_ema.value("value")?;
        self.histogram = self.macd - self.signal;

        Ok(())
    }

    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "macd" => Ok(self.macd),
            "signal" => Ok(self.signal),
            "histogram" => Ok(self.histogram),
            _ => anyhow::bail!("Unknown MACD output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.signal_ema.reset();
        self.macd = 0.0;
        self.signal = 0.0;
        self.histogram = 0.0;
    }
}

// Bollinger Bands
pub struct BBandsIndicator {
    sma: SmaIndicator,
    std_dev: RollingStdDev,
    std_multiplier: f64,
    length: usize,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl BBandsIndicator {
    pub fn new(length: usize, std_multiplier: f64, source: String) -> Result<Self> {
        Ok(Self {
            sma: SmaIndicator::new(length, source.clone())?,
            std_dev: RollingStdDev::new(length),
            std_multiplier,
            length,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        })
    }
}

impl IndicatorEvaluator for BBandsIndicator {
    fn warmup(&self) -> usize {
        self.sma.warmup()
    }

    fn update(&mut self, candle: &Candle) -> Result<()> {
        self.sma.update(candle)?;
        let price = get_price(candle, "close"); // Use close for std dev
        self.std_dev.update(price);

        self.middle = self.sma.value("value")?;
        let sd = self.std_dev.std_dev();
        self.upper = self.middle + self.std_multiplier * sd;
        self.lower = self.middle - self.std_multiplier * sd;

        Ok(())
    }

    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "upper" => Ok(self.upper),
            "middle" => Ok(self.middle),
            "lower" => Ok(self.lower),
            _ => anyhow::bail!("Unknown BBands output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.sma.reset();
        // Recreate std_dev with same period
        self.std_dev = RollingStdDev::new(self.length);
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
}

// Stochastic Oscillator (%K and %D)
pub struct StochIndicator {
    k_period: usize,
    k_smooth: usize,
    d_period: usize,
    high_deque: MinMaxDeque,
    low_deque: MinMaxDeque,
    k_values: Vec<f64>,
    k: f64,
    d: f64,
}

impl StochIndicator {
    pub fn new(k_period: usize, k_smooth: usize, d_period: usize) -> Result<Self> {
        Ok(Self {
            k_period,
            k_smooth,
            d_period,
            high_deque: MinMaxDeque::new(k_period),
            low_deque: MinMaxDeque::new(k_period),
            k_values: Vec::new(),
            k: 0.0,
            d: 0.0,
        })
    }
}

impl IndicatorEvaluator for StochIndicator {
    fn warmup(&self) -> usize {
        self.k_period + self.d_period
    }

    fn update(&mut self, candle: &Candle) -> Result<()> {
        self.high_deque.push(candle.high);
        self.low_deque.push(candle.low);

        if let (Some(high), Some(low)) = (self.high_deque.max(), self.low_deque.min()) {
            if high != low {
                let raw_k = 100.0 * (candle.close - low) / (high - low);

                // Smooth %K if needed
                if self.k_smooth > 1 {
                    self.k_values.push(raw_k);
                    if self.k_values.len() > self.k_smooth {
                        self.k_values.remove(0);
                    }
                    if self.k_values.len() == self.k_smooth {
                        self.k = self.k_values.iter().sum::<f64>() / self.k_smooth as f64;
                    }
                } else {
                    self.k = raw_k;
                }

                // Calculate %D (SMA of %K)
                if self.k_values.len() >= self.d_period {
                    let start = self.k_values.len().saturating_sub(self.d_period);
                    let sum: f64 = self.k_values[start..].iter().sum();
                    self.d = sum / self.d_period as f64;
                }
            }
        }
        Ok(())
    }

    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "k" | "value" => Ok(self.k),
            "d" => Ok(self.d),
            _ => anyhow::bail!("Unknown Stochastic output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.high_deque = MinMaxDeque::new(self.k_period);
        self.low_deque = MinMaxDeque::new(self.k_period);
        self.k_values.clear();
        self.k = 0.0;
        self.d = 0.0;
    }
}

// ATR - Average True Range
pub struct AtrIndicator {
    period: usize,
    tr_values: Vec<f64>,
    atr: f64,
    prev_close: Option<f64>,
    initialized: bool,
}

impl AtrIndicator {
    pub fn new(period: usize) -> Result<Self> {
        Ok(Self {
            period,
            tr_values: Vec::new(),
            atr: 0.0,
            prev_close: None,
            initialized: false,
        })
    }
}

impl IndicatorEvaluator for AtrIndicator {
    fn warmup(&self) -> usize {
        self.period + 1
    }

    fn update(&mut self, candle: &Candle) -> Result<()> {
        let tr = if let Some(prev_close) = self.prev_close {
            let hl = candle.high - candle.low;
            let hc = (candle.high - prev_close).abs();
            let lc = (candle.low - prev_close).abs();
            hl.max(hc).max(lc)
        } else {
            candle.high - candle.low
        };

        self.tr_values.push(tr);
        if self.tr_values.len() > self.period {
            self.tr_values.remove(0);
        }

        if !self.initialized && self.tr_values.len() == self.period {
            // Initial ATR: simple average
            self.atr = self.tr_values.iter().sum::<f64>() / self.period as f64;
            self.initialized = true;
        } else if self.initialized {
            // Wilder's smoothing
            self.atr = (self.atr * (self.period - 1) as f64 + tr) / self.period as f64;
        }

        self.prev_close = Some(candle.close);
        Ok(())
    }

    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "value" | "atr" => Ok(self.atr),
            _ => anyhow::bail!("Unknown ATR output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.tr_values.clear();
        self.atr = 0.0;
        self.prev_close = None;
        self.initialized = false;
    }
}

// ADX - Average Directional Index
pub struct AdxIndicator {
    period: usize,
    atr: AtrIndicator,
    plus_dm_values: Vec<f64>,
    minus_dm_values: Vec<f64>,
    plus_di: f64,
    minus_di: f64,
    dx: f64,
    adx: f64,
    prev_high: Option<f64>,
    prev_low: Option<f64>,
    initialized: bool,
}

impl AdxIndicator {
    pub fn new(period: usize) -> Result<Self> {
        Ok(Self {
            period,
            atr: AtrIndicator::new(period)?,
            plus_dm_values: Vec::new(),
            minus_dm_values: Vec::new(),
            plus_di: 0.0,
            minus_di: 0.0,
            dx: 0.0,
            adx: 0.0,
            prev_high: None,
            prev_low: None,
            initialized: false,
        })
    }
}

impl IndicatorEvaluator for AdxIndicator {
    fn warmup(&self) -> usize {
        self.period * 2
    }

    fn update(&mut self, candle: &Candle) -> Result<()> {
        self.atr.update(candle)?;

        if let (Some(prev_high), Some(prev_low)) = (self.prev_high, self.prev_low) {
            let up_move = candle.high - prev_high;
            let down_move = prev_low - candle.low;

            let plus_dm = if up_move > down_move && up_move > 0.0 {
                up_move
            } else {
                0.0
            };
            let minus_dm = if down_move > up_move && down_move > 0.0 {
                down_move
            } else {
                0.0
            };

            self.plus_dm_values.push(plus_dm);
            self.minus_dm_values.push(minus_dm);

            if self.plus_dm_values.len() > self.period {
                self.plus_dm_values.remove(0);
                self.minus_dm_values.remove(0);
            }

            if !self.initialized && self.plus_dm_values.len() == self.period {
                // Initial DI values
                let atr_value = self.atr.value("value")?;
                if atr_value != 0.0 {
                    let plus_dm_sum: f64 = self.plus_dm_values.iter().sum();
                    let minus_dm_sum: f64 = self.minus_dm_values.iter().sum();
                    self.plus_di = 100.0 * plus_dm_sum / (self.period as f64 * atr_value);
                    self.minus_di = 100.0 * minus_dm_sum / (self.period as f64 * atr_value);
                }
                self.initialized = true;
            } else if self.initialized {
                // Wilder's smoothing for DI
                let atr_value = self.atr.value("value")?;
                if atr_value != 0.0 {
                    let plus_dm_avg =
                        (self.plus_di * (self.period - 1) as f64 + plus_dm) / self.period as f64;
                    let minus_dm_avg =
                        (self.minus_di * (self.period - 1) as f64 + minus_dm) / self.period as f64;
                    self.plus_di = 100.0 * plus_dm_avg / atr_value;
                    self.minus_di = 100.0 * minus_dm_avg / atr_value;
                }

                // Calculate DX and ADX
                let di_sum = self.plus_di + self.minus_di;
                if di_sum != 0.0 {
                    self.dx = 100.0 * (self.plus_di - self.minus_di).abs() / di_sum;
                    // ADX is smoothed DX
                    if self.adx == 0.0 {
                        self.adx = self.dx;
                    } else {
                        self.adx =
                            (self.adx * (self.period - 1) as f64 + self.dx) / self.period as f64;
                    }
                }
            }
        }

        self.prev_high = Some(candle.high);
        self.prev_low = Some(candle.low);
        Ok(())
    }

    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "adx" | "value" => Ok(self.adx),
            "plus_di" | "+di" => Ok(self.plus_di),
            "minus_di" | "-di" => Ok(self.minus_di),
            "dx" => Ok(self.dx),
            _ => anyhow::bail!("Unknown ADX output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.atr.reset();
        self.plus_dm_values.clear();
        self.minus_dm_values.clear();
        self.plus_di = 0.0;
        self.minus_di = 0.0;
        self.dx = 0.0;
        self.adx = 0.0;
        self.prev_high = None;
        self.prev_low = None;
        self.initialized = false;
    }
}

// OBV - On-Balance Volume
pub struct ObvIndicator {
    obv: f64,
    prev_close: Option<f64>,
}

impl ObvIndicator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            obv: 0.0,
            prev_close: None,
        })
    }
}

impl IndicatorEvaluator for ObvIndicator {
    fn warmup(&self) -> usize {
        1
    }

    fn update(&mut self, candle: &Candle) -> Result<()> {
        if let Some(prev_close) = self.prev_close {
            if candle.close > prev_close {
                self.obv += candle.volume;
            } else if candle.close < prev_close {
                self.obv -= candle.volume;
            }
            // If close == prev_close, OBV stays the same
        } else {
            // Initialize with first volume
            self.obv = candle.volume;
        }
        self.prev_close = Some(candle.close);
        Ok(())
    }

    fn value(&self, output: &str) -> Result<f64> {
        match output {
            "value" | "obv" => Ok(self.obv),
            _ => anyhow::bail!("Unknown OBV output: {}", output),
        }
    }

    fn reset(&mut self) {
        self.obv = 0.0;
        self.prev_close = None;
    }
}
