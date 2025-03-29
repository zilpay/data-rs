use crate::config::zilliqa::PROVIDERS;
use log::{error, warn};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Error as ReqwestError};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, Map, Value};
use std::io::{Error, ErrorKind};
use thiserror::Error;

const MAX_BATCH_SIZE: usize = 2;

#[derive(Serialize, Debug, Clone)]
pub struct JsonBodyReq {
    pub id: String,
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

#[derive(Deserialize, Debug)]
pub struct JsonBodyRes<T> {
    pub result: Option<T>,
    pub error: Option<Map<String, Value>>,
}

impl<T> JsonBodyRes<T> {
    pub fn get_result(&self) -> Result<&T, ZilliqaError> {
        match (&self.result, &self.error) {
            (Some(result), _) => Ok(result),
            (None, Some(err)) => {
                let err_msg = format!("RPC error: {:?}", err);
                error!("{}", err_msg);
                Err(ZilliqaError::RpcError(err_msg))
            }
            (None, None) => Err(ZilliqaError::EmptyResponse),
        }
    }
}

#[derive(Error, Debug)]
pub enum ZilliqaError {
    #[error("All providers are down or unreachable")]
    AllProvidersDown,

    #[error("RPC returned error: {0}")]
    RpcError(String),

    #[error("Empty response with no result or error")]
    EmptyResponse,

    #[error("Request error: {0}")]
    RequestError(#[from] ReqwestError),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] Error),
}

impl From<ZilliqaError> for Error {
    fn from(err: ZilliqaError) -> Self {
        Error::new(ErrorKind::Other, err.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Zilliqa {
    providers: Vec<String>,
    client: Client,
}

impl Zilliqa {
    pub fn new() -> Self {
        let providers = PROVIDERS.iter().map(ToString::to_string).collect();

        Zilliqa {
            providers,
            client: Client::new(),
        }
    }

    pub fn from(providers: Vec<String>) -> Self {
        Zilliqa {
            providers,
            client: Client::new(),
        }
    }

    pub fn extend_providers(&mut self, urls: Vec<String>) {
        self.providers.extend(urls);
    }

    pub async fn fetch<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        bodies: Vec<JsonBodyReq>,
    ) -> Result<Vec<JsonBodyRes<T>>, ZilliqaError> {
        if self.providers.is_empty() {
            return Err(ZilliqaError::AllProvidersDown);
        }

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let mut all_results = Vec::with_capacity(bodies.len());

        for chunk in bodies.chunks(MAX_BATCH_SIZE) {
            let mut provider_success = false;

            for provider in &self.providers {
                let response_result = self
                    .try_fetch_from_provider::<Vec<JsonBodyRes<T>>>(provider, chunk, &headers)
                    .await;

                match response_result {
                    Ok(responses) => {
                        for (i, response) in responses.iter().enumerate() {
                            if let Some(err) = &response.error {
                                let method = chunk.get(i).map_or("unknown", |b| &b.method);
                                error!("RPC error for method {}: {:?}", method, err);
                            }
                        }

                        all_results.extend(responses);
                        provider_success = true;
                        break;
                    }
                    Err(e) => {
                        warn!("Provider {} failed: {}", provider, e);
                        continue;
                    }
                }
            }

            if !provider_success {
                warn!("All providers failed for this batch");
            }
        }

        if all_results.is_empty() {
            return Err(ZilliqaError::AllProvidersDown);
        }

        Ok(all_results)
    }

    async fn try_fetch_from_provider<R: DeserializeOwned>(
        &self,
        provider: &str,
        bodies: &[JsonBodyReq],
        headers: &HeaderMap,
    ) -> Result<R, ZilliqaError> {
        let response = self
            .client
            .post(provider)
            .headers(headers.clone())
            .json(bodies)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let text = response.text().await?;
            let error_msg = format!("HTTP error {}: {}", status_code, text);
            error!("{}", error_msg);
            return Err(ZilliqaError::RpcError(error_msg));
        }

        let text = response.text().await?;

        match from_str::<R>(&text) {
            Ok(res) => Ok(res),
            Err(e) => {
                error!(
                    "Failed to parse JSON response: {}\nResponse text: {}",
                    e, text
                );
                Err(ZilliqaError::JsonError(e))
            }
        }
    }

    pub fn build_body(&self, method: &str, params: Value) -> JsonBodyReq {
        JsonBodyReq {
            id: "1".to_string(),
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }

    pub async fn fetch_single<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<JsonBodyRes<T>, ZilliqaError> {
        let body = self.build_body(method, params);
        let results = self.fetch::<T>(vec![body]).await?;
        results
            .into_iter()
            .next()
            .ok_or(ZilliqaError::EmptyResponse)
    }
}
