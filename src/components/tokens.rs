use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenQuotesError {
    #[error("API request error: {0}")]
    ApiRequestError(String),

    #[error("Response parsing error: {0}, content: {1}")]
    ParseResponseError(String, String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TokenType {
    FT,
    NFT,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TokenStatus {
    Available,
    Buned,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    pub address: String,
    pub scope: u64,
    pub name: String,
    pub symbol: String,
    pub token_type: TokenType,
    pub decimals: u8,
    pub listed: bool,
    pub status: TokenStatus,
    pub chain_id: u64,
    pub rate: f64,
    pub last_price: f64,
}

pub type Result<T> = std::result::Result<T, TokenQuotesError>;

pub async fn pancakeswap_get_tokens() -> Result<Vec<Token>> {
    let url = "https://tokens.pancakeswap.finance/pancakeswap-extended.json";

    let client = Client::new();
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| TokenQuotesError::ApiRequestError(e.to_string()))?;
    let json: Value = res
        .json()
        .await
        .map_err(|e| TokenQuotesError::ApiRequestError(e.to_string()))?;

    let api_tokens: Vec<serde_json::Value> = json["tokens"]
        .as_array()
        .ok_or_else(|| {
            TokenQuotesError::ParseResponseError(
                "Invalid tokens format".to_string(),
                json.to_string(),
            )
        })?
        .clone();

    let total_length = api_tokens.len() as u64;
    let tokens: Vec<Token> = api_tokens
        .into_iter()
        .enumerate()
        .map(|(index, token)| Token {
            scope: total_length - index as u64,
            chain_id: token["chainId"].as_u64().unwrap_or_default(),
            name: token["name"].as_str().unwrap_or_default().to_string(),
            symbol: token["symbol"].as_str().unwrap_or_default().to_string(),
            decimals: token["decimals"].as_u64().unwrap_or(0) as u8,
            address: token["address"].as_str().unwrap_or_default().to_string(),
            token_type: TokenType::FT,
            listed: true,
            status: TokenStatus::Available,
            rate: 0.0,
            last_price: 0.0,
        })
        .collect();

    Ok(tokens)
}

pub async fn coingecko_get_tokens(chain_name: &str) -> Result<Vec<Token>> {
    let url = format!("https://tokens.coingecko.com/{}/all.json", chain_name);

    let client = Client::new();
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| TokenQuotesError::ApiRequestError(e.to_string()))?;
    let json: Value = res
        .json()
        .await
        .map_err(|e| TokenQuotesError::ApiRequestError(e.to_string()))?;

    let api_tokens: Vec<serde_json::Value> = json["tokens"]
        .as_array()
        .ok_or_else(|| {
            TokenQuotesError::ParseResponseError(
                "Invalid tokens format".to_string(),
                json.to_string(),
            )
        })?
        .clone();

    let total_length = api_tokens.len() as u64;
    let tokens: Vec<Token> = api_tokens
        .into_iter()
        .enumerate()
        .map(|(index, token)| Token {
            scope: total_length - index as u64,
            chain_id: token["chainId"].as_u64().unwrap_or_default(),
            name: token["name"].as_str().unwrap_or_default().to_string(),
            symbol: token["symbol"].as_str().unwrap_or_default().to_string(),
            decimals: token["decimals"].as_u64().unwrap_or(0) as u8,
            address: token["address"].as_str().unwrap_or_default().to_string(),
            token_type: TokenType::FT,
            listed: true,
            status: TokenStatus::Available,
            rate: 0.0,
            last_price: 0.0,
        })
        .collect();

    Ok(tokens)
}

#[cfg(test)]
mod pancakeswap_get_tokens_tests {

    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_get_tokens_bsc() {
        let tokens = pancakeswap_get_tokens().await.unwrap();
        assert!(!tokens.is_empty());
        let token = &tokens[0];
        assert!(!token.name.is_empty());
        assert!(!token.symbol.is_empty());
        assert!(token.decimals > 0);
        assert_eq!(token.scope, tokens.len() as u64);
        assert_eq!(tokens.last().unwrap().scope, 1);
    }

    #[tokio::test]
    async fn test_get_tokens() {
        let tokens = coingecko_get_tokens("base").await.unwrap();
        assert!(!tokens.is_empty());
        let token = &tokens[0];
        assert!(!token.name.is_empty());
        assert!(!token.symbol.is_empty());
        assert!(token.decimals > 0);
        assert_eq!(token.scope, tokens.len() as u64);
        assert_eq!(tokens.last().unwrap().scope, 1);
    }

    #[tokio::test]
    async fn test_get_tokens_chain() {
        let tokens = coingecko_get_tokens("ethereum").await.unwrap();
        assert!(!tokens.is_empty());
        let token = &tokens[0];
        assert!(!token.name.is_empty());
        assert!(!token.symbol.is_empty());
        assert!(token.decimals > 0);
        assert_eq!(token.scope, tokens.len() as u64);
        assert_eq!(tokens.last().unwrap().scope, 1);
    }

    #[tokio::test]
    async fn test_get_tokens_default_values() {
        let tokens = coingecko_get_tokens("tron").await.unwrap();
        assert!(!tokens.is_empty());
        let token = &tokens[0];
        assert!(!token.name.is_empty());
        assert!(!token.symbol.is_empty());
        assert!(token.decimals > 0);
        assert_eq!(token.scope, tokens.len() as u64);
        assert_eq!(tokens.last().unwrap().scope, 1);
    }

    // #[tokio::test]
    // async fn test_update_rates() {
    //     let tokens = pancakeswap_get_tokens().await.unwrap();
    //     let symbols: Vec<&str> = tokens.iter().map(|t| t.symbol.as_str()).collect();

    //     let chunk_size = CRYPTOCOMPARE_TOKENS_LIMIT;
    //     let mut all_rates = HashMap::new();

    //     for chunk in symbols.chunks(chunk_size) {
    //         dbg!(chunk.len());
    //         let rates = get_cryptocompare_prices(chunk).await.unwrap();
    //         all_rates.extend(rates);
    //     }

    //     dbg!(&all_rates);

    //     for symbol in &symbols {
    //         assert!(
    //             all_rates.contains_key(*symbol),
    //             "Missing rate for {}",
    //             symbol
    //         );
    //     }
    // }
}
