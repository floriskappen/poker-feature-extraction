mod game;
mod encode;
mod load;
mod logger;

use dotenv::dotenv;
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use game::sample_hand_strength;
use crate::game::get_hand_from_cards_id;
use crate::encode::encode_hand_strength_histogram;
use crate::load::HandLoader;
use crate::logger::init_logger;

static EXPORT_PATH: &str = "./exports";

fn save_hand_strength_histograms_to_file(hand_strength_histograms: &Vec<Vec<u8>>, round: usize, batch: usize) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = format!("{}/round_{}_batch_{}.bin", EXPORT_PATH, round, batch);

    let file = File::create(filepath)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, hand_strength_histograms)?;
    Ok(())
}

fn main() {
    init_logger().expect("Failed to initialize logger");
    dotenv().ok();
    
    let round = 0;
    let mut hand_loader = HandLoader::new(round).expect("Failed to initialize HandLoader");

    log::info!("Initialized HandLoader with round {} and batch 0/{}", round, hand_loader.total_batches-1);

    let results: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));

    let trials = if round == 0 || round == 1 {
        1000
    } else if round == 2 {
        700
    } else {
        400
    };

    for batch in 0..hand_loader.total_batches {
        let hands = &hand_loader.current_batch_hands;

        hands.par_iter().for_each(|hand| {
            let hand = get_hand_from_cards_id(hand.clone());
            let hand_histogram = sample_hand_strength(hand, trials);
            let encoded_hand_strength_histogram = encode_hand_strength_histogram(&hand_histogram);
            let encoded_hand_strength_histogram_bytes = encoded_hand_strength_histogram.to_bytes_le();

            let mut results_guard = results.lock().unwrap();
            results_guard.push(encoded_hand_strength_histogram_bytes);
            if results_guard.len() > 0 && results_guard.len() % 1000 == 0 {
                log::info!(
                    "Calculated HSH for round {} batch {}/{} hand {}/{}",
                    round,
                    batch,
                    hand_loader.total_batches-1,
                    results_guard.len(),
                    hands.len()
                )
            }
        });

        let mut results_guard = results.lock().unwrap();
        log::info!(
            "Calculated HSH for round {} batch {}/{} hand {}/{}",
            round,
            batch,
            hand_loader.total_batches-1,
            results_guard.len(),
            hands.len()
        );

        save_hand_strength_histograms_to_file(&results_guard, round, batch)
            .expect(format!("ERROR: Failed to save HSH for round {} batch #{}", round, batch).as_str());
        results_guard.clear();

        log::info!("Saved encoded HSHs for round {} batch {}/{}", round, batch, hand_loader.total_batches-1);

        if batch < hand_loader.total_batches - 1 {
            hand_loader.load_next_batch();
            log::info!("Loaded HandLoader with round {} and batch {}/{}", round, batch + 1, hand_loader.total_batches-1);
        }

    }
}
