use std::env;
use std::str::FromStr;
use balance_fetcher::BalanceFetcher;
use solana_sdk::pubkey::Pubkey;

mod balance_fetcher;

type Result<T> = anyhow::Result<T>;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Please Usage: {} <address>", args[0]);
        eprintln!("Example: {} 53zSj4G935ZY2a5x2UnGAiJXSuXXmGHaLph2zhAUvYpg", args[0]);
        std::process::exit(1);
    }

    let rpc_url = "https://api.mainnet-beta.solana.com";
    let balance_fetcher = BalanceFetcher::new(rpc_url);

    let addr = Pubkey::from_str(args[1].as_str())
        .unwrap_or_else(|_| {
            eprintln!("Invalid address. Good address example: 53zSj4G935ZY2a5x2UnGAiJXSuXXmGHaLph2zhAUvYpg");
            std::process::exit(1);
        });

    let balance_sol = balance_fetcher.balance_sol(&addr)?;
    let balance_wsol = balance_fetcher.balance_wsol(&addr)?;
    let balance_sol_unified = balance_fetcher.balance_sol_unified(&addr)?;
    let balance_sol_position = balance_fetcher.position_sol_usdc_1bp(&addr)?.0;

    let sol_decimals = 9;
    let sol_multiplier = 10u64.pow(sol_decimals);
    let (balance_sol, balance_wsol, balance_sol_unified, balance_sol_position) = (
        balance_sol as f64 / sol_multiplier as f64,
        balance_wsol as f64 / sol_multiplier as f64,
        balance_sol_unified as f64 / sol_multiplier as f64,
        balance_sol_position as f64 / sol_multiplier as f64,
    );

    println!("
SOL Balance/Position Summary for address: {}
- SOL: {}
- WSOL: {}
- SOL Unified (SOL + WSOL): {}
- SOL in SOL-USDC.1bp LP Position: {}
    ", addr, balance_sol, balance_wsol, balance_sol_unified, balance_sol_position);
    Ok(())
}
