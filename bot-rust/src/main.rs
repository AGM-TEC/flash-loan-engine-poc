use dotenv::dotenv;
use ethers::prelude::*;
use log::{info, error};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio::sync::RwLock;

abigen!(
    ArbitrageContract,
    r#"[
        function executeArbitrage(address tokenA, address tokenB, uint256 flashAmount) external
    ]"#,
);

#[derive(Clone, Default)]
struct PoolState {
    reserve0: U256,
    reserve1: U256,
    price: U256,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Moteur HFT Phase 5.1 (COMBAT READY) - Découplage Observation/Exécution...");

    dotenv().ok();
    let wss_url = env::var("ARBITRUM_WSS").expect("Rupture: ARBITRUM_WSS");
    let private_key = env::var("HOT_WALLET_PRIVATE_KEY").expect("Rupture: Clé");
    let contract_address = env::var("CONTRACT_ADDRESS").expect("Rupture: Contrat").parse::<Address>().unwrap();
    
    // Couche d'Observation (Filtre DEX)
    let target_pool_a = env::var("TARGET_POOL_A").expect("Rupture: Cible LP A").parse::<Address>().unwrap();
    let target_pool_b = env::var("TARGET_POOL_B").expect("Rupture: Cible LP B").parse::<Address>().unwrap();
    
    // Couche d'Exécution (Base Tokens L2)
    let token_a = env::var("TOKEN_A").expect("Rupture: TOKEN_A").parse::<Address>().unwrap();
    let token_b = env::var("TOKEN_B").expect("Rupture: TOKEN_B").parse::<Address>().unwrap();

    let ws_provider = match timeout(Duration::from_secs(10), Provider::<Ws>::connect(&wss_url)).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => { error!("Rupture I/O: Handshake échoué : {:?}", e); std::process::exit(1); },
        Err(_) => { error!("Timeout I/O dRPC."); std::process::exit(1); }
    };

    let chain_id = ws_provider.get_chainid().await?.as_u64();
    let wallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let client = Arc::new(SignerMiddleware::new(ws_provider, wallet));
    let contract = ArbitrageContract::new(contract_address, client.clone());

    let sync_signature = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1".parse::<H256>().unwrap();
    let filter = Filter::new().topic0(sync_signature);

    let state_a = Arc::new(RwLock::new(PoolState::default()));
    let state_b = Arc::new(RwLock::new(PoolState::default()));

    info!("Veille Active | Écoute: A({:?}) B({:?}) | Trading: WETH/USDT", target_pool_a, target_pool_b);

    let mut stream = client.subscribe_logs(&filter).await?;

    while let Some(log) = stream.next().await {
        if log.data.len() >= 64 {
            let pool_addr = log.address;
            
            if pool_addr != target_pool_a && pool_addr != target_pool_b { continue; }

            let r0 = U256::from_big_endian(&log.data[0..32]);
            let r1 = U256::from_big_endian(&log.data[32..64]);

            let state_a_ref = state_a.clone();
            let state_b_ref = state_b.clone();
            let contract_ref = contract.clone();
            
            // Clonage pour le thread
            let exec_token_a = token_a;
            let exec_token_b = token_b;

            tokio::spawn(async move {
                if r0.is_zero() || r1.is_zero() { return; }
                let precision = U256::exp10(18);
                let current_price = (r1 * precision) / r0;

                let mut s_a = state_a_ref.write().await;
                let mut s_b = state_b_ref.write().await;

                if pool_addr == target_pool_a {
                    s_a.reserve0 = r0; s_a.reserve1 = r1; s_a.price = current_price;
                } else {
                    s_b.reserve0 = r0; s_b.reserve1 = r1; s_b.price = current_price;
                }

                if !s_a.price.is_zero() && !s_b.price.is_zero() {
                    let spread = if s_a.price > s_b.price { s_a.price - s_b.price } else { s_b.price - s_a.price };
                    
                    let min_profitable_spread = U256::from(1000000000000000u64); 

                    if spread > min_profitable_spread {
                        let min_reserve_0 = if s_a.reserve0 < s_b.reserve0 { s_a.reserve0 } else { s_b.reserve0 };
                        let optimal_flash_amount = (min_reserve_0 * 2) / 100;
                        
                        let min_borrow_limit = U256::exp10(15); 
                        if optimal_flash_amount < min_borrow_limit { return; }

                        info!("🔥 SPREAD L2: {} | Emprunt: {} | Tir WETH/USDT en cours...", spread, optimal_flash_amount);
                        
                        // Exécution stricte sur les actifs de base, non sur les LP
                        match contract_ref.execute_arbitrage(exec_token_a, exec_token_b, optimal_flash_amount).send().await {
                            Ok(tx) => info!("✅ Tx On-Chain L2 propulsée : {:?}", tx.tx_hash()),
                            Err(e) => error!("❌ Disjoncteur L2 (Revert Gaz/Rentabilité) : {:?}", e),
                        }
                    }
                }
            });
        }
    }
    error!("Rupture d'I/O.");
    std::process::exit(1);
}
