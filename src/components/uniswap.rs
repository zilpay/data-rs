use serde_json::json;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use num_bigint::BigUint;
use hex;

#[derive(Error, Debug)]
pub enum UniswapDexError {
    #[error("Environment variable error: {0}")]
    EnvVar(String),
    #[error("HTTP request error: {0}")]
    HttpRequest(String),
    #[error("JSON-RPC error: {0}")]
    JsonRpcError(String),
    #[error("ABI encoding error: {0}")]
    AbiEncodeError(String),
    #[error("ABI decoding error: {0}")]
    AbiDecodeError(String),
    #[error("Address parsing error: {0}")]
    AddressParseError(String),
    #[error("Hex decoding error: {0}")]
    HexError(#[from] hex::FromHexError),
}

fn abi_encode_address_padded(addr: &str) -> Result<String, UniswapDexError> {
    let trimmed_addr = addr.strip_prefix("0x").unwrap_or(addr);
    if trimmed_addr.len() != 40 {
        return Err(UniswapDexError::AddressParseError(format!(
            "Invalid address length: {}",
            trimmed_addr
        )));
    }
    hex::decode(trimmed_addr).map_err(|e| UniswapDexError::AddressParseError(format!("Invalid hex in address: {}", e)))?;
    Ok(format!("{:0>64}", trimmed_addr))
}

fn abi_encode_uint_padded<T: std::fmt::LowerHex>(val: T, size_bytes: usize) -> String {
    if size_bytes == 3 { 
        format!("{:0>64}", format!("{:06x}", val))
    } else if size_bytes == 20 { 
        format!("{:0>64}", format!("{:040x}", val))
    } else { 
        format!("{:0>64}", format!("{:064x}", val))
    }
}

pub async fn get_token_quote(
    token_in_address: &str,
    token_in_decimals: u8,
    pool_fee: u32,
    base_url_override: Option<String>, 
) -> Result<(String, u64), UniswapDexError> {
    let api_key = std::env::var("INFURA_API_KEY")
        .map_err(|e| UniswapDexError::EnvVar(format!("INFURA_API_KEY not set: {}", e)))?;
    let base_url = base_url_override.unwrap_or_else(|| "https://mainnet.infura.io/v3".to_string());

    const QUOTER_V2_ADDRESS: &str = "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6";
    const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    const FUNCTION_SELECTOR: &str = "7a05c363";

    let token_in_encoded = abi_encode_address_padded(token_in_address)?;
    let token_out_encoded = abi_encode_address_padded(WETH_ADDRESS)?;
    let fee_hex = format!("{:06x}", pool_fee);
    let fee_encoded = format!("{:0>64}", fee_hex);
    let amount_in_val = BigUint::from(10u32).pow(token_in_decimals as u32);
    let amount_in_encoded = abi_encode_uint_padded(&amount_in_val, 32);
    let sqrt_price_limit_x96_encoded = abi_encode_uint_padded(BigUint::from(0u32), 20);

    let data_string = format!(
        "0x{}{}{}{}{}{}",
        FUNCTION_SELECTOR,
        token_in_encoded,
        token_out_encoded,
        fee_encoded,
        amount_in_encoded,
        sqrt_price_limit_x96_encoded
    );

    let infura_url = format!("{}/{}", base_url, api_key);
    let client = reqwest::Client::new();
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": QUOTER_V2_ADDRESS, "data": data_string}, "latest"],
        "id": 1
    });

    let response = client
        .post(&infura_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| UniswapDexError::HttpRequest(e.to_string()))?;

    if !response.status().is_success() {
        return Err(UniswapDexError::HttpRequest(format!(
            "HTTP error: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )));
    }

    let rpc_response_text = response.text().await.map_err(|e| UniswapDexError::HttpRequest(format!("Failed to read response text: {}", e)))?;
    let rpc_response: serde_json::Value = serde_json::from_str(&rpc_response_text)
        .map_err(|e| UniswapDexError::JsonRpcError(format!("Failed to parse JSON-RPC response: {}. Response text: {}", e, rpc_response_text)))?;

    if let Some(error) = rpc_response.get("error") {
        return Err(UniswapDexError::JsonRpcError(format!(
            "Error in JSON-RPC response: {}",
            error
        )));
    }

    let result = rpc_response
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| UniswapDexError::JsonRpcError("Missing 'result' field or not a string in JSON-RPC response".to_string()))?;

    let result_trimmed = result.strip_prefix("0x").unwrap_or(result);
    if result_trimmed.len() < 64 { 
        return Err(UniswapDexError::AbiDecodeError(format!(
            "Result too short to decode amountOut: {}",
            result_trimmed
        )));
    }
    let amount_out_hex = result_trimmed[..64].to_string();

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| UniswapDexError::AbiEncodeError(format!("System time error: {}", e)))?
        .as_secs();

    Ok((amount_out_hex, timestamp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH, Duration};

    const DUMMY_API_KEY: &str = "test_api_key";
    const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    const USDC_DECIMALS: u8 = 6;
    const DEFAULT_POOL_FEE: u32 = 3000;

    struct EnvGuard {
        key: String,
        original_value: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &str, value: &str) -> Self {
            let original_value = env::var(key).ok();
            env::set_var(key, value);
            EnvGuard { key: key.to_string(), original_value }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(val) = &self.original_value {
                env::set_var(&self.key, val);
            } else {
                env::remove_var(&self.key);
            }
        }
    }

    #[tokio::test]
    async fn get_token_quote_success() {
        let _guard = EnvGuard::new("INFURA_API_KEY", DUMMY_API_KEY);
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url(); // This is just the base e.g. http://127.0.0.1:1234

        let expected_amount_out_hex = "0000000000000000000000000000000000000000000000000de0b6b3a7640000"; // Example: 1 ETH
        let mock_response_data = format!(
            "0x{}{}00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // Other return values, padded
            expected_amount_out_hex, // amountOut
            "0000000000000000000000000000000000000000000000000000000000000000" // sqrtPriceX96After (dummy)
        );

        let mock = server.mock("POST", &*format!("/{}", DUMMY_API_KEY)) // Mockito path needs to be relative
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": mock_response_data
            }).to_string())
            .create_async().await;

        let before_call_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let result = get_token_quote(USDC_ADDRESS, USDC_DECIMALS, DEFAULT_POOL_FEE, Some(mock_url)).await;
        let after_call_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        mock.assert_async().await;
        assert!(result.is_ok());
        let (amount_out_hex, timestamp) = result.unwrap();
        assert_eq!(amount_out_hex, expected_amount_out_hex);
        assert!(timestamp >= before_call_ts && timestamp <= after_call_ts);
    }

    #[tokio::test]
    async fn get_token_quote_infura_error() {
        let _guard = EnvGuard::new("INFURA_API_KEY", DUMMY_API_KEY);
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server.mock("POST", &*format!("/{}", DUMMY_API_KEY))
            .with_status(200) // HTTP success, but RPC error
            .with_header("content-type", "application/json")
            .with_body(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": {"code": -32000, "message": "Some Infura error"}
            }).to_string())
            .create_async().await;

        let result = get_token_quote(USDC_ADDRESS, USDC_DECIMALS, DEFAULT_POOL_FEE, Some(mock_url)).await;
        
        mock.assert_async().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            UniswapDexError::JsonRpcError(_) => {} // Expected error
            e => panic!("Unexpected error type: {:?}", e),
        }
    }

    #[tokio::test]
    async fn get_token_quote_bad_response_format_non_json() {
        let _guard = EnvGuard::new("INFURA_API_KEY", DUMMY_API_KEY);
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server.mock("POST", &*format!("/{}", DUMMY_API_KEY))
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("This is not JSON")
            .create_async().await;

        let result = get_token_quote(USDC_ADDRESS, USDC_DECIMALS, DEFAULT_POOL_FEE, Some(mock_url)).await;
        
        mock.assert_async().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            UniswapDexError::JsonRpcError(_) => {} // serde_json::from_str will fail
            e => panic!("Unexpected error type: {:?}", e),
        }
    }
    
    #[tokio::test]
    async fn get_token_quote_bad_response_format_missing_result() {
        let _guard = EnvGuard::new("INFURA_API_KEY", DUMMY_API_KEY);
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server.mock("POST", &*format!("/{}", DUMMY_API_KEY))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "jsonrpc": "2.0",
                "id": 1
                // "result" field is missing
            }).to_string())
            .create_async().await;

        let result = get_token_quote(USDC_ADDRESS, USDC_DECIMALS, DEFAULT_POOL_FEE, Some(mock_url)).await;
        
        mock.assert_async().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            UniswapDexError::JsonRpcError(msg) => {
                assert!(msg.contains("Missing 'result' field"));
            }
            e => panic!("Unexpected error type: {:?}", e),
        }
    }

    #[tokio::test]
    async fn get_token_quote_invalid_token_address_format() {
        // No mock server needed as this should fail before HTTP request
        let _guard = EnvGuard::new("INFURA_API_KEY", DUMMY_API_KEY); // Still need API key for initial check

        let invalid_address = "0xINVALID_ADDRESS"; // Too short, not valid hex chars
        let result = get_token_quote(invalid_address, USDC_DECIMALS, DEFAULT_POOL_FEE, None).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            UniswapDexError::AddressParseError(_) => {} // Expected
            e => panic!("Unexpected error type: {:?}", e),
        }

        let invalid_address_len = "0x12345678901234567890123456789012345678"; // Too short
        let result_len = get_token_quote(invalid_address_len, USDC_DECIMALS, DEFAULT_POOL_FEE, None).await;
        assert!(result_len.is_err());
         match result_len.unwrap_err() {
            UniswapDexError::AddressParseError(msg) => {
                 assert!(msg.contains("Invalid address length"));
            }
            e => panic!("Unexpected error type: {:?}", e),
        }
    }
}
