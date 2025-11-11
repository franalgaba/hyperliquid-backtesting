#[derive(Debug, Clone)]
pub struct FeeCalculator {
    maker_fee_bps: i16,
    taker_fee_bps: i16,
    slippage_bps: u16,
}

impl FeeCalculator {
    pub fn new(maker_fee_bps: i16, taker_fee_bps: i16, slippage_bps: u16) -> Self {
        Self {
            maker_fee_bps,
            taker_fee_bps,
            slippage_bps,
        }
    }

    pub fn calculate_fee(&self, notional: f64, is_maker: bool) -> f64 {
        let fee_bps = if is_maker {
            self.maker_fee_bps
        } else {
            self.taker_fee_bps
        };

        // Negative fee means rebate
        if fee_bps < 0 {
            -notional * (fee_bps.abs() as f64) / 10000.0
        } else {
            notional * (fee_bps as f64) / 10000.0
        }
    }

    pub fn apply_slippage(&self, price: f64, is_buy: bool) -> f64 {
        let slippage_factor = self.slippage_bps as f64 / 10000.0;
        if is_buy {
            price * (1.0 + slippage_factor)
        } else {
            price * (1.0 - slippage_factor)
        }
    }
}

