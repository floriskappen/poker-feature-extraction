use rs_poker::core::{Card, Deck, Hand, Rankable, Suit, Value};
use rand::seq::SliceRandom;
use itertools::Itertools;

use crate::encode::decode_cards;

// Returns the character representation of a card's rank.
pub const RANK_TO_CHAR: &[char] = &['2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', 'A'];

// Returns the character representation of a card's suit.
pub const SUIT_TO_CHAR: &[char] = &['s', 'h', 'd', 'c'];

const SUIT_TO_RS_POKER_SUIT: &[u8] = &[0, 2, 3, 1];

// Extracts the suit from a card value.
pub fn deck_get_suit(card: u8) -> u8 {
    card & 3
}

// Extracts the rank from a card value.
pub fn deck_get_rank(card: u8) -> u8 {
    card >> 2
}

pub fn get_hand_from_cards_id(cards_id: i64) -> Hand {
    let card_numbers: Vec<u8> = decode_cards(cards_id);

    return Hand::new_with_cards(
        card_numbers.iter()
            .map(|card_number| {
                return Card::new(
                    Value::from_u8(deck_get_rank(card_number.clone())),
                    Suit::from_u8(
                        SUIT_TO_RS_POKER_SUIT[deck_get_suit(card_number.clone()) as usize]
                    )
                )
            })
            .collect()
    );
}


pub fn sample_hand_strength(canonical_hand: Hand, trials: usize) -> Vec<u8> {
    let mut histogram = vec![0.0; 30];
    let mut deck = Deck::default();

    for card in canonical_hand.iter() {
        deck.remove(card);
    }
    let mut remaining_deck: Vec<&Card> = deck.iter().collect();

    let community_cards_known: Vec<Card> = canonical_hand.cards()[2..].to_vec();
    let player_hole_cards: Vec<Card> = canonical_hand.cards().iter().take(2).cloned().collect_vec();
    let number_of_community_cards_to_draw = 5 - community_cards_known.len();

    for _ in 0..trials {
        // Shuffle the remaining deck and draw the rest of the community cards
        remaining_deck.shuffle(&mut rand::thread_rng());

        let community_sample: Vec<Card> = remaining_deck.iter()
            .take(number_of_community_cards_to_draw)
            .cloned()
            .map(|&card| card)
            .collect();
        let community_cards: Vec<Card> = community_cards_known.iter()
            .map(|&card| card)
            .chain(community_sample.iter().cloned())
            .collect();

        let player_hand = Hand::new_with_cards(
            player_hole_cards.iter().chain(community_cards.iter()).cloned().collect()
        );
        let player_score = player_hand.rank();

        let mut opponents_beaten: u32 = 0;
        let mut total_opponent_hands: u32 = 0;
        let remaining_deck_after_community = remaining_deck[number_of_community_cards_to_draw..].to_vec();

        for opponent_cards in remaining_deck_after_community.iter().combinations(2) {
            let dereferenced_opponent_cards: Vec<Card> = opponent_cards.into_iter()
                .map(|&&card| card.clone())
                .collect();

            // Now, chain `dereferenced_opponent_cards` with `community_cards` correctly
            let vec3: Vec<Card> = dereferenced_opponent_cards.into_iter()
                .chain(community_cards.iter().cloned())
                .collect();

            let opponent_hand = Hand::new_with_cards(vec3);
            let opponent_score = opponent_hand.rank();

            if player_score > opponent_score {
                opponents_beaten += 2;
            } else if player_score == opponent_score {
                opponents_beaten += 1;
            }

            total_opponent_hands += 1;
        }

        let hand_strength: f32 = opponents_beaten as f32 / (total_opponent_hands * 2) as f32;
        let bin_index = (hand_strength * (histogram.len() - 1) as f32) as usize;
        histogram[bin_index] += 1.0;
    }

    // Round them so they fit in cassandra's tinyint value
    // TODO: Maybe see if we can give them some more resulution as the tinyint can be -128 to 127
    let histogram: Vec<u8> = histogram.iter().map(|&bin| ((bin / trials as f32) * 100.0) as u8).collect();
    // println!("{:?}", histogram);
    return histogram;
}
