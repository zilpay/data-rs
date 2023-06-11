use crate::config::zilliqa::{CHARSET, HRP};
use sha2::{Digest, Sha256};
use std::io::{Error, ErrorKind};

pub const GENERATOR: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];

pub fn polymod(values: &[u8]) -> u32 {
    let mut chk: u32 = 1;

    for p in values {
        let top = chk >> 25;

        chk = ((chk & 0x1ffffff) << 5) ^ (*p as u32);

        for i in 0..5 {
            if ((top >> i) & 1) == 1 {
                chk ^= GENERATOR[i];
            }
        }
    }

    chk
}

pub fn gen_limited_vec(start: u64, end: u64, limit: usize) -> Vec<u64> {
    let mut list = Vec::with_capacity(limit);

    for i in start..end {
        if list.len() >= limit {
            break;
        }

        list.push(i);
    }

    list
}

pub fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut ret = Vec::new();

    for p in 0..hrp.len() {
        ret.push(hrp.as_bytes()[p] >> 5);
    }

    ret.push(0);

    for p in 0..hrp.len() {
        ret.push(hrp.as_bytes()[p] & 31);
    }

    ret
}

pub fn create_checksum(hrp: &str, data: &Vec<u8>) -> Vec<u8> {
    let mut values: Vec<u8> = Vec::new();

    values.extend(hrp_expand(hrp));
    values.extend(data);
    values.extend(vec![0; 6]);

    let polymod = polymod(&values) ^ 1;
    let mut ret = Vec::new();

    for p in 0..6 {
        ret.push(((polymod >> (5 * (5 - p))) & 31) as u8);
    }

    ret
}

pub fn verify_checksum(hrp: &str, data: &Vec<u8>) -> bool {
    let values = [&hrp_expand(hrp)[..], data].concat();

    polymod(&values) == 1
}

pub fn decode(bech_string: &str) -> Option<(String, Vec<u8>)> {
    let mut has_lower = false;
    let mut has_upper = false;

    for c in bech_string.chars() {
        let code = c as u32;
        if code < 33 || code > 126 {
            return None;
        }
        if code >= 97 && code <= 122 {
            has_lower = true;
        }
        if code >= 65 && code <= 90 {
            has_upper = true;
        }
    }

    if has_lower && has_upper {
        return None;
    }

    let bech_string = bech_string.to_lowercase();
    let pos = bech_string.rfind('1').unwrap_or(0);

    if pos < 1 || pos + 7 > bech_string.len() || bech_string.len() > 90 {
        return None;
    }

    let hrp = bech_string[..pos].to_string();
    let mut data = Vec::new();

    for c in bech_string[pos + 1..].chars() {
        let d = CHARSET.find(c).unwrap_or(0);
        data.push(d as u8);
    }

    if !verify_checksum(&hrp, &data) {
        return None;
    }

    Some((hrp, data[..data.len() - 6].to_vec()))
}

pub fn encode(hrp: &str, data: &Vec<u8>) -> String {
    let checksum = create_checksum(hrp, data);
    let combined = [&data[..], &checksum[..]].concat();
    let mut ret = String::from(hrp) + "1"; // hrp is zil so it is zil1.

    for p in 0..combined.len() {
        let idx = combined[p] as usize;
        let value = CHARSET.chars().nth(idx);

        match value {
            Some(v) => ret.push(v),
            None => continue,
        }
    }

    ret
}

pub fn get_address_from_public_key(public_key: &str) -> Result<String, Error> {
    let normalized = match hex::decode(public_key.to_lowercase().replace("0x", "")) {
        Ok(h) => h,
        Err(_) => {
            let pub_key_err = Error::new(ErrorKind::Other, "Invalid pub_key");

            return Err(pub_key_err);
        }
    };
    let mut hasher = Sha256::new();

    hasher.update(normalized);

    let hash_result = hasher.finalize();
    let hex_string = hex::encode(hash_result);
    let sliced_hex = &hex_string[24..];

    Ok(sliced_hex.to_string())
}

pub fn convert_bits(data: &Vec<u8>, from_width: u32, to_width: u32, pad: bool) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut ret = Vec::new();
    let maxv = (1 << to_width) - 1;

    assert!(from_width < u32::MAX);
    assert!(to_width < u32::MAX);

    for value in data {
        if (*value as u32) >> from_width != 0 {
            return None;
        }

        acc = (acc << from_width) | (*value as u32);
        bits += from_width;

        while bits >= to_width {
            bits -= to_width;
            ret.push(((acc >> bits) & maxv) as u8);
        }
    }

    if pad {
        if bits > 0 {
            ret.push(((acc << (to_width - bits)) & maxv) as u8);
        }
    } else if bits >= from_width || (acc << (to_width - bits)) & maxv != 0 {
        return None;
    }

    Some(ret)
}

pub fn from_bech32_address(address: &str) -> Option<Vec<u8>> {
    let (hrp, data) = match decode(address) {
        Some(addr) => addr,
        None => return None,
    };

    if hrp != HRP {
        return None;
    }

    let buf = match convert_bits(&data, 5, 8, false) {
        Some(buf) => buf,
        None => return None,
    };

    Some(buf)
}

#[test]
fn test_polymod() {
    let bytes: [u8; 16] = [
        65, 29, 177, 250, 15, 49, 136, 8, 34, 192, 119, 116, 123, 146, 130, 62,
    ];
    let res = polymod(&bytes);

    assert_eq!(98216235, res);
}

#[test]
fn test_addr_from_pub_key() {
    let public_key = "0x0308518cf944ece57f0bedc155deb093e1fb8f73aadbd025687a0409cae9ed19b1";
    let addr = get_address_from_public_key(public_key).unwrap();

    assert_eq!(addr, "8885906da076a450138ff794796530a34b958b91");
}

#[test]
fn test_hrp_expand() {
    let test_str = "test";
    let res = hrp_expand(test_str);
    let should: Vec<u8> = vec![3, 3, 3, 3, 0, 20, 5, 19, 20];

    assert_eq!(should, res);
}

#[test]
fn test_create_checksum() {
    let hrp = "test";
    let data: Vec<u8> = vec![255, 64, 0, 0, 0, 2];
    let res = create_checksum(hrp, &data);
    let should: Vec<u8> = vec![2, 14, 10, 20, 25, 19];

    assert_eq!(res, should);
}

#[test]
fn test_encode() {
    let hrp = "test";
    let data = vec![128, 0, 64, 32];
    let res = encode(hrp, &data);
    let should = "test1qep0uve";

    assert_eq!(should, res);
}

#[test]
fn test_gen_limited_vec() {
    let start = 200;
    let end = 500;
    let limit = 20;
    let result = gen_limited_vec(start, end, limit);
    let should_be = vec![
        200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217,
        218, 219,
    ];

    assert_eq!(result, should_be);
}

#[test]
fn test_convert_bits() {
    let byte_vec = hex::decode("7793a8e8c09d189d4d421ce5bc5b3674656c5ac1").unwrap();
    let addr_bz = convert_bits(&byte_vec, 8, 5, true).unwrap();
    let shoud = "0e1e091a111a060013140c091a130a020313121b181619160e11121618161601";

    assert_eq!(hex::encode(addr_bz), shoud);
}

#[test]
fn test_decode() {
    let bech32 = "zil1w7f636xqn5vf6n2zrnjmckekw3jkckkpyrd6z8";
    let (hrp, data) = decode(bech32).unwrap();

    assert_eq!(hrp, "zil");
    assert_eq!(
        hex::encode(data),
        "0e1e091a111a060013140c091a130a020313121b181619160e11121618161601"
    );
}

#[test]
fn test_from_bech32_address() {
    let bech32 = "zil1w7f636xqn5vf6n2zrnjmckekw3jkckkpyrd6z8";
    let base16_buff = from_bech32_address(bech32).unwrap();
    let base16 = hex::encode(base16_buff);

    assert_eq!(base16, "7793a8e8c09d189d4d421ce5bc5b3674656c5ac1");
}
