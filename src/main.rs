extern crate rs_poker;

mod database;
mod constants;
mod game;

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::VecDeque;

use game::sample_hand_strength;
use crate::constants::UPDATE_BATCH_SIZE;
use crate::database::{create_session, retrieve_batch, update_batch, DatabasePokerHand};
use crate::game::get_hand_from_cards_id;

#[tokio::main]
async fn main() {
    let session = create_session().await;

    let results: Arc<Mutex<VecDeque<DatabasePokerHand>>> = Arc::new(Mutex::new(VecDeque::new()));
    let total_batches: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    // Step 1: Set up a Tokio channel
    let (tx, mut rx) = mpsc::channel(32); // Adjust the channel size as needed

    // Step 2: Spawn an async listener task
    tokio::spawn(async move {
        while let Some(batch) = rx.recv().await {
            let session = create_session().await;
            // Async database operation
            update_batch(&session, batch).await;
        }
    });

    let mut rows: Vec<DatabasePokerHand> = retrieve_batch(&session, None).await;
    while rows.len() > 0 {
        // Use Rayon's parallel iterator to process each row in parallel
        rows.par_iter().for_each(|row| {
            let hand = get_hand_from_cards_id(&row.cards_id);
            let hand_histogram = sample_hand_strength(hand, 1000);
    
            // Acquire lock to update shared state
            let mut results_guard = results.lock().unwrap();
            results_guard.push_back(
                DatabasePokerHand { cards_id: row.cards_id.clone(), histogram: Some(hand_histogram.clone()), token: None }
            );
    
            // Check if we need to push to database
            if results_guard.len() >= UPDATE_BATCH_SIZE {
                let mut total_batches_guard = total_batches.lock().unwrap();
                // Assuming we can drain the VecDeque to exactly 900 elements for the database push
                let results_to_update = results_guard.drain(..UPDATE_BATCH_SIZE).collect::<Vec<_>>();
                drop(results_guard); // Explicitly drop the lock before the blocking operation
                let _ = tx.try_send(results_to_update);
    
                *total_batches_guard += 1;
                println!("Generated and updated histograms for batch #{}", total_batches_guard);
            }
    
        });

        rows = retrieve_batch(&session, Some(rows[rows.len()-1].token.unwrap())).await;
    }

    let mut remaining_results = results.lock().unwrap();
    if !remaining_results.is_empty() {
        let results_to_update = remaining_results.drain(..).collect::<Vec<_>>();
        update_batch(&session, results_to_update).await;
    }
}