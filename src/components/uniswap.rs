use std::time::Duration;

use alloy::{
    primitives::{address, Address, U256},
    sol,
    sol_types::SolCall,
};
use reqwest::Client;
use serde_json::json;
use thiserror::Error;

use super::tokens::Token;

pub const URLS: [&str; 5] = [
    "https://cloudflare-eth.com",
    "https://eth.llamarpc.com",
    "https://eth.rpc.blxrbdn.com",
    "https://virginia.rpc.blxrbdn.com",
    "https://rpc.flashbots.net",
];
const WETH_ADDRESS: Address = address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
const FACTORY_ADDRESS: Address = address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");

sol! {
    #[sol(rpc)]
    contract IUniswapV2Factory {
        function getPair(address tokenA, address tokenB) external view returns (address pair);
    }
}

sol! {
    #[sol(rpc)]
    contract IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function token0() external view returns (address);
    }
}

#[derive(Error, Debug)]
pub enum UniswapDexError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Missing environment variable: {0}")]
    EnvVar(String),
}

fn create_eth_call_request(id: String, to: Address, data: Vec<u8>) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "eth_call",
        "params": [
            {
                "to": alloy::hex::encode_prefixed(to),
                "data": alloy::hex::encode_prefixed(data)
            },
            "latest"
        ]
    })
}

async fn send_batch_request(
    client: &Client,
    urls: &[&str],
    requests: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, UniswapDexError> {
    for url in urls {
        match client.post(*url).json(requests).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let responses: Vec<serde_json::Value> =
                        response.json().await.map_err(UniswapDexError::Reqwest)?;
                    if responses.iter().all(|r| r.get("result").is_some()) {
                        return Ok(responses);
                    }
                }
            }
            Err(_e) => {}
        }
    }
    Err(UniswapDexError::ApiError(
        "All nodes failed or returned errors".to_string(),
    ))
}

pub async fn get_token_prices_in_eth(
    tokens: &mut [Token],
    urls: &[&str],
) -> Result<(), UniswapDexError> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    let mut batch_requests = Vec::with_capacity(tokens.len());
    for (i, token) in tokens.iter().enumerate() {
        let get_pair_call = IUniswapV2Factory::getPairCall {
            tokenA: token.address.parse().unwrap_or_default(),
            tokenB: WETH_ADDRESS,
        };
        let data = get_pair_call.abi_encode();
        let request = create_eth_call_request(format!("getPair_{}", i), FACTORY_ADDRESS, data);
        batch_requests.push(request);
    }

    let responses = send_batch_request(&client, urls, &batch_requests).await?;

    let mut pair_addresses = vec![Address::ZERO; tokens.len()];
    for resp in responses {
        if let Some(id) = resp["id"].as_str() {
            if let Some(result) = resp["result"].as_str() {
                if let Some(index) = id
                    .strip_prefix("getPair_")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    let data = alloy::hex::decode(&result[2..]).map_err(|e| {
                        UniswapDexError::ApiError(format!("Hex decode error: {}", e))
                    })?;
                    let decoded = IUniswapV2Factory::getPairCall::abi_decode_returns(&data)
                        .map_err(|e| {
                            UniswapDexError::ApiError(format!("ABI decode error: {}", e))
                        })?;
                    pair_addresses[index] = decoded;
                }
            }
        }
    }

    let mut batch_requests_2 = Vec::new();
    for (i, &pair_address) in pair_addresses.iter().enumerate() {
        if pair_address != Address::ZERO {
            let get_reserves_call = IUniswapV2Pair::getReservesCall {};
            let data_reserves = get_reserves_call.abi_encode();
            let request_reserves =
                create_eth_call_request(format!("getReserves_{}", i), pair_address, data_reserves);
            batch_requests_2.push(request_reserves);

            let token0_call = IUniswapV2Pair::token0Call {};
            let data_token0 = token0_call.abi_encode();
            let request_token0 =
                create_eth_call_request(format!("token0_{}", i), pair_address, data_token0);
            batch_requests_2.push(request_token0);
        }
    }

    let responses_2 = send_batch_request(&client, urls, &batch_requests_2).await?;

    let mut reserves = vec![None; tokens.len()];
    let mut token0s = vec![None; tokens.len()];
    for resp in responses_2 {
        if let Some(id) = resp["id"].as_str() {
            if let Some(result) = resp["result"].as_str() {
                let data = alloy::hex::decode(&result[2..])
                    .map_err(|e| UniswapDexError::ApiError(format!("Hex decode error: {}", e)))?;
                if let Some(index) = id
                    .strip_prefix("getReserves_")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    let decoded = IUniswapV2Pair::getReservesCall::abi_decode_returns(&data)
                        .map_err(|e| {
                            UniswapDexError::ApiError(format!("ABI decode error: {}", e))
                        })?;
                    reserves[index] = Some((decoded.reserve0, decoded.reserve1));
                } else if let Some(index) = id
                    .strip_prefix("token0_")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    let decoded =
                        IUniswapV2Pair::token0Call::abi_decode_returns(&data).map_err(|e| {
                            UniswapDexError::ApiError(format!("ABI decode error: {}", e))
                        })?;
                    token0s[index] = Some(decoded);
                }
            }
        }
    }

    for i in 0..tokens.len() {
        if let (Some((reserve0, reserve1)), Some(token0_address)) = (reserves[i], token0s[i]) {
            let token_address = &tokens[i].address;
            let (reserve_token, reserve_weth) = if token_address == &token0_address.to_string() {
                (U256::from(reserve0), U256::from(reserve1))
            } else {
                (U256::from(reserve1), U256::from(reserve0))
            };

            if reserve_token != U256::ZERO {
                let reserve_weth_eth = f64::from(reserve_weth) / 1e18;
                let reserve_token_tokens =
                    f64::from(reserve_token) / 10f64.powi(tokens[i].decimals as i32);
                let new_rate = reserve_weth_eth / reserve_token_tokens;

                tokens[i].last_price = tokens[i].rate;
                tokens[i].rate = new_rate;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::tokens::{TokenStatus, TokenType};

    #[tokio::test]
    async fn test_get_token_prices_in_eth() {
        let mut tokens = vec![
            Token {
                address: "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(), // DAI
                scope: 0,
                name: "Dai Stablecoin".to_string(),
                symbol: "DAI".to_string(),
                token_type: TokenType::FT,
                decimals: 18,
                listed: true,
                status: TokenStatus::Available,
                chain_id: 1,
                rate: 0.0,
                last_price: 0.0,
            },
            Token {
                address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(), // USDC
                scope: 0,
                name: "USD Coin".to_string(),
                symbol: "USDC".to_string(),
                token_type: TokenType::FT,
                decimals: 6,
                listed: true,
                status: TokenStatus::Available,
                chain_id: 1,
                rate: 0.0,
                last_price: 0.0,
            },
            Token {
                address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(), // USDC
                scope: 0,
                name: "Wrapped Bitcoin".to_string(),
                symbol: "WBTC".to_string(),
                token_type: TokenType::FT,
                decimals: 8,
                listed: true,
                status: TokenStatus::Available,
                chain_id: 1,
                rate: 0.0,
                last_price: 0.0,
            },
        ];

        let urls = URLS.to_vec();

        get_token_prices_in_eth(&mut tokens, &urls)
            .await
            .expect("Failed to fetch token prices");

        dbg!(&tokens);

        assert!(tokens[0].rate > 0.0, "DAI price should be positive");
        assert!(tokens[1].rate > 0.0, "USDC price should be positive");
        assert!(tokens[2].rate > 0.0, "WBTC price should be positive");
    }
}
