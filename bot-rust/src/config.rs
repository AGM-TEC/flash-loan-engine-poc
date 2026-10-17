use ethers::types::U256;

pub struct EngineConfig {
    pub target_weth: &'static str,
    pub target_token: &'static str,
    pub min_spread_threshold_pct: f64,
    pub max_idle_cycles: u32,
    pub estimated_gas_limit: u64,
    pub min_net_profit_wei: U256,
}

impl EngineConfig {
    pub fn load_production_profile() -> Self {
        Self {
            target_weth: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
            target_token: "0x25118290e6A5f4139381D072181157035864099d",
            min_spread_threshold_pct: 1.5,
            max_idle_cycles: 720,
            estimated_gas_limit: 185_000,
            min_net_profit_wei: U256::from(10_000_000_000_000u64),
        }
    }

    pub fn validate_spread(&self, spread_pct: f64) -> bool {
        spread_pct >= self.min_spread_threshold_pct
    }
}
