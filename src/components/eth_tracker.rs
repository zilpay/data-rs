// use std::fs;

// use super::{
//     tokens::{coingecko_get_tokens, Token},
//     uniswap::get_token_prices_in_eth,
// };
// use bincode::config::{self, Configuration};
// use thiserror::Error;

// const ETH_STORAGE_KEY: &str = "eth_tokens";

// #[derive(Error, Debug)]
// pub enum EthTrackerError {
//     #[error("Bincode serialization error: {0}")]
//     Bincode(String),

//     #[error("Token fetch error: {0}")]
//     TokenFetch(#[from] super::tokens::TokenQuotesError),

//     #[error("Uniswap price fetch error: {0}")]
//     Uniswap(#[from] super::uniswap::UniswapDexError),
// }

// pub struct EthTracker {
//     bincode_config: Configuration,
// }

// impl EthTracker {
//     pub fn new(db_path: &str) -> Result<Self, EthTrackerError> {
//         let db_full_path = format!("{}/eth_tracker_db", db_path);

//         fs::create_dir_all(db_path).unwrap_or_default();

//         opts.create_if_missing(true);
//         let bincode_config = config::standard();
//         Ok(EthTracker { bincode_config })
//     }

//     pub fn get_tokens(&self) -> Result<Vec<Token>, EthTrackerError> {
//         if let Some(encoded) = self.db.get(ETH_STORAGE_KEY)? {
//             let (tokens, _): (Vec<Token>, usize) =
//                 bincode::decode_from_slice(&encoded[..], self.bincode_config)
//                     .map_err(|e| EthTrackerError::Bincode(e.to_string()))?;
//             Ok(tokens)
//         } else {
//             Ok(vec![])
//         }
//     }

//     pub fn save_tokens(&self, tokens: Vec<Token>) -> Result<(), EthTrackerError> {
//         let bytes = bincode::encode_to_vec(tokens, self.bincode_config)
//             .map_err(|e| EthTrackerError::Bincode(e.to_string()))?;

//         self.db.put(ETH_STORAGE_KEY, bytes)?;
//         self.db.flush()?;

//         Ok(())
//     }

//     pub async fn update_tokens_from_coingecko(&self) -> Result<(), EthTrackerError> {
//         let mut current_tokens = self.get_tokens()?;
//         let mut new_tokens = coingecko_get_tokens("ethereum").await?;

//         new_tokens.truncate(1500);

//         for new_token in new_tokens {
//             if !current_tokens
//                 .iter()
//                 .any(|t| t.address == new_token.address)
//             {
//                 current_tokens.push(new_token);
//             }
//         }

//         self.save_tokens(current_tokens)?;
//         Ok(())
//     }

//     pub async fn update_rates_from_uniswap(&self) -> Result<(), EthTrackerError> {
//         let mut tokens = self.get_tokens()?;
//         get_token_prices_in_eth(&mut tokens).await?;
//         self.save_tokens(tokens)?;
//         Ok(())
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::components::tokens::{Token, TokenStatus, TokenType};
//     use tempfile::tempdir;

//     fn create_mock_token(address: &str) -> Token {
//         Token {
//             address: address.to_string(),
//             scope: 1,
//             name: "Test Token".to_string(),
//             symbol: "TST".to_string(),
//             token_type: TokenType::FT,
//             decimals: 18,
//             listed: true,
//             status: TokenStatus::Available,
//             chain_id: 1,
//             rate: 0.0,
//             last_price: 0.0,
//         }
//     }

//     fn setup_tracker() -> EthTracker {
//         let temp_dir = tempdir().unwrap();
//         let db_path = temp_dir.path().to_str().unwrap();

//         EthTracker::new(db_path).unwrap()
//     }

//     #[test]
//     fn test_new_tracker() {
//         let tracker = setup_tracker();
//         assert!(tracker.db.get(ETH_STORAGE_KEY).unwrap().is_none());
//     }

//     #[test]
//     fn test_save_and_get_tokens() {
//         let tracker = setup_tracker();
//         let tokens = vec![create_mock_token("0x1"), create_mock_token("0x2")];

//         // Save tokens
//         tracker.save_tokens(tokens.clone()).unwrap();

//         // Retrieve tokens
//         let retrieved = tracker.get_tokens().unwrap();
//         assert_eq!(retrieved.len(), 2);
//         assert_eq!(retrieved[0].address, "0x1");
//         assert_eq!(retrieved[1].address, "0x2");
//     }

//     #[tokio::test]
//     async fn test_get_empty_tokens() {
//         let tracker = setup_tracker();
//         tracker.update_tokens_from_coingecko().await.unwrap();
//         let tokens = tracker.get_tokens().unwrap();

//         dbg!(&tokens);
//         // assert!(tokens.is_empty());
//     }
// }
