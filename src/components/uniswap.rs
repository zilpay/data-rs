// This file implements functions for interacting with Uniswap V3, specifically for
// fetching token prices against ETH using the Quoter V2 contract.
// It involves building JSON-RPC batch requests, executing them via Infura,
// and parsing the responses.

use alloy_primitives::{Address, U256, utils as alloy_utils};
use serde_json::json;
// Removed: use alloy::providers::{Provider, ProviderBuilder}; // Not used by new functions
use thiserror::Error;
use alloy_json_abi::Function;
use alloy_sol_types::{sol_data, SolType, SolValue};
use reqwest::Client;
use std::collections::HashSet; // For tracking IDs in parse_batch_quote_response

// The base RPC URL for Infura mainnet. The API key will be appended to this.
const RPC_URL: &str = "https://mainnet.infura.io/v3/";

// Ethereum Address Constants
/// Wrapped Ether (WETH) address on Ethereum mainnet. Used as the output token for quotes.
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
/// Uniswap V3 Quoter V2 contract address on Ethereum mainnet.
const UNISWAP_V3_QUOTER_V2_ADDRESS: &str = "0x61fFE014bA17989E743c5F6d46519827a8b74267";

// Uniswap V3 Quoting Constants
/// Default amount of input token for quotes (10^18, typically representing 1 token unit if 18 decimals).
const DEFAULT_QUOTE_AMOUNT_IN: U256 = U256::from_limbs([1_000_000_000_000_000_000u64, 0, 0, 0]);
/// Default fee tier for Uniswap V3 pools (e.g., 0.3% = 3000).
const DEFAULT_FEE_TIER: u32 = 3000;


/// Represents the price of a specific token in terms of ETH.
#[derive(Debug)]
pub struct TokenPrice {
    /// The ERC20 contract address of the token.
    pub address: String,
    /// The price of one unit of the token, expressed in ETH.
    pub price_in_eth: f64,
}

/// Defines the possible errors that can occur while interacting with the Uniswap DEX module.
#[derive(Error, Debug)]
pub enum UniswapDexError {
    /// Error originating from the HTTP request library (`reqwest`).
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    /// Error reported by the Ethereum JSON-RPC API.
    #[error("API error: {0}")]
    ApiError(String),

    /// Error due to a missing required environment variable (e.g., Infura API key).
    #[error("Missing environment variable: {0}")]
    EnvVar(String),

    /// Error related to smart contract calls (currently unused but kept for potential future use).
    #[error("Smart contract call failed: {0}")]
    ContractCall(String),

    /// Error during data conversion, e.g., parsing hex strings to numbers.
    #[error("Data conversion error: {0}")]
    DataConversion(String),

    /// Error due to invalid input parameters provided to a function.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Error during JSON serialization or deserialization.
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    /// Error during ABI encoding or decoding of smart contract parameters.
    #[error("ABI encoding error: {0}")]
    AbiEncoding(#[from] alloy_sol_types::Error),
}

/// (Private) Builds a single JSON-RPC payload for the `quoteExactInputSingle` method call.
///
/// This function constructs the necessary data for calling the Uniswap V3 Quoter V2's
/// `quoteExactInputSingle` function, which provides an estimated amount of output token
/// (WETH in this case) for a given input token and amount.
///
/// # Arguments
///
/// * `request_id` - A unique identifier for this specific JSON-RPC request within a batch.
/// * `token_in_address` - The contract address of the input token (the token to be priced).
///
/// # Returns
///
/// A `Result` containing the JSON-RPC call object as a `serde_json::Value`,
/// or an `UniswapDexError` if payload construction fails (e.g., invalid address, ABI encoding error).
fn build_quote_json_rpc_payload(
    request_id: u64,
    token_in_address: &str,
) -> Result<serde_json::Value, UniswapDexError> {
    // Parse the input token address string into an Address type.
    let token_in_addr = token_in_address
        .parse::<Address>()
        .map_err(|e| UniswapDexError::InvalidInput(format!("Invalid token_in_address: {}. Error: {}", token_in_address, e)))?;
    
    // Parse the constant WETH address. Should not fail.
    let weth_addr = WETH_ADDRESS
        .parse::<Address>()
        .expect("Internal error: WETH_ADDRESS constant should be a valid address.");

    // Parse the constant Quoter V2 address. Should not fail.
    let quoter_v2_addr = UNISWAP_V3_QUOTER_V2_ADDRESS
        .parse::<Address>()
        .expect("Internal error: UNISWAP_V3_QUOTER_V2_ADDRESS constant should be a valid address.");

    // Define the function signature for `quoteExactInputSingle`.
    let function_signature = "quoteExactInputSingle(address,address,uint24,uint256,uint160)";
    // Parse the signature. This is critical and should not fail with a valid signature.
    let func = Function::parse(function_signature)
        .expect("Internal error: Failed to parse Uniswap Quoter function signature. Please check the signature string.");

    // Define the Solidity types of the parameters for `quoteExactInputSingle`.
    // (tokenIn, tokenOut, fee, amountIn, sqrtPriceLimitX96)
    let param_types: Vec<&dyn SolType> = vec![
        &Address::sol_type(),   // tokenIn: address (input token)
        &Address::sol_type(),   // tokenOut: address (WETH)
        &SolType::Uint(24),     // fee: uint24 (pool fee tier)
        &U256::sol_type(),      // amountIn: uint256 (amount of input token)
        &SolType::Uint(160),    // sqrtPriceLimitX96: uint160 (0 for no limit)
    ];

    // Define the actual values for these parameters.
    let param_values: Vec<SolValue> = vec![
        SolValue::Address(token_in_addr),                  // Input token address
        SolValue::Address(weth_addr),                      // Output token address (WETH)
        SolValue::Uint(U256::from(DEFAULT_FEE_TIER), 24),  // Pool fee tier
        SolValue::from(DEFAULT_QUOTE_AMOUNT_IN),           // Amount of input token
        SolValue::Uint(U256::ZERO, 160),                   // sqrtPriceLimitX96 (0 for no limit)
    ];
    
    // ABI-encode the parameters.
    let encoded_args = sol_data::encode_params(&param_types, &param_values)?;
    
    // Prepend the function selector to the ABI-encoded arguments to get the full call data.
    let call_data = func.selector_and_args(&encoded_args);

    // Construct the JSON-RPC request object.
    Ok(json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "eth_call", // Using eth_call to simulate the transaction and get return value
        "params": [{
            "to": quoter_v2_addr.to_string(), // The Quoter V2 contract address
            "data": format!("0x{}", hex::encode(call_data)) // The ABI-encoded call data
        }, "latest"] // Perform the call on the latest block
    }))
}

/// Builds a batch of JSON-RPC requests for quoting multiple token prices against WETH.
///
/// Each request in the batch is structured to call the `quoteExactInputSingle` method
/// of the Uniswap V3 Quoter V2 contract.
///
/// # Arguments
///
/// * `token_addresses` - A slice of token contract addresses (as strings) for which prices are needed.
///
/// # Returns
///
/// A `Result` containing a `serde_json::Value` representing the array of JSON-RPC call objects,
/// or an `UniswapDexError` if the input slice is empty or any individual payload construction fails.
pub fn build_batch_quote_request(
    token_addresses: &[String],
) -> Result<serde_json::Value, UniswapDexError> {
    if token_addresses.is_empty() {
        return Err(UniswapDexError::InvalidInput(
            "Token addresses slice cannot be empty.".to_string(),
        ));
    }

    let mut batch_requests = Vec::with_capacity(token_addresses.len());
    // Create a JSON-RPC payload for each token address.
    // Request IDs are 1-based and sequential.
    for (i, token_address) in token_addresses.iter().enumerate() {
        let request_id = (i + 1) as u64; // Ensure unique, non-zero IDs for the batch
        let payload = build_quote_json_rpc_payload(request_id, token_address)?;
        batch_requests.push(payload);
    }

    // The batch request is a JSON array of individual request objects.
    Ok(serde_json::Value::Array(batch_requests))
}

/// Executes a batch JSON-RPC request by sending it to an Ethereum node via Infura.
///
/// # Arguments
///
/// * `batch_payload` - A `serde_json::Value` representing the batch JSON-RPC request,
///   typically created by `build_batch_quote_request`.
///
/// # Returns
///
/// A `Result` containing the parsed `serde_json::Value` response from the Ethereum node,
/// or an `UniswapDexError` if an error occurs (e.g., network issue, API error, missing API key).
pub async fn execute_batch_quote_request(
    batch_payload: serde_json::Value,
) -> Result<serde_json::Value, UniswapDexError> {
    // Retrieve the Infura API key from environment variables.
    let api_key = std::env::var("INFURA_API_KEY")
        .map_err(|_| UniswapDexError::EnvVar("INFURA_API_KEY not set. Please set this environment variable.".to_string()))?;
    let rpc_url_with_key = format!("{}{}", RPC_URL, api_key);

    // Create a reqwest HTTP client.
    let client = Client::new();
    // Send the POST request with the batch payload.
    let response = client
        .post(&rpc_url_with_key)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&batch_payload)?) // Serialize payload to JSON string
        .send()
        .await?; // Propagate Reqwest errors (network, etc.)

    // Check if the HTTP request itself was successful (e.g., status 200 OK).
    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body from Infura.".to_string());
        return Err(UniswapDexError::ApiError(format!(
            "Infura API request failed with status {}: {}",
            status, error_body
        )));
    }

    // Parse the successful response body as JSON.
    // This can still fail if the body is not valid JSON, or if reqwest encounters an error here.
    let response_json: serde_json::Value = response.json().await?;

    Ok(response_json)
}

/// Parses a batch JSON-RPC response to extract the quoted amounts (as `U256`).
///
/// The function expects the response to be an array of JSON-RPC response objects.
/// It ensures that results are returned in an order corresponding to the original request IDs
/// (1 through `num_requests`).
///
/// # Arguments
///
/// * `response_payload` - A `serde_json::Value` from `execute_batch_quote_request`.
/// * `num_requests` - The total number of individual requests sent in the batch.
///
/// # Returns
///
/// A `Result` containing a `Vec<U256>` of the quoted amounts (amount of WETH in Wei),
/// ordered by their original request ID.
/// Returns an `UniswapDexError` if parsing fails, the response is malformed,
/// an error is present in any individual response object, or if not all responses are present.
pub fn parse_batch_quote_response(
    response_payload: serde_json::Value,
    num_requests: usize,
) -> Result<Vec<U256>, UniswapDexError> {
    // The top-level response should be a JSON array.
    let response_array = response_payload.as_array().ok_or_else(|| {
        UniswapDexError::ApiError("Batch response payload is not a JSON array.".to_string())
    })?;

    // Validate that the number of responses matches the number of requests.
    if response_array.len() != num_requests {
        return Err(UniswapDexError::ApiError(format!(
            "Mismatched response count: expected {}, got {}.",
            num_requests,
            response_array.len()
        )));
    }

    // Initialize a vector to store results in the order of request IDs.
    // `None` indicates a response for that ID hasn't been processed yet.
    let mut ordered_results: Vec<Option<U256>> = vec![None; num_requests];
    // Keep track of IDs found to detect duplicates and ensure all are present.
    let mut found_ids = HashSet::with_capacity(num_requests);

    for response_object in response_array {
        // Each item in the array should be a JSON object.
        // Extract the 'id' field.
        let id_val = response_object
            .get("id")
            .ok_or_else(|| UniswapDexError::ApiError("Missing 'id' in a response object.".to_string()))?;
        
        let id = id_val.as_u64().ok_or_else(|| {
            UniswapDexError::ApiError(format!("Invalid 'id' type in response object (expected u64): {:?}.", id_val))
        })?;

        // Validate the ID: must be within the expected range (1 to num_requests).
        if id == 0 || id > num_requests as u64 {
            return Err(UniswapDexError::ApiError(format!(
                "Invalid 'id' {} found in response object; expected range 1-{}.",
                id, num_requests
            )));
        }
        
        // Check for duplicate IDs in the response batch.
        if !found_ids.insert(id) {
            return Err(UniswapDexError::ApiError(format!("Duplicate 'id' {} found in response batch.", id)));
        }

        // Convert 1-based ID to 0-based index for `ordered_results` vector.
        let result_index = (id - 1) as usize;

        // Check if the response object contains an 'error' field.
        if let Some(error_obj) = response_object.get("error") {
            let error_message = error_obj
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown RPC error structure."); // Default if message field is missing/wrong type
            return Err(UniswapDexError::ApiError(format!(
                "RPC error for request id {}: {}",
                id, error_message
            )));
        }

        // Extract the 'result' field, which should contain the quoted amount as a hex string.
        let result_val = response_object.get("result").ok_or_else(|| {
            UniswapDexError::ApiError(format!("Missing 'result' field in response for id {}.", id))
        })?;

        let hex_str = result_val.as_str().ok_or_else(|| {
            UniswapDexError::ApiError(format!(
                "Invalid 'result' type (expected hex string) for id {}: {:?}.",
                id, result_val
            ))
        })?;

        // Parse the hex string (e.g., "0x...") into a U256 value.
        let parsed_value = U256::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .map_err(|e| {
                UniswapDexError::DataConversion(format!(
                    "Failed to parse result hex string '{}' for id {}: {}.",
                    hex_str, id, e
                ))
            })?;
        
        // Store the parsed value at the correct index.
        // This check is a safeguard, as `found_ids.insert` should prevent reprocessing.
        if ordered_results[result_index].is_some() {
            return Err(UniswapDexError::ApiError(format!("Duplicate processed data for id {}.", id)));
        }
        ordered_results[result_index] = Some(parsed_value);
    }
    
    // After processing all response objects, ensure all original request IDs were covered.
    if found_ids.len() != num_requests {
        // This implies some responses were missing, or there were issues not caught above.
        return Err(UniswapDexError::ApiError(
            "One or more request IDs were not found in the response batch.".to_string(),
        ));
    }

    // Convert `Vec<Option<U256>>` to `Vec<U256>`.
    // If any element is still `None` at this point, it indicates an internal logic error
    // as `found_ids.len() == num_requests` should ensure all slots are filled.
    let final_results: Vec<U256> = ordered_results.into_iter().map(|opt_val| {
        opt_val.ok_or_else(|| UniswapDexError::ApiError(
            "Internal error: A response ID was marked as found, but its data is missing.".to_string()
        ))
    }).collect::<Result<Vec<_>, _>>()?;

    Ok(final_results)
}

/// Fetches the current price in ETH for a list of token contract addresses using Uniswap V3.
///
/// This function orchestrates the process:
/// 1. Builds a batch of JSON-RPC requests for the given token addresses.
/// 2. Executes this batch request against an Ethereum node (Infura).
/// 3. Parses the batch response to extract the quoted amounts.
/// 4. Converts these amounts (which are in Wei) into ETH prices (`f64`) and packages them.
///
/// # Arguments
///
/// * `token_addresses` - A slice of token contract addresses (as strings).
///
/// # Returns
///
/// A `Result` containing a `Vec<TokenPrice>` where each `TokenPrice` includes the
/// original token address and its calculated price in ETH.
/// Returns an `UniswapDexError` if any step in the process fails.
pub async fn get_eth_prices_for_tokens(
    token_addresses: &[String],
) -> Result<Vec<TokenPrice>, UniswapDexError> {
    if token_addresses.is_empty() {
        // If no addresses are provided, return an empty vector immediately.
        return Ok(Vec::new());
    }

    let num_requests = token_addresses.len();

    // Step 1: Build the batch JSON-RPC request payload.
    let batch_payload = build_batch_quote_request(token_addresses)?;

    // Step 2: Execute the batch request via an HTTP client.
    let response_payload = execute_batch_quote_request(batch_payload).await?;

    // Step 3: Parse the batch response to get raw U256 quoted amounts (in Wei).
    let raw_quoted_amounts = parse_batch_quote_response(response_payload, num_requests)?;

    // Step 4: Convert raw quoted amounts (Wei) to ETH prices (f64) and create TokenPrice structs.
    let mut token_prices = Vec::with_capacity(num_requests);
    for (i, raw_amount_wei) in raw_quoted_amounts.iter().enumerate() {
        // The `raw_amount_wei` is the amount of WETH (in Wei) obtained for `DEFAULT_QUOTE_AMOUNT_IN`
        // of the input token. Since `DEFAULT_QUOTE_AMOUNT_IN` is 10^18 (representing 1 unit of an
        // 18-decimal token), `raw_amount_wei` effectively represents the price of 1 unit of the
        // input token in Wei.
        // We convert this Wei amount to its ETH equivalent (f64).
        let price_in_eth = alloy_utils::wei_to_ether_f64(*raw_amount_wei);

        token_prices.push(TokenPrice {
            address: token_addresses[i].clone(),
            price_in_eth,
        });
    }

    Ok(token_prices)
}

// The old `quote_exact_input_single` function is removed as it's not part of the main workflow
// and its functionality is superseded by the batch processing logic.

#[cfg(test)]
mod tests {
    use mockito; // Ensure mockito is in dev-dependencies
    use super::*;
    use alloy_primitives::{hex, U256}; 
    use serde_json::json;
    use std::env; // For testing with environment variables

    // The old test `test_quote_rates_old_function` is removed as the function it tested
    // (`quote_exact_input_single`) has been removed.

    #[test]
    fn test_build_single_payload_valid_address() { 
        let token_address = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"; // WBTC (example)
        let request_id = 1;
        let payload = build_quote_json_rpc_payload(request_id, token_address).unwrap();

        assert_eq!(payload["jsonrpc"], "2.0");
        assert_eq!(payload["id"], request_id);
        assert_eq!(payload["method"], "eth_call");

        let params = payload["params"].as_array().unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[1], "latest");

        let call_object = &params[0];
        assert_eq!(call_object["to"], UNISWAP_V3_QUOTER_V2_ADDRESS);

        let data = call_object["data"].as_str().unwrap();
        assert!(data.starts_with("0x"), "Data should be a hex string starting with 0x");

        // Selector for quoteExactInputSingle(address,address,uint24,uint256,uint160) is 0xcdca1753
        let expected_selector_hex = "cdca1753";
        assert!(
            data.strip_prefix("0x").unwrap().starts_with(expected_selector_hex),
            "Data should start with the correct function selector 0x{}", expected_selector_hex
        );
        
        // Check total length: selector (4 bytes) + 5 parameters * 32 bytes/param = 164 bytes
        // 164 bytes = 328 hex characters. Plus "0x" prefix.
        let hex_data_without_prefix = data.strip_prefix("0x").unwrap();
        assert_eq!(hex_data_without_prefix.len(), 164 * 2, "Encoded data hex length is incorrect");
        assert_eq!(hex::decode(hex_data_without_prefix).unwrap().len(), 164, "Encoded data byte length is incorrect");
    }

    #[test]
    fn test_build_single_payload_invalid_address() { 
        let result = build_quote_json_rpc_payload(1, "invalid-address");
        assert!(matches!(result, Err(UniswapDexError::InvalidInput(_))));
    }

    #[test]
    fn test_build_batch_request_empty() { 
        let result = build_batch_quote_request(&[]);
        assert!(matches!(result, Err(UniswapDexError::InvalidInput(_))));
    }

    #[test]
    fn test_build_batch_request_single() { 
        let token_addresses = [String::from("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599")];
        let batch_payload = build_batch_quote_request(&token_addresses).unwrap();
        let batch_array = batch_payload.as_array().unwrap();
        assert_eq!(batch_array.len(), 1);
        assert_eq!(batch_array[0]["id"], 1);
    }

    #[test]
    fn test_build_batch_request_multiple() { 
        let token_addresses = [
            String::from("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"), // WBTC
            String::from("0x6B175474E89094C44Da98b954EedeAC495271d0F"), // DAI
        ];
        let batch_payload = build_batch_quote_request(&token_addresses).unwrap();
        let batch_array = batch_payload.as_array().unwrap();
        assert_eq!(batch_array.len(), 2);
        assert_eq!(batch_array[0]["id"], 1);
        assert_eq!(batch_array[1]["id"], 2);
        assert_ne!(batch_array[0]["params"][0]["data"], batch_array[1]["params"][0]["data"]);
    }

    #[tokio::test]
    async fn test_execute_batch_quote_request_env_var_missing() {
        let original_key = env::var("INFURA_API_KEY");
        env::remove_var("INFURA_API_KEY");
        let sample_payload = json!([{"id": 1, "jsonrpc": "2.0", "method": "eth_call", "params": [{}]}]);
        let result = execute_batch_quote_request(sample_payload.clone()).await;
        assert!(matches!(result, Err(UniswapDexError::EnvVar(_))));
        if let Ok(key) = original_key {
            env::set_var("INFURA_API_KEY", key);
        }
    }
    
    // Note: More comprehensive tests for execute_batch_quote_request would involve mocking reqwest::Client
    // or using a library like mockito with a configurable RPC_URL in the main function.

    #[test]
    fn test_parse_batch_response_success_ordered() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1, "result": "0x01"}, // 1 Wei
            {"jsonrpc": "2.0", "id": 2, "result": "0x02"}  // 2 Wei
        ]);
        let num_requests = 2;
        let expected = vec![U256::from(1), U256::from(2)];
        assert_eq!(parse_batch_quote_response(response_payload, num_requests).unwrap(), expected);
    }

    #[test]
    fn test_parse_batch_response_success_unordered() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 2, "result": "0x2a"}, // 42 Wei
            {"jsonrpc": "2.0", "id": 1, "result": "0x10"}  // 16 Wei
        ]);
        let num_requests = 2;
        let expected = vec![U256::from(16), U256::from(42)]; // Results should be ordered by ID
        assert_eq!(parse_batch_quote_response(response_payload, num_requests).unwrap(), expected);
    }

    #[test]
    fn test_parse_batch_response_missing_id_in_payload() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1, "result": "0x01"}
            // Missing response for id 2
        ]);
        let num_requests = 2;
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result.unwrap_err().to_string(), "Mismatched response count: expected 2, got 1.");
    }
    
    #[test]
    fn test_parse_batch_response_one_id_missing_from_sequential_ids() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1, "result": "0x01"},
            {"jsonrpc": "2.0", "id": 3, "result": "0x03"} // ID 2 is missing
        ]);
        let num_requests = 3;
        let result = parse_batch_quote_response(response_payload, num_requests);
         assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result.unwrap_err().to_string(), "Mismatched response count: expected 3, got 2.");

         let response_payload_correct_length_missing_id = json!([
            {"jsonrpc": "2.0", "id": 1, "result": "0x01"},
            {"jsonrpc": "2.0", "id": 1, "result": "0x01"}, // Duplicate ID
            {"jsonrpc": "2.0", "id": 3, "result": "0x03"}
        ]);
        let result_dup_id = parse_batch_quote_response(response_payload_correct_length_missing_id, 3);
        assert!(matches!(result_dup_id, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result_dup_id.unwrap_err().to_string(), "Duplicate 'id' 1 found in response batch.");
    }

    #[test]
    fn test_parse_batch_response_rpc_error() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1, "error": {"code": -32000, "message": "Test RPC error"}}
        ]);
        let num_requests = 1;
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result.unwrap_err().to_string(), "RPC error for request id 1: Test RPC error");
    }

    #[test]
    fn test_parse_batch_response_missing_result() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1} // No result or error field
        ]);
        let num_requests = 1;
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result.unwrap_err().to_string(), "Missing 'result' field in response for id 1.");
    }

    #[test]
    fn test_parse_batch_response_invalid_result_type() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1, "result": 123} // Result is not a string
        ]);
        let num_requests = 1;
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert!(result.unwrap_err().to_string().contains("Invalid 'result' type (expected hex string) for id 1:"));
    }

    #[test]
    fn test_parse_batch_response_data_conversion_error() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1, "result": "0xinvalidhex"}
        ]);
        let num_requests = 1;
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::DataConversion(_))));
        assert!(result.unwrap_err().to_string().contains("Failed to parse result hex string '0xinvalidhex' for id 1:"));
    }

    #[test]
    fn test_parse_batch_response_not_an_array() { 
        let response_payload = json!({"error": "not an array"});
        let num_requests = 1;
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result.unwrap_err().to_string(), "Batch response payload is not a JSON array.");
    }
    
    #[test]
    fn test_parse_batch_response_duplicate_id_in_response() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 1, "result": "0x01"},
            {"jsonrpc": "2.0", "id": 1, "result": "0x02"}
        ]);
        let num_requests = 2; // Expecting 2 unique responses
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result.unwrap_err().to_string(), "Duplicate 'id' 1 found in response batch.");
    }

    #[test]
    fn test_parse_batch_response_id_out_of_range() { 
        let response_payload = json!([
            {"jsonrpc": "2.0", "id": 0, "result": "0x01"} // ID 0 is invalid (expect 1-based)
        ]);
        let num_requests = 1;
        let result = parse_batch_quote_response(response_payload, num_requests);
        assert!(matches!(result, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result.unwrap_err().to_string(), "Invalid 'id' 0 found in response object; expected range 1-1.");

        let response_payload_too_high = json!([
            {"jsonrpc": "2.0", "id": 2, "result": "0x01"} // ID 2 is > num_requests (1)
        ]);
        let result_too_high = parse_batch_quote_response(response_payload_too_high, num_requests);
        assert!(matches!(result_too_high, Err(UniswapDexError::ApiError(_))));
        assert_eq!(result_too_high.unwrap_err().to_string(), "Invalid 'id' 2 found in response object; expected range 1-1.");
    }

    #[tokio::test]
    async fn test_get_eth_prices_for_tokens_integration() {
        if env::var("INFURA_API_KEY").is_err() {
            println!("INFURA_API_KEY not set, skipping test_get_eth_prices_for_tokens_integration.");
            return;
        }

        let token_addresses = vec![
            "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(), // USDC
            "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(), // DAI
        ];

        let result = get_eth_prices_for_tokens(&token_addresses).await;

        assert!(
            result.is_ok(),
            "get_eth_prices_for_tokens should return Ok. Error: {:?}",
            result.err()
        );

        let prices = result.unwrap();
        assert_eq!(
            prices.len(),
            token_addresses.len(),
            "Should return the same number of prices as addresses provided."
        );

        for (price_info, input_address) in prices.iter().zip(token_addresses.iter()) {
            assert_eq!(
                &price_info.address, input_address,
                "Returned address should match input address."
            );
            assert!(
                price_info.price_in_eth > 0.0,
                "Price for {} should be a positive value, got {}", input_address, price_info.price_in_eth
            );
            println!(
                "Token: {}, Price in ETH: {}",
                price_info.address, price_info.price_in_eth
            );
        }
    }
}
