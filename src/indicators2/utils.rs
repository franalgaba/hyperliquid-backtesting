use crate::data::types::Candle;

/// Ring buffer for efficient rolling window calculations
/// Optimized for cache locality and branchless operations
pub struct RingBuffer {
    buffer: Box<[f64]>, // Use Box<[T]> for better cache locality
    index: usize,
    size: usize,
    size_minus_1: usize, // Precomputed for branchless index update
    sum: f64,
}

impl RingBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size].into_boxed_slice(),
            index: 0,
            size,
            size_minus_1: size.saturating_sub(1),
            sum: 0.0,
        }
    }

    #[inline]
    pub fn push(&mut self, value: f64) -> f64 {
        // Use mem::replace for better optimization
        let old_value = std::mem::replace(&mut self.buffer[self.index], value);
        self.sum = self.sum - old_value + value;
        
        // Branchless index update: if index == size-1 then 0 else index+1
        // Using multiplication trick: (index != size_minus_1) as usize * (index + 1)
        self.index = ((self.index != self.size_minus_1) as usize) * (self.index + 1);
        
        old_value
    }

    pub fn mean(&self) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        self.sum / self.size as f64
    }

    pub fn get(&self, offset: usize) -> Option<f64> {
        if offset >= self.size {
            return None;
        }
        let idx = (self.index + self.size - 1 - offset) % self.size;
        Some(self.buffer[idx])
    }

    pub fn is_full(&self) -> bool {
        self.buffer[self.index] != 0.0 || self.index == 0
    }
}

/// Rolling standard deviation using Welford's algorithm
pub struct RollingStdDev {
    buffer: RingBuffer,
    mean: f64,
    m2: f64, // Sum of squares of differences from mean
    count: usize,
}

impl RollingStdDev {
    pub fn new(period: usize) -> Self {
        Self {
            buffer: RingBuffer::new(period),
            mean: 0.0,
            m2: 0.0,
            count: 0,
        }
    }

    pub fn update(&mut self, value: f64) {
        let old_value = self.buffer.push(value);
        
        if self.count < self.buffer.size {
            // Welford's algorithm for incremental variance
            self.count += 1;
            let delta = value - self.mean;
            self.mean += delta / self.count as f64;
            let delta2 = value - self.mean;
            self.m2 += delta * delta2;
        } else {
            // Update with removal of old value
            let delta_old = old_value - self.mean;
            let delta_new = value - self.mean;
            self.mean += (value - old_value) / self.buffer.size as f64;
            let delta_new2 = value - self.mean;
            self.m2 += delta_new * delta_new2 - delta_old * (old_value - self.mean);
        }
    }

    pub fn std_dev(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        (self.m2 / (self.count - 1) as f64).sqrt()
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }
}

/// Deque for tracking min/max in a sliding window
pub struct MinMaxDeque {
    values: Vec<f64>,
    max_deque: Vec<usize>, // Indices of max values
    min_deque: Vec<usize>, // Indices of min values
    window_size: usize,
}

impl MinMaxDeque {
    pub fn new(window_size: usize) -> Self {
        Self {
            values: Vec::with_capacity(window_size * 2),
            max_deque: Vec::new(),
            min_deque: Vec::new(),
            window_size,
        }
    }

    pub fn push(&mut self, value: f64) {
        let idx = self.values.len();
        self.values.push(value);

        // Remove indices outside window
        while let Some(&front_idx) = self.max_deque.first() {
            if idx - front_idx >= self.window_size {
                self.max_deque.remove(0);
            } else {
                break;
            }
        }
        while let Some(&front_idx) = self.min_deque.first() {
            if idx - front_idx >= self.window_size {
                self.min_deque.remove(0);
            } else {
                break;
            }
        }

        // Remove smaller values from max deque
        while let Some(&back_idx) = self.max_deque.last() {
            if self.values[back_idx] <= value {
                self.max_deque.pop();
            } else {
                break;
            }
        }
        self.max_deque.push(idx);

        // Remove larger values from min deque
        while let Some(&back_idx) = self.min_deque.last() {
            if self.values[back_idx] >= value {
                self.min_deque.pop();
            } else {
                break;
            }
        }
        self.min_deque.push(idx);
    }

    pub fn max(&self) -> Option<f64> {
        self.max_deque.first().map(|&idx| self.values[idx])
    }

    pub fn min(&self) -> Option<f64> {
        self.min_deque.first().map(|&idx| self.values[idx])
    }
}

/// Helper to extract price from candle based on source
/// Optimized: avoid string allocation and matching in hot path
#[inline]
pub fn get_price(candle: &Candle, source: &str) -> f64 {
    // Fast path for common case (close)
    if source.is_empty() || source == "close" {
        return candle.close;
    }
    
    // Use byte comparison for speed (most sources are ASCII)
    match source.as_bytes() {
        b"open" => candle.open,
        b"high" => candle.high,
        b"low" => candle.low,
        b"close" => candle.close,
        b"hl2" => (candle.high + candle.low) * 0.5,
        b"hlc3" => (candle.high + candle.low + candle.close) / 3.0,
        b"ohlc4" => (candle.open + candle.high + candle.low + candle.close) * 0.25,
        _ => {
            // Fallback to lowercase string matching for other cases
            match source.to_lowercase().as_str() {
                "open" => candle.open,
                "high" => candle.high,
                "low" => candle.low,
                "close" => candle.close,
                "hl2" => (candle.high + candle.low) * 0.5,
                "hlc3" => (candle.high + candle.low + candle.close) / 3.0,
                "ohlc4" => (candle.open + candle.high + candle.low + candle.close) * 0.25,
                _ => candle.close, // Default to close
            }
        }
    }
}

