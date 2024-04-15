mod game;
mod encode;
mod load;
mod logger;
mod kernel;

use dotenv::dotenv;
use itertools::Itertools;
use ocl::builders::{BufferBuilder, KernelBuilder};
use ocl::ffi::libc::abort;
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use ocl::{ProQue, Buffer, flags};

use game::sample_hand_strength;
use crate::game::get_hand_from_cards_id;
use crate::encode::encode_hand_strength_histogram;
use crate::kernel::{KernelContainer};
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

    let trials_per_hand = if round == 0 || round == 1 {
        1000
    } else if round == 2 {
        700
    } else {
        400
    };
    let cards_per_hand = if round == 0 { 2 } else if round == 1 { 5 } else if round == 3 { 6 } else { 7 };

    let src = include_str!("/Users/kade/git/personal/pluribus/poker-hand-strength-histogram/kernel.cl");
    let kernel_container = KernelContainer::new(src);

    for _batch_index in 0..hand_loader.total_batches {
        let hands = &hand_loader.current_batch_hands;

        let gpu_chunk_size = 5000;

        for (index, chunk) in hands.chunks(gpu_chunk_size).enumerate() {
            let hands_data_flattened = chunk.to_vec().concat();

            // Number of hands and trials
            let num_hands = chunk.len();

            let mut histograms: Vec<i32> = vec![0; num_hands * 30];

            let hands_buffer = BufferBuilder::<u8>::new()
                .flags(ocl::flags::MEM_READ_ONLY)
                .len(hands_data_flattened.len())
                .copy_host_slice(&hands_data_flattened)
                .context(&kernel_container.context)
                .build().unwrap();

            let histograms_buffer = BufferBuilder::<i32>::new()
                .flags(ocl::flags::MEM_READ_WRITE)
                .len(histograms.len())
                .context(&kernel_container.context)
                .build().unwrap();

            // Generate a seed based on the current time
            let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32 + index as u32;

            // Setup kernel
            let kernel = KernelBuilder::new()
                .program(&kernel_container.program)
                .name("simulate_poker_hands")
                .arg(&hands_buffer)
                .arg(&histograms_buffer)
                .arg(num_hands as u32)
                .arg(trials_per_hand as u32)
                .arg(cards_per_hand as u32)
                .arg(seed)
                .build()
                .unwrap();

            unsafe { 
                kernel
                    .cmd()
                    .queue(&kernel_container.queue)
                    .global_work_size(num_hands)
                    .enq()
                    .unwrap();
            }
    
            // Read the data back into a Rust vector
            histograms_buffer.cmd().queue(&kernel_container.queue).read(&mut histograms).enq().unwrap();

            let histogram_converted = histograms.chunks(30)
                .collect_vec()
                .iter()
                .map(|histogram| histogram.iter()
                    .map(|&bin_value| {
                        return ((bin_value as f32 / trials_per_hand as f32) * 100.0) as u8
                    }).collect()
                )
                .collect::<Vec<Vec<u8>>>();
            println!("OUTPUT: {:?}", histogram_converted);
            // for chunk in histograms.chunks(30) {
            //     println!("chunk_sum: {}", chunk.iter().sum::<i32>())
            // }
            unsafe { abort() };
        }



        // let mut results_guard = results.lock().unwrap();
        // log::info!(
        //     "Calculated HSH for round {} batch {}/{} hand {}/{}",
        //     round,
        //     batch,
        //     hand_loader.total_batches-1,
        //     results_guard.len(),
        //     hands.len()
        // );

        // save_hand_strength_histograms_to_file(&results_guard, round, batch)
        //     .expect(format!("ERROR: Failed to save HSH for round {} batch #{}", round, batch).as_str());
        // results_guard.clear();

        // log::info!("Saved encoded HSHs for round {} batch {}/{}", round, batch, hand_loader.total_batches-1);

        // if batch < hand_loader.total_batches - 1 {
        //     hand_loader.load_next_batch();
        //     log::info!("Loaded HandLoader with round {} and batch {}/{}", round, batch + 1, hand_loader.total_batches-1);
        // }

    }
}
