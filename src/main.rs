use dotenv::dotenv;
use futures::future::join_all;

use tokio::task;
use anyhow::{Result, Context, Error};
use std::error::Error as StdError;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use cdrs_tokio::types::prelude::Error as crdsError;


extern crate rs_poker;

mod database;
mod constants;
mod game;
mod encode;

use game::sample_hand_strength;
use crate::constants::UPDATE_BATCH_SIZE;
use crate::database::{create_session, create_session_with_retry, retrieve_batch, update_batch, DatabasePokerHand};
use crate::game::get_hand_from_cards_id;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Step 1: Set up a Tokio channel
    let (tx, mut rx) = mpsc::channel::<Vec<DatabasePokerHand>>(32); // Adjust the channel size as needed
    
    // Step 2: Spawn an async listener task
    tokio::spawn(async move {
        while let Some(batch) = rx.recv().await {
            let mut session = create_session_with_retry().await;
            let chunks = batch.chunks(UPDATE_BATCH_SIZE).collect::<Vec<_>>();
    
            for db_batch in chunks {
                let db_batch = db_batch.to_vec();
                loop {
                    match update_batch(&session, db_batch.clone()).await {
                        Ok(_) => {
                            println!("UPSERTED BATCH");
                            break;
                        }
                        Err(e) => {
                            eprintln!("General error: {:?}", e);
                            println!("Retry after 5s due to error...");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            session = create_session_with_retry().await;
                        }
                    }
                }
            }
        }
    });

    let mut session = create_session_with_retry().await;

    // let starting_value: i64 = 9213042079234272081;
    // let mut rows: Vec<DatabasePokerHand> = retrieve_batch(&session, Some(starting_value)).await;

    let mut rows: Vec<DatabasePokerHand> = vec![];
    loop {
        match retrieve_batch(&session, None).await {
            Ok(result_rows) => {
                rows = result_rows;
                break;
            }

            Err(crdsError::Io(_)) => {
                println!("Retry after 5s due to Io (probably connection reset) error...");
                tokio::time::sleep(Duration::from_secs(5)).await;
                session = create_session_with_retry().await;
            }
            Err(err) => {
                // Handle other errors
                eprintln!("Error: {:?}", err);
                println!("Retry after 5s due to error...");
                tokio::time::sleep(Duration::from_secs(5)).await;
                session = create_session_with_retry().await;
            }
        }
    }

    let results: Arc<Mutex<Vec<DatabasePokerHand>>> = Arc::new(Mutex::new(Vec::new()));

    while rows.len() > 0 {
        // Use Rayon's parallel iterator to process each row in parallel
        rows.par_iter().for_each(|row| {
            let hand = get_hand_from_cards_id(row.cards_id);
            let hand_histogram = sample_hand_strength(hand, 1000);
    
            // Acquire lock to update shared state
            let mut results_guard = results.lock().unwrap();
            results_guard.push(
                DatabasePokerHand { cards_id: row.cards_id.clone(), histogram: Some(hand_histogram.clone()), token: None }
            );
            drop(results_guard);
        });

        let mut results_guard = results.lock().unwrap();
        let _ = tx.try_send(results_guard.clone().to_vec());
        results_guard.clear();

        loop {
            match retrieve_batch(&session, Some(rows[rows.len()-1].token.unwrap())).await {
                Ok(result_rows) => {
                    rows = result_rows;
                    break;
                }
    
                Err(crdsError::Io(_)) => {
                    println!("Retry after 5s due to Io (probably connection reset) error...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    session = create_session_with_retry().await;
                }
                Err(err) => {
                    // Handle other errors
                    eprintln!("Error: {:?}", err);
                    println!("Retry after 5s due to error...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    session = create_session_with_retry().await;
                }
            }
        };
    }

    let mut remaining_results = results.lock().unwrap();
    if !remaining_results.is_empty() {
        let results_to_update = remaining_results.drain(..).collect::<Vec<_>>();
        for db_batch in results_to_update.chunks(UPDATE_BATCH_SIZE) {
            let _ = update_batch(&session, db_batch.to_vec()).await;
        }
    }
}
