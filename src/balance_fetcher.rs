use std::str::FromStr;
use anchor_lang::AccountDeserialize;
use raydium_amm_v3::libraries::{get_delta_amount_0_unsigned, get_delta_amount_1_unsigned, tick_math};
use solana_account_decoder::parse_token::{TokenAccountType, UiAccountState};
use solana_account_decoder::UiAccountData;
use solana_client::rpc_client::RpcClient;
use solana_rpc_client_api::client_error::ErrorKind;
use solana_rpc_client_api::request::{RpcError, TokenAccountsFilter};
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;

type Result<T> = anyhow::Result<T>;
pub struct BalanceFetcher {
    pub rpc: RpcClient,
}

#[allow(dead_code)]
pub struct SPLToken {
    amount: u64,
    pub decimals: u8,
}

// Program ID for Solana mainnet.
pub const WSOL_MINT_ADDRESS: &str = "So11111111111111111111111111111111111111112";
pub const RAYDIUM_V3_PROGRAM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const SOL_USDC_1BP_POOL: &str = "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj";

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
        let ui_token_amount =
            match self.rpc.get_token_account_balance(&addr) {
                Ok(ui_token_amount) => ui_token_amount,
                Err(err) => {
                    match err.kind {
                        ErrorKind::RpcError(RpcError::RpcResponseError { .. }) => {
                            // If the token account does not exist, RPC return error.
                            // This is a temporary solution.
                            log::warn!("address {} does not have token account for SPL token {}", addr, token_mint_address);
                            return Ok(SPLToken { amount: 0, decimals: 0 });
                        }
                        _ => {
                            return Err(err.into());
                        }
                    }
                }
            };
        // Amount is the raw balance without decimals, a string representation of u64
        let amount = u64::from_str(&ui_token_amount.amount).unwrap();
        let decimals = ui_token_amount.decimals;
        let spl_token = SPLToken { amount, decimals };
        Ok(spl_token)
    }

    /// Fetch the LP position amounts of Raydium SOL-USDC.1bp pool
    ///
    /// # Arguments
    /// - `wallet_address` - The wallet address
    ///
    /// # Returns
    /// - `(u64, u64)` - The total amount of SOL and USDC in wallet_address's LP position in the pool
    pub fn position_sol_usdc_1bp(&self, wallet_address: &Pubkey) -> Result<(u64, u64)> {
        let pool_id = Pubkey::from_str(SOL_USDC_1BP_POOL)?;
        self.raydium_pool_position(wallet_address, &pool_id)
    }

    /// Fetch LP position amounts of Raydium pool
    ///
    /// # Arguments
    /// - `wallet_address` - The wallet address
    /// - `pool_id` - The pool ID, e.g. 8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj (SOL-USDC.1bp Pool in Raydium mainnet)
    /// - `raydium_v3_program` - The Raydium V3 program ID, e.g. CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK (Raydium mainnet program ID)
    ///
    /// # Returns
    /// - `(u64, u64)` - The total amount of token 0 and token 1 in wallet_address's LP position in the pool
    pub fn raydium_pool_position(&self, wallet_address: &Pubkey, pool_id: &Pubkey) -> Result<(u64, u64)> {
        let raydium_v3_program = Pubkey::from_str(RAYDIUM_V3_PROGRAM).unwrap();
        let positions = self.get_nft_account_and_position_by_owner(
            &wallet_address,
            spl_token_2022::id(),
            &raydium_v3_program,
        );
        let positions: Vec<Pubkey> = positions
            .iter()
            .map(|item| item.position)
            .collect();
        let positions = self.rpc.get_multiple_accounts(&positions)?;

        let positions = positions.into_iter().filter_map(|p|
            match p {
                None => None,
                Some(rsp) => {
                    let position = deserialize_anchor_account::<
                        raydium_amm_v3::states::PersonalPositionState,
                    >(&rsp);
                    match position {
                        Err(_) => {
                            log::warn!("deserialize_anchor_account error");
                            None
                        }
                        Ok(position) => {
                            if position.pool_id == *pool_id {
                                Some(position)
                            } else {
                                None
                            }
                        }
                    }
                }
            }
        ).collect::<Vec<_>>();
        let mut amount_0 = 0;
        let mut amount_1 = 0;
        for position in positions {
            let tick_lower_price_x64 = tick_math::get_sqrt_price_at_tick(position.tick_lower_index)?;
            let tick_upper_price_x64 = tick_math::get_sqrt_price_at_tick(position.tick_upper_index)?;
            let delta_amount0 =
                get_delta_amount_0_unsigned(tick_lower_price_x64, tick_upper_price_x64, position.liquidity, true)?;
            let delta_amount1 =
                get_delta_amount_1_unsigned(tick_upper_price_x64, tick_lower_price_x64, position.liquidity, true)?;
            amount_0 += delta_amount0;
            amount_1 += delta_amount1;
        };
        Ok((amount_0, amount_1))
    }

    // Reference: https://github.com/raydium-io/raydium-clmm/blob/master/client/src/main.rs#L281
    fn get_nft_account_and_position_by_owner(
        &self,
        owner: &Pubkey,
        token_program: Pubkey,
        raydium_amm_v3_program: &Pubkey,
    ) -> Vec<PositionNftTokenInfo> {
        let all_tokens = self.rpc
            .get_token_accounts_by_owner(owner, TokenAccountsFilter::ProgramId(token_program))
            .unwrap();
        let mut position_nft_accounts = Vec::new();
        for keyed_account in all_tokens {
            if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
                if parsed_account.program == "spl-token" || parsed_account.program == "spl-token-2022" {
                    if let Ok(TokenAccountType::Account(ui_token_account)) =
                        serde_json::from_value(parsed_account.parsed)
                    {
                        let _frozen = ui_token_account.state == UiAccountState::Frozen;

                        let token = ui_token_account
                            .mint
                            .parse::<Pubkey>()
                            .unwrap_or_else(|err| panic!("Invalid mint: {}", err));
                        let token_account = keyed_account
                            .pubkey
                            .parse::<Pubkey>()
                            .unwrap_or_else(|err| panic!("Invalid token account: {}", err));
                        let token_amount = ui_token_account
                            .token_amount
                            .amount
                            .parse::<u64>()
                            .unwrap_or_else(|err| panic!("Invalid token amount: {}", err));

                        let _close_authority = ui_token_account.close_authority.map_or(*owner, |s| {
                            s.parse::<Pubkey>()
                                .unwrap_or_else(|err| panic!("Invalid close authority: {}", err))
                        });

                        if ui_token_account.token_amount.decimals == 0 && token_amount == 1 {
                            let (position_pda, _) = Pubkey::find_program_address(
                                &[
                                    raydium_amm_v3::states::POSITION_SEED.as_bytes(),
                                    token.to_bytes().as_ref(),
                                ],
                                &raydium_amm_v3_program,
                            );
                            position_nft_accounts.push(PositionNftTokenInfo {
                                key: token_account,
                                program: token_program,
                                position: position_pda,
                                mint: token,
                                amount: token_amount,
                                decimals: ui_token_account.token_amount.decimals,
                            });
                        }
                    }
                }
            }
        }
        position_nft_accounts
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PositionNftTokenInfo {
    key: Pubkey,
    program: Pubkey,
    position: Pubkey,
    mint: Pubkey,
    amount: u64,
    decimals: u8,
}

pub fn deserialize_anchor_account<T: AccountDeserialize>(account: &Account) -> Result<T> {
    let mut data: &[u8] = &account.data;
    T::try_deserialize(&mut data).map_err(Into::into)
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

    #[test]
    fn test_get_raydium_pool_position() {
        let fetcher = new_balancer_fetcher();
        let wallet = Pubkey::from_str("53zSj4G935ZY2a5x2UnGAiJXSuXXmGHaLph2zhAUvYpg").unwrap();
        // SOL-USDC.1bp Pool
        let pool_id = Pubkey::from_str("8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj").unwrap();
        let (amount_0, amount_1) = fetcher.raydium_pool_position(&wallet, &pool_id).unwrap();
        assert!(amount_0 > 0);
        assert!(amount_1 > 0);
    }
}
