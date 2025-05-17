use alloy::providers::{Provider, ProviderBuilder};
use thiserror::Error;

const RPC_URL: &str = "https://mainnet.infura.io/v3/";

#[derive(Error, Debug)]
pub enum UniswapDexError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Missing environment variable: {0}")]
    EnvVar(String),
}

pub async fn quote_exact_input_single() -> Result<(), UniswapDexError> {
    let api_key = std::env::var("INFURA_API_KEY")
        .map_err(|_| UniswapDexError::EnvVar("INFURA_API_KEY not set".to_string()))?;
    let url = format!("{}{}", RPC_URL, api_key);
    let provider = ProviderBuilder::new()
        .connect(&url)
        .await
        .map_err(|e| UniswapDexError::ApiError(e.to_string()))?;

    let balance = provider
        .get_balance(
            "0x246C5881E3F109B2aF170F5C773EF969d3da581B"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();

    dbg!(&balance);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quote_rates() {
        quote_exact_input_single().await.unwrap();
    }
}
