//! End-to-end and property-based round-trip tests.

use major_mnemonic_core::{decode, encode};
use proptest::prelude::*;

/// Fixed cases covering the interesting edges.
#[test]
fn roundtrip_fixtures() {
    for input in [
        "0",
        "00",
        "05",
        "007",
        "7",
        "12",
        "314159",
        "314159265358979",
        "9999999999",
        "0000000000",
        "1234567890",
    ] {
        let sentence = encode(input).unwrap();
        assert_eq!(sentence.digit_len, input.len());
        let decoded = decode(&sentence.to_text()).unwrap();
        assert_eq!(decoded, input, "round trip failed for {input:?}");
    }
}

proptest! {
    /// The core guarantee: any digit string round-trips exactly through the
    /// canonical (pretty) rendering, including leading zeros and odd lengths.
    #[test]
    fn decode_encode_is_identity(s in "[0-9]{1,40}") {
        let sentence = encode(&s).unwrap();
        let decoded = decode(&sentence.to_text()).unwrap();
        prop_assert_eq!(decoded, s);
    }

    /// The plain (lowercase, no punctuation) rendering also round-trips.
    #[test]
    fn plain_rendering_roundtrips(s in "[0-9]{1,40}") {
        let sentence = encode(&s).unwrap();
        let decoded = decode(&sentence.to_plain()).unwrap();
        prop_assert_eq!(decoded, s);
    }
}
