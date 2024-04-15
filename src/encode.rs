const BASE_CARD: i64 = 53; // Max card + 2
const BASE_HAND_STRENGTH: i64 = 102;
use cdrs_tokio::types::blob::Blob;
use num_bigint::{BigUint, ToBigUint};

pub fn encode_cards(cards: &Vec<u8>) -> i64 {
    let mut encoded_cards: i64 = 0;
    for &card in cards {
        encoded_cards = encoded_cards * BASE_CARD + (card + 1) as i64;
    }

    return encoded_cards;
}

pub fn decode_cards(encoded_cards: i64) -> Vec<u8> {
    let mut cards: Vec<u8> = vec![];
    let mut encoded_value = encoded_cards;
    while encoded_value > 0 {
        cards.push(((encoded_value % BASE_CARD) as u8) - 1);
        encoded_value /= BASE_CARD;
    }
    cards.reverse();
    return cards;
}

pub fn hand_strength_histogram_into_blob(hand_strength_histogram: &Vec<i8>) -> Blob {
    let base: BigUint = ToBigUint::to_biguint(&BASE_HAND_STRENGTH).unwrap();
    let mut encoded_hsh: BigUint = ToBigUint::to_biguint(&0).unwrap();
    for (i, bucket) in hand_strength_histogram.iter().enumerate() {
        encoded_hsh = encoded_hsh + ToBigUint::to_biguint(&(bucket + 1)).unwrap() * ToBigUint::to_biguint(&base.pow(i as u32)).unwrap()
    }

    let bytes = encoded_hsh.to_bytes_le();

    let blob = Blob::from(bytes);

    return blob;
}

pub fn hand_strength_histogram_from_blob(hand_strength_histogram_blob: Blob) -> Vec<i8> {
    let bytes = hand_strength_histogram_blob.into_vec();

    let mut encoded_hsh = BigUint::from_bytes_le(&bytes);

    let base: BigUint = ToBigUint::to_biguint(&BASE_HAND_STRENGTH).unwrap();
    let zero = ToBigUint::to_biguint(&0).unwrap();

    // Initialize an empty vector to store the decoded histogram
    let mut hand_strength_histogram: Vec<i8> = Vec::new();

    while encoded_hsh > zero {
        // Extract the least significant digit
        let bucket = (&encoded_hsh % &base).to_u32_digits()[0] as i8;
        // Add the bucket value to the histogram
        hand_strength_histogram.push(bucket - 1);
        // Divide the encoded value by the base
        encoded_hsh /= &base;
    }

    // Fill in any missing values (if any)
    while hand_strength_histogram.len() < 30 {
        hand_strength_histogram.push(0);
    }

    return hand_strength_histogram;

}

