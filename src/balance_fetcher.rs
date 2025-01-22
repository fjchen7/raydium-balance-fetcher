use std::str::FromStr;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

type Result<T> = anyhow::Result<T>;
pub struct BalanceFetcher {
    pub rpc: RpcClient,
}

pub struct SPLToken {
    pub amount: u64,
    pub decimals: u8,
}

// WSOL (Wrapped SOL) mint address
pub const WSOL_MINT_ADDRESS: &str = "So11111111111111111111111111111111111111112";

impl BalanceFetcher {
    pub fn new<T: ToString>(rpc_url: T) -> Self {
        let rpc = RpcClient::new(rpc_url.to_string());
        Self { rpc }
    }

    /// Fetch the SOL balance of a wallet
    ///
    /// # Arguments
    /// - `wallet_address` - The wallet address
    ///
    /// # Returns
    /// - `u64` - The SOL balance of the wallet
    pub fn balance_sol(&self, wallet_address: &Pubkey) -> Result<u64> {
        let balance = self.rpc.get_balance(&wallet_address)?;
        Ok(balance)
    }

    /// Fetch the WSOL (Wrapped SOL) balance of a wallet
    ///
    /// # Arguments
    /// - `wallet_address` - The wallet address
    ///
    /// # Returns
    /// - `u64` - The WSOL balance of the wallet
    pub fn balance_wsol(&self, wallet_address: &Pubkey) -> Result<u64> {
        let wsol_mint_address = Pubkey::from_str(WSOL_MINT_ADDRESS).unwrap();
        let balance = self.balance_spl_token(wallet_address, &wsol_mint_address)?;
        Ok(balance.amount)
    }

    /// Fetch the SOL and WSOL (Wrapped SOL) balance sum of a wallet
    ///
    /// # Arguments
    /// - `wallet_address` - The wallet address
    ///
    /// # Returns
    /// - `u64` - The SOL and WSOL balance of the wallet
    pub fn balance_sol_unified(&self, wallet_address: &Pubkey) -> Result<u64> {
        let sol_balance = self.balance_sol(wallet_address)?;
        let wsol_balance = self.balance_wsol(wallet_address)?;
        Ok(sol_balance + wsol_balance)
    }

    /// Fetch the balance of a SPL token account
    ///
    /// # Arguments
    /// - `wallet_address` - The wallet address
    /// - `token_mint_address` - The mint address of the token
    ///
    /// # Returns
    /// - `SPLToken` - The balance and decimals of the token account
    pub fn balance_spl_token(&self, wallet_address: &Pubkey, token_mint_address: &Pubkey) -> Result<SPLToken> {
        let addr = spl_associated_token_account::get_associated_token_address(&wallet_address, &token_mint_address);
        let ui_token_amount = self.rpc.get_token_account_balance(&addr)?;
        // Amount is the raw balance without decimals, a string representation of u64
        let amount = u64::from_str(&ui_token_amount.amount).unwrap();
        let decimals = ui_token_amount.decimals;
        let spl_token = SPLToken { amount, decimals };
        Ok(spl_token)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn new_balancer_fetcher() -> BalanceFetcher {
        let rpc_url = "https://api.mainnet-beta.solana.com";
        BalanceFetcher::new(rpc_url)
    }

    #[test]
    fn test_balance_sol() {
        let fetcher = new_balancer_fetcher();
        // Binance wallet address
        let pubkey = Pubkey::from_str("5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9").unwrap();
        let balance_sol = fetcher.balance_sol(&pubkey).unwrap();
        assert!(balance_sol > 0);
        let balance_sol_unified = fetcher.balance_sol_unified(&pubkey).unwrap();
        assert!(balance_sol_unified > balance_sol);
    }

    #[test]
    fn test_balance_spl_token() {
        let balancer_fetcher = new_balancer_fetcher();
        let wallet = Pubkey::from_str("5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9").unwrap();
        // WSOL (Wrapped SOL) mint address
        let token_mint_address = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let balance_spl_token = balancer_fetcher.balance_spl_token(&wallet, &token_mint_address).unwrap();
        assert!(balance_spl_token.amount > 0);
        assert_eq!(balance_spl_token.decimals, 9);
    }
}
