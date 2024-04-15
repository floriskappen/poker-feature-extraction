const BASE_CARD: i64 = 53; // Max card + 2
const BASE_HAND_STRENGTH: u64 = 102;
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

pub fn encode_hand_strength_histogram(hand_strength_histogram: &Vec<u8>) -> BigUint {
    let base: BigUint = ToBigUint::to_biguint(&BASE_HAND_STRENGTH).unwrap();
    let mut encoded_hsh: BigUint = ToBigUint::to_biguint(&0).unwrap();
    for (i, bucket) in hand_strength_histogram.iter().enumerate() {
        encoded_hsh = encoded_hsh + ToBigUint::to_biguint(&(bucket + 1)).unwrap() * ToBigUint::to_biguint(&base.pow(i as u32)).unwrap()
    }

    return encoded_hsh
}
