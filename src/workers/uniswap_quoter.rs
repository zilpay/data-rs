use crate::{
    components::{
        tokens::{pancakeswap_get_tokens, coingecko_get_tokens, Token, TokenType}, // Assuming these are the functions to get tokens
        uniswap::get_token_quote, // Our Uniswap quote function
    },
    db::quotes::save_quote, // Our Sled save function
    models::quotes::TokenQuote, // The quote data structure
};
use log::{error, info, warn};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use num_bigint::BigUint;
use std::str::FromStr;

// Configuration for the worker
const WORKER_LOOP_DELAY_SECONDS: u64 = 300; // 5 minutes
const ETH_DECIMALS: u8 = 18; // ETH has 18 decimals

// A common pool fee for major pairs on Uniswap V3 (e.g., 0.3%).
// This might need to be more dynamic in a production system (e.g., fetched per pool or configurable).
const DEFAULT_POOL_FEE: u32 = 3000;

pub async fn run_uniswap_quoter_worker() {
    info!("Uniswap Quoter Worker started.");

    loop {
        info!("Uniswap Quoter Worker: Fetching tokens...");

        // --- Token Retrieval ---
        // Combine tokens from different sources if necessary.
        // For this example, let's assume we primarily want ERC20 tokens from Ethereum (chain_id: 1)
        // that are likely to be on Uniswap.
        let mut all_tokens: Vec<Token> = Vec::new();

        match coingecko_get_tokens("ethereum").await {
            Ok(mut tokens) => {
                // Filter for Ethereum mainnet tokens (chain_id == 1) if not already filtered by the function
                tokens.retain(|token| token.chain_id == 1 && token.token_type == TokenType::FT);
                all_tokens.extend(tokens);
                info!("Fetched {} Ethereum FT tokens from CoinGecko.", all_tokens.len());
            }
            Err(e) => {
                error!("Uniswap Quoter Worker: Failed to fetch CoinGecko tokens: {:?}", e);
            }
        }
        
        // Example: Potentially add other token sources like PancakeSwap, filtering for Ethereum tokens
        // match pancakeswap_get_tokens().await {
        //     Ok(mut tokens) => {
        //         tokens.retain(|token| token.chain_id == 1 && token.token_type == TokenType::FT);
        //         info!("Fetched {} potential Ethereum FT tokens from PancakeSwap.", tokens.len());
        //         all_tokens.extend(tokens);
        //     }
        //     Err(e) => {
        //         error!("Uniswap Quoter Worker: Failed to fetch PancakeSwap tokens: {:?}", e);
        //     }
        // }

        // Deduplicate tokens by address if sources might overlap
        all_tokens.sort_by_key(|t| t.address.clone());
        all_tokens.dedup_by_key(|t| t.address.clone());
        
        info!("Uniswap Quoter Worker: Total unique Ethereum tokens to process: {}", all_tokens.len());

        for token in all_tokens {
            if token.address.is_empty() {
                warn!("Uniswap Quoter Worker: Token has empty address. Skipping.");
                continue;
            }
            if token.decimals == 0 {
                // Uniswap requires knowing tokenIn decimals. If 0, it's likely an error or not suitable.
                warn!("Uniswap Quoter Worker: Token {} ({}) has 0 decimals. Skipping.", token.name, token.address);
                continue;
            }

            info!("Uniswap Quoter Worker: Getting quote for {} ({})", token.name, token.address);

            match get_token_quote(&token.address, token.decimals, DEFAULT_POOL_FEE).await {
                Ok((amount_out_hex, timestamp)) => {
                    // Convert hex amount_out to f64, considering ETH decimals
                    match BigUint::from_str_radix(&amount_out_hex, 16) {
                        Ok(amount_out_biguint) => {
                            // Perform calculation using f64 for simplicity in representing price.
                            // amount_out_biguint / 10^ETH_DECIMALS
                            let eth_price = amount_out_biguint.to_string().parse::<f64>().unwrap_or(0.0) / (10f64.powi(ETH_DECIMALS as i32));

                            if eth_price == 0.0 {
                                warn!("Uniswap Quoter Worker: Calculated ETH price is 0 for {} ({}) from hex {}. Skipping save.", token.name, token.address, amount_out_hex);
                                continue;
                            }

                            let token_quote = TokenQuote {
                                token_address: token.address.clone(),
                                eth_price,
                                timestamp,
                            };

                            info!("Uniswap Quoter Worker: Successfully quoted {} ({}): 1 {} = {} ETH. Timestamp: {}",
                                token.name, token.address, token.symbol, token_quote.eth_price, token_quote.timestamp);

                            if let Err(e) = save_quote(&token.address, &token_quote) {
                                error!("Uniswap Quoter Worker: Failed to save quote for {} ({}): {:?}", token.name, token.address, e);
                            }
                        }
                        Err(e) => {
                            error!("Uniswap Quoter Worker: Failed to parse amount_out_hex '{}' for token {} ({}): {:?}", amount_out_hex, token.name, token.address, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Uniswap Quoter Worker: Failed to get quote for {} ({}): {:?}", token.name, token.address, e);
                }
            }
            // Add a small delay to avoid hitting rate limits too quickly if any
            sleep(Duration::from_millis(500)).await;
        }

        info!("Uniswap Quoter Worker: Finished processing all tokens. Sleeping for {} seconds.", WORKER_LOOP_DELAY_SECONDS);
        sleep(Duration::from_secs(WORKER_LOOP_DELAY_SECONDS)).await;
    }
}
