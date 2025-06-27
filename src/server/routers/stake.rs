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

const PROTO_MAINNET_POOLS: [EvmPool; 15] = [
    EvmPool {
        address: "0xA0572935d53e14C73eBb3de58d319A9Fe51E1FC8",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "Moonlet",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "ZIL",
    },
    EvmPool {
        address: "0x2Abed3a598CBDd8BB9089c09A9202FD80C55Df8c",
        token_address: "0xD8B61fed51b9037A31C2Bf0a5dA4B717AF0C0F78",
        name: "AtomicWallet",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "SHARK",
    },
    EvmPool {
        address: "0xB9d689c64b969ad9eDd1EDDb50be42E217567fd3",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "CEX.IO",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "ZIL",
    },
    EvmPool {
        address: "0xe0C095DBE85a8ca75de4749B5AEe0D18100a3C39",
        token_address: "0x7B213b5AEB896bC290F0cD8B8720eaF427098186",
        name: "PlunderSwap",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "pZIL",
    },
    EvmPool {
        address: "0xC0247d13323F1D06b6f24350Eea03c5e0Fbf65ed",
        token_address: "0x2c51C97b22E73AfD33911397A20Aa5176e7Ab951",
        name: "Luganodes",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "LNZIL",
    },
    EvmPool {
        address: "0x8A0dEd57ABd3bc50A600c94aCbEcEf62db5f4D32",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "DTEAM",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "ZIL",
    },
    EvmPool {
        address: "0x3b1Cd55f995a9A8A634fc1A3cEB101e2baA636fc",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "Shardpool",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "ZIL",
    },
    EvmPool {
        address: "0x66a2bb4AD6999966616B2ad209833260F8eA07C8",
        token_address: "0xA1Adc08C12c684AdB28B963f251d6cB1C6a9c0c1",
        name: "Encapsulate",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "encapZIL",
    },
    EvmPool {
        address: "0xe59D98b887e6D40F52f7Cc8d5fb4CF0F9Ed7C98B",
        token_address: "0xf564DF9BeB417FB50b38A58334CA7607B36D3BFb",
        name: "Amazing Pool - Avely and ZilPay",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "stZIL",
    },
    EvmPool {
        address: "0xd090424684a9108229b830437b490363eB250A58",
        token_address: "0xE10575244f8E8735d71ed00287e9d1403f03C960",
        name: "PathrockNetwork",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "zLST",
    },
    EvmPool {
        address: "0x33cDb55D7fD68d0Da1a3448F11bCdA5fDE3426B3",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "BlackNodes",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "ZIL",
    },
    EvmPool {
        address: "0x35118Af4Fc43Ce58CEcBC6Eeb21D0C1Eb7E28Bd3",
        token_address: "0x245E6AB0d092672B18F27025385f98E2EC3a3275",
        name: "Lithium Digital",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "litZil",
    },
    EvmPool {
        address: "0x62269F615E1a3E36f96dcB7fDDF8B823737DD618",
        token_address: "0x770a35A5A95c2107860E9F74c1845e20289cbfe6",
        name: "TorchWallet.io",
        pool_type: StakingPoolType::LIQUID,
        token_decimals: 18,
        token_symbol: "tZIL",
    },
    EvmPool {
        address: "0xa45114E92E26B978F0B37cF19E66634f997250f9",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "Stakefish",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "ZIL",
    },
    EvmPool {
        address: "0x02376bA9e0f98439eA9F76A582FBb5d20E298177",
        token_address: "0x0000000000000000000000000000000000000000",
        name: "AlphaZIL (former Ezil)",
        pool_type: StakingPoolType::NORMAL,
        token_decimals: 18,
        token_symbol: "ZIL",
    },
];

pub async fn handle_get_pools(
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // Просто сериализуем статический вектор в JSON
    let json = serde_json::to_string(&PROTO_MAINNET_POOLS).unwrap_or_else(|e| {
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
