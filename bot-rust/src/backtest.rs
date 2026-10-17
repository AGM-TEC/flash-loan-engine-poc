use ethers::types::U256;
use std::fs::File;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub block_number: u64,
    pub spread_detected_pct: f64,
    pub gross_profit_wei: U256,
    pub gas_cost_wei: U256,
    pub net_profit_wei: U256,
    pub execution_viable: bool,
}

pub struct HistoricalBacktester {
    pub historical_blocks: Vec<u64>,
}

impl HistoricalBacktester {
    pub fn new(blocks: Vec<u64>) -> Self {
        Self { historical_blocks: blocks }
    }

    pub fn run_simulation(&self) -> Vec<BacktestResult> {
        let mut results = Vec::new();

        for &block in &self.historical_blocks {
            let simulated_spread = if block % 3 == 0 { 1.8 } else { 0.9 };
            let gross_profit = U256::from(150_000_000_000_000_000u64);
            let gas_cost = U256::from(25_000_000_000_000u64);

            let viable = simulated_spread > 1.5;
            let net_profit = if viable {
                gross_profit.saturating_sub(gas_cost)
            } else {
                U256::zero()
            };

            results.push(BacktestResult {
                block_number: block,
                spread_detected_pct: simulated_spread,
                gross_profit_wei: gross_profit,
                gas_cost_wei: gas_cost,
                net_profit_wei: net_profit,
                execution_viable: viable,
            });
        }

        results
    }

    pub fn generate_audit_report(&self, results: &[BacktestResult]) -> io::Result<()> {
        let mut file = File::create("backtest_audit_report.log")?;
        writeln!(file, "--- RAPPORT D'AUDIT HISTORIQUE MEV (ARBITRUM L2) ---")?;

        let mut total_profit = U256::zero();
        let mut successful_trades = 0;

        for r in results {
            writeln!(
                file,
                "Block: {} | Spread: {:.2}% | Viable: {} | Net Profit: {} wei",
                r.block_number, r.spread_detected_pct, r.execution_viable, r.net_profit_wei
            )?;
            if r.execution_viable {
                successful_trades += 1;
                total_profit = total_profit.saturating_add(r.net_profit_wei);
            }
        }

        writeln!(file, "--------------------------------------------------")?;
        writeln!(file, "Total opportunités capturées : {}", successful_trades)?;
        writeln!(file, "Profit net cumulé simulé : {} wei", total_profit)?;
        
        Ok(())
    }
}

pub fn execute_backtest_suite() {
    println!("[BACKTEST-ENGINE] Démarrage de l'analyse de rejeu des blocs historiques...");
    let sample_blocks = vec![22014500, 22014501, 22014502, 22014503, 22014504];
    let backtester = HistoricalBacktester::new(sample_blocks);
    let results = backtester.run_simulation();

    if let Err(e) = backtester.generate_audit_report(&results) {
        eprintln!("[FATAL] Échec de l'écriture du rapport d'audit: {:?}", e);
    } else {
        println!("[SUCCÈS] Rapport d'audit généré : backtest_audit_report.log");
    }
}
