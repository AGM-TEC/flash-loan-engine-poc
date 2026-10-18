use revm::primitives::{Address, U256};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SequencerPreConf {
    pub tx_hash: [u8; 32],
    pub target_pool: Address,
    pub delta_reserve_0: U256,
    pub delta_reserve_1: U256,
}

pub struct PreStateInjector {
    simulated_pools: HashMap<Address, (U256, U256)>,
}

impl PreStateInjector {
    pub fn new() -> Self {
        Self {
            simulated_pools: HashMap::new(),
        }
    }

    pub fn register_pool(&mut self, pool: Address, reserve_0: U256, reserve_1: U256) {
        self.simulated_pools.insert(pool, (reserve_0, reserve_1));
    }

    pub fn inject_pre_confirmation(&mut self, pre_conf: &SequencerPreConf) -> Option<(U256, U256)> {
        if let Some((r0, r1)) = self.simulated_pools.get_mut(&pre_conf.target_pool) {
            *r0 = r0.saturating_add(pre_conf.delta_reserve_0);
            *r1 = r1.saturating_sub(pre_conf.delta_reserve_1);
            Some((*r0, *r1))
        } else {
            None
        }
    }

    pub fn evaluate_arbitrage_opportunity(&self, pool: Address, min_spread_bps: u64) -> bool {
        if let Some((r0, r1)) = self.simulated_pools.get(&pool) {
            if *r1 == U256::ZERO {
                return false;
            }
            let price_ratio = (*r0 * U256::from(10_000)) / *r1;
            price_ratio > U256::from(min_spread_bps)
        } else {
            false
        }
    }
}

#[tokio::test]
async fn test_pre_state_injection() {
    let mut injector = PreStateInjector::new();
    let pool_addr = Address::repeat_byte(0xAA);
    
    injector.register_pool(
        pool_addr, 
        U256::from(1_000_000_000_000_000_000u128), 
        U256::from(1_000_000_000_000_000_000u128)
    );

    let pre_conf = SequencerPreConf {
        tx_hash: [0x01; 32],
        target_pool: pool_addr,
        delta_reserve_0: U256::from(500_000_000_000_000_000u128),
        delta_reserve_1: U256::from(200_000_000_000_000_000u128),
    };

    let updated_state = injector.inject_pre_confirmation(&pre_conf);
    assert!(updated_state.is_some());
}
