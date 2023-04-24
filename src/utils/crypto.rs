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

#[test]
fn test_polymod() {
    let bytes: [u8; 16] = [
        65, 29, 177, 250, 15, 49, 136, 8, 34, 192, 119, 116, 123, 146, 130, 62,
    ];
    let res = polymod(&bytes);

    assert_eq!(98216235, res);
}
