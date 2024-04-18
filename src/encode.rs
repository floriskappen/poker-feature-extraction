const BASE_CARD: i64 = 53; // Max card + 2
const BASE_HAND_STRENGTH: u64 = 102;

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
