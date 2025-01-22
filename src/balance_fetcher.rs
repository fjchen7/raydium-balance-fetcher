use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

type Result<T> = anyhow::Result<T>;
pub struct BalanceFetcher {
    pub rpc: RpcClient,
}

impl BalanceFetcher {
    pub fn new<T: ToString>(rpc_url: T) -> Self {
        let rpc = RpcClient::new(rpc_url.to_string());
        Self { rpc }
    }

    pub fn balance_sol(&self, pubkey: &Pubkey) -> Result<u64> {
        let balance = self.rpc.get_balance(&pubkey)?;
        Ok(balance)
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
        let balancer_fetcher = new_balancer_fetcher();
        let pubkey = Pubkey::from_str("5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9").unwrap();
        let balance_sol = balancer_fetcher.balance_sol(&pubkey).unwrap();
        assert!(balance_sol > 0);
    }
}
