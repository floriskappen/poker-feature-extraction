extern crate rs_poker;

mod database;
mod constants;
mod game;

use game::sample_hand_strength;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rs_poker::core::Hand;
use rs_poker::holdem::MonteCarloGame;

use crate::database::{create_session, retrieve_batch, DatabasePokerHand};
use crate::game::get_hand_from_cards_id;

const GAMES_COUNT: i32 = 3_000_000;
const STARTING_HANDS: [&str; 2] = ["AdAh", "3c2s"];


#[tokio::main]
async fn main() {
    let session = create_session().await;

    let rows: Vec<DatabasePokerHand> = retrieve_batch(&session, None).await;

    // for row in rows {
    //     let hand = get_hand_from_cards_id(row.cards_id);

    //     // let histogram: Vec<u8> = sample_hand_str

    //     sample_hand_strength(hand, 1000);
    // }

    // Use Rayon's parallel iterator to process each row in parallel
    rows.par_iter().for_each(|row| {
        let hand = get_hand_from_cards_id(&row.cards_id);

        // Assuming sample_hand_strength() is CPU-bound and can be called in parallel
        sample_hand_strength(hand, 1000);
    });

    // let hands = STARTING_HANDS
    //     .iter()
    //     .map(|s| Hand::new_from_str(s).expect("Should be able to create a hand."))
    //     .collect();
    // let mut g = MonteCarloGame::new(hands).expect("Should be able to create a game.");
    // let mut wins: [u64; 2] = [0, 0];
    // for _ in 0..GAMES_COUNT {
    //     let r = g.simulate();
    //     g.reset();
    //     wins[r.0.ones().next().unwrap()] += 1
    // }

    // let normalized: Vec<f64> = wins
    //     .iter()
    //     .map(|cnt| *cnt as f64 / GAMES_COUNT as f64)
    //     .collect();

    // println!("Starting Hands =\t{:?}", STARTING_HANDS);
    // println!("Wins =\t\t\t{:?}", wins);
    // println!("Normalized Wins =\t{:?}", normalized);
}