use crate::config::zilliqa::CHARSET;

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

#[test]
fn test_polymod() {
    let bytes: [u8; 16] = [
        65, 29, 177, 250, 15, 49, 136, 8, 34, 192, 119, 116, 123, 146, 130, 62,
    ];
    let res = polymod(&bytes);

    assert_eq!(98216235, res);
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
