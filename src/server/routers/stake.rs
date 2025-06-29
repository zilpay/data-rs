use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    header::{self, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN},
    http::HeaderValue,
    Request, Response,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
enum StakingPoolType {
    LIQUID,
    NORMAL,
}

#[derive(Debug, Serialize)]
struct EvmPool<'a> {
    address: &'a str,
    token_address: &'a str,
    name: &'a str,
    pool_type: StakingPoolType,
    token_decimals: u8,
    token_symbol: &'a str,
}

const MAINNET_POOLS: [EvmPool; 12] = [
    EvmPool {
        address: "0x1f0e86Bc299Cc66df2e5512a7786C3F528C0b5b6",
        token_address: "0x8a2afD8Fe79F8C694210eB71f4d726Fc8cAFdB31",
        name: "Amazing Pool (Avely)",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "aZIL",
    },
    EvmPool {
        address: "0xCDb0B23Db1439b28689844FD093C478d73C0786A",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "2ZilMoon (Make Zilliqa Great Again)",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "2ZIL",
    },
    EvmPool {
        address: "0x82245678902345678902345678918278372382",
        token_address: "0x1234567890234567890234567890234567231",
        name: "Plunderswap",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "plZIL",
    },
    EvmPool {
        address: "0x96525678902345678902345678918278372212",
        token_address: "0x1234567890234567890234567890234567232",
        name: "IgniteDao",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "igZIL",
    },
    EvmPool {
        address: "0x965256789023456789023456789182783K92Uh",
        token_address: "0x1234567890234567890234567890234567234",
        name: "ADAMine",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "adaZIL",
    },
    EvmPool {
        address: "0xe863906941de820bde06701a0d804dd0b8575d67",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "K23k2322",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "kZIL",
    },
    EvmPool {
        address: "0x7A28eda6899d816e574f7dFB62Cc8A84A4fF92a6",
        token_address: "0x3fE49722fC4F9F119AB18fE0CF7D340A23C8388b",
        name: "Validator 1",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "LST1",
    },
    EvmPool {
        address: "0x62f3FC68ba2Ff62b23E73c48010262aD64054032",
        token_address: "0x7854BFB32CC7a377165Ee3B5C8103a80A07913B2",
        name: "Validator 2",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "LST2",
    },
    EvmPool {
        address: "0x7a0b7e6d24ede78260c9ddbd98e828b0e11a8ea2",
        token_address: "0x9e5c257D1c6dF74EaA54e58CdccaCb924669dc83",
        name: "Collective",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "xZIL",
    },
    EvmPool {
        address: "0x7e02c204daf4e1140a331d6dfad1eeb265d9544f",
        token_address: "0xDbdb7f1f01c438f9951d780Ac9C42E9795Bb938f",
        name: "Quantum",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "yZIL",
    },
    EvmPool {
        address: "0x983fC5214be8fB08A205902ea73A2cA10811060c",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "Citadel",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "cZIL",
    },
    EvmPool {
        address: "0xA0572935d53e14C73eBb3de58d319A9Fe51E1FC8",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "Moonlet",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "mZIL",
    },
];

pub async fn handle_get_pools(
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // Просто сериализуем статический вектор в JSON
    let json = serde_json::to_string(&MAINNET_POOLS).unwrap_or_else(|e| {
        // В случае ошибки сериализации возвращаем пустой массив
        eprintln!("Error serializing pools: {}", e);
        "[]".to_string()
    });

    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap();

    response
        .headers_mut()
        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    response.headers_mut().insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET"),
    );

    Ok(response)
}
