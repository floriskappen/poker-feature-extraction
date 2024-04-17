mod encode;
mod load;
mod logger;
mod kernel;

use dotenv::dotenv;
use itertools::Itertools;
use num_bigint::BigUint;
use ocl::builders::{BufferBuilder, KernelBuilder};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::process::abort;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use rmp_serde::{Deserializer, Serializer};

use crate::encode::{decode_hand_strength_histogram};
use crate::kernel::KernelContainer;
use crate::load::HandLoader;
use crate::logger::init_logger;

static EXPORT_PATH: &str = "./exports";

#[derive(Serialize, Deserialize)]
struct Histograms(Vec<Vec<u8>>);

fn save_hand_strength_histograms_to_file(hand_strength_histograms: &Vec<Vec<u8>>, round: usize, batch: usize) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = format!("{}/round_{}_batch_{}.bin", EXPORT_PATH, round, batch);

    let output_file = File::create(filepath)?;

    let mut writer = BufWriter::new(output_file);
    let histograms = Histograms(hand_strength_histograms.clone());
    histograms.serialize(&mut Serializer::new(&mut writer))?;

    Ok(())
}

// fn reserealize_hand_strength_histogram() -> Result<(), Box<dyn Error>> {
//     // let filepath = "/Users/kade/git/personal/pluribus/poker-k-means/data_in/round_0_batch_0.bin";
//     let input_filepath = "/Users/kade/git/personal/pluribus/poker-k-means/data_in/round_2_batch_1.bin";
//     let output_filepath = "/Users/kade/git/personal/pluribus/poker-k-means/data_in/round_2_batch_1_converted.bin";

//     // Open the file in read-only mode.
//     let input_file = File::open(input_filepath)?;

//     // Create a buffer reader for efficient reading.
//     let reader = BufReader::new(input_file);

//     // Deserialize the data from the file using bincode.
//     let hand_strength_histograms_bigints: Vec<Vec<u8>> = bincode::deserialize_from(reader)?;
//     let hand_strength_histograms: Vec<Vec<u8>> = hand_strength_histograms_bigints.iter().map(|bigint_serialized| {
//         let bigint_histogram = BigUint::from_bytes_le(bigint_serialized);
//         let decoded_hsh = decode_hand_strength_histogram(bigint_histogram);
//         return decoded_hsh;
//     }).collect();
//     drop(hand_strength_histograms_bigints);
//     let histograms = Histograms(hand_strength_histograms);

//     let output_file = File::create(output_filepath)?;
//     let mut buf_writer = BufWriter::new(output_file);
//     histograms.serialize(&mut Serializer::new(&mut buf_writer))?;

//     Ok(())
// }

fn main() {
    init_logger().expect("Failed to initialize logger");
    dotenv().ok();
    
    let round = 0;
    let mut hand_loader = HandLoader::new(round).expect("Failed to initialize HandLoader");

    log::info!("Initialized HandLoader with round {} and batch 0/{}", round, hand_loader.total_batches-1);

    let trials_per_hand = if round == 0 || round == 1 {
        1000
    } else if round == 2 {
        700
    } else {
        400
    };
    let cards_per_hand = if round == 0 { 2 } else if round == 1 { 5 } else if round == 2 { 6 } else { 7 };

    let src = include_str!("../kernel.cl");
    let kernel_container = KernelContainer::new(src);
    let max_work_group_size = kernel_container.device.max_wg_size().unwrap();
    let gpu_chunk_size = max_work_group_size * 32;
    // let gpu_chunk_size = 10;
    log::info!("Set max group size to {}", gpu_chunk_size);
    
    for batch_index in 0..hand_loader.total_batches {
        let mut hands_analyzed = 0;
        let current_batch_hands = hand_loader.current_batch_hands.len();

        let hands = &hand_loader.current_batch_hands;
        let mut results: Vec<Vec<u8>> = vec![];

        for (gpu_batch_index, chunk) in hands.chunks(gpu_chunk_size).enumerate() {
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

            histograms_buffer.cmd()
                .queue(&kernel_container.queue)
                .fill(0, None)
                .enq()
                .unwrap();

            // Generate a seed based on the current time
            let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32 + gpu_batch_index as u32;

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
                    .global_work_size(gpu_chunk_size)
                    .enq()
                    .unwrap();
            }
            
            kernel_container.queue.finish().unwrap();

            // Read the data back into a Rust vector
            histograms_buffer.cmd().queue(&kernel_container.queue).read(&mut histograms).enq().unwrap();

            let histograms_unflattened_normalized = histograms.chunks(30)
                .map(|chunk| {
                    chunk.iter().map(|&bin_value| {
                        let normalized = ((bin_value as f32 / trials_per_hand as f32) * 100.0) as u8;
                        return normalized
                    }).collect::<Vec<u8>>()
                })
                .collect_vec();

            if histograms_unflattened_normalized[histograms_unflattened_normalized.len()-1].iter().map(|&ch| ch as u32).sum::<u32>() == 0 {
                log::error!(
                    "Last histogram (and probably others) is not correctly filled. Round {}, batch {}/{} gpu batch {}",
                    round,
                    batch_index,
                    hand_loader.total_batches-1,
                    gpu_batch_index
                );
            }

            hands_analyzed += histograms_unflattened_normalized.len();
            log::info!(
                "Finished GPU batch. Round {}, batch {}/{} gpu batch {} hands {}/{} in batch",
                round,
                batch_index,
                hand_loader.total_batches-1,
                gpu_batch_index,
                hands_analyzed,
                current_batch_hands
            );

            results.extend(histograms_unflattened_normalized);
        }

        save_hand_strength_histograms_to_file(&results, round, batch_index)
            .expect(format!("ERROR: Failed to save HSH for round {} batch #{}", round, batch_index).as_str());

        results.clear();

        hand_loader.load_next_batch();
    }
}
