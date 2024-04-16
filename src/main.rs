mod encode;
mod load;
mod logger;
mod kernel;

use dotenv::dotenv;
use itertools::Itertools;
use ocl::builders::{BufferBuilder, KernelBuilder};
use std::fs::File;
use std::io::BufWriter;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::encode::encode_hand_strength_histogram;
use crate::kernel::KernelContainer;
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

    let trials_per_hand = if round == 0 || round == 1 {
        1000
    } else if round == 2 {
        700
    } else {
        400
    };
    let cards_per_hand = if round == 0 { 2 } else if round == 1 { 5 } else if round == 3 { 6 } else { 7 };

    let src = include_str!("../kernel.cl");
    let kernel_container = KernelContainer::new(src);
    let max_work_group_size = kernel_container.device.max_wg_size().unwrap();
    let gpu_chunk_size = max_work_group_size * 32;
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
                    .global_work_size(num_hands)
                    .enq()
                    .unwrap();
            }
            
            kernel_container.queue.finish().unwrap();

            // Read the data back into a Rust vector
            histograms_buffer.cmd().queue(&kernel_container.queue).read(&mut histograms).enq().unwrap();

            let histograms_unflattened = histograms.chunks(30)
                .map(|chunk| chunk.iter().map(|&char| char as u8).collect::<Vec<u8>>())
                .collect_vec();
            if histograms_unflattened[histograms_unflattened.len()-1].iter().map(|&ch| ch as u32).sum::<u32>() == 0 {
                log::error!(
                    "Last histogram (and probably others) is not correctly filled. Round {}, batch {}/{} gpu batch {}",
                    round,
                    batch_index,
                    hand_loader.total_batches-1,
                    gpu_batch_index
                );
            }

            hands_analyzed += histograms_unflattened.len();
            log::info!(
                "Finished GPU batch. Round {}, batch {}/{} gpu batch {} hands {}/{} in batch",
                round,
                batch_index,
                hand_loader.total_batches-1,
                gpu_batch_index,
                hands_analyzed,
                current_batch_hands
            );

            let histograms_encoded: Vec<Vec<u8>> = histograms_unflattened.iter()
                .map(|histogram| {
                    let encoded_histogram = encode_hand_strength_histogram(histogram);
                    let encoded_histogram_bytes = encoded_histogram.to_bytes_le();
                    return encoded_histogram_bytes
                }).collect();

            results.extend(histograms_encoded);
        }

        save_hand_strength_histograms_to_file(&results, round, batch_index)
            .expect(format!("ERROR: Failed to save HSH for round {} batch #{}", round, batch_index).as_str());

        results.clear();

        hand_loader.load_next_batch();
    }
}
