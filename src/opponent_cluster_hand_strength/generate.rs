
use hand_isomorphism_rust::deck::card_to_string;
use ocl::builders::{BufferBuilder, KernelBuilder};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::process::abort;
use itertools::Itertools;
use std::error::Error;

use crate::kernel::KernelContainer;
use crate::load::HandLoader;
use crate::opponent_cluster_hand_strength::save::save_opponent_cluster_hand_strengths_to_file;

#[derive(Serialize, Deserialize, Debug)]
pub struct Labels(Vec<usize>);

fn load_opponent_cluster_labels(labels_filepath: &str,) -> Result<Labels, Box<dyn Error>> {
    let input_file = File::open(labels_filepath)?;
    let mut buf_reader = BufReader::new(input_file);
    let mut deserialized = Deserializer::new(&mut buf_reader);
    
    // Deserialize directly using MessagePack deserializer
    let labels: Labels = Deserialize::deserialize(&mut deserialized)?;

    return Ok(labels);
}


pub fn generate_opponent_cluster_hand_strengths(round: usize, path_export: &str, path_opponent_cluster_labels: &str) {
    let mut hand_loader = HandLoader::new(round).expect("Failed to initialize HandLoader");
    let hand_loader_preflop = HandLoader::new(0).expect("Failed to initialize preflop HandLoader for round");
    log::info!("Initialized HandLoader with round {} and batch 0/{}", round, hand_loader.total_batches-1);
    let hands_preflop = &hand_loader_preflop.current_batch_hands;
    let labels_preflop = load_opponent_cluster_labels(path_opponent_cluster_labels)
        .expect("Failed to load opponent cluster labels");

    // println!("hands_preflop: {:?}", hands_preflop);
    // println!("labels_preflop: {:?}", labels_preflop);

    // Build clusters from the labels and hands
    let mut clusters: Vec< // Holds all 8 clusters
        Vec< // Cluster
            &Vec<u8> // Hand (size 2)
        >
    > = vec![vec![]; 8];

    for (index, hand) in hands_preflop.iter().enumerate() {
        clusters[labels_preflop.0[index] as usize].push(hand)
    }

    // Prepare to flatten clusters
    let mut cluster_hands: Vec<u8> = Vec::new();
    let mut cluster_offsets: Vec<i32> = Vec::new();
    let mut cluster_sizes: Vec<i32> = Vec::new();

    // Flatten the clusters and collect metadata
    let mut current_offset = 0;
    for cluster in clusters.iter() {
        cluster_offsets.push(current_offset);
        cluster_sizes.push(cluster.len() as i32);
        for hand in cluster {
            // Assumes each hand is a Vec<u8> with exactly 2 elements
            cluster_hands.extend_from_slice(hand);
        }
        current_offset += (cluster.len() * 2) as i32; // Each hand has 2 cards, hence `* 2`
    }
    // println!("clusters: {:?}", clusters);
    // println!("clusters: {:?}", clusters.iter()
    //     .map(|cluster| cluster.iter()
    //         .map(|hand| {
    //             hand.iter().map(|&card| card_to_string(card)).collect::<Vec<String>>()
    //         })
    //         .collect::<Vec<Vec<String>>>()
    //     ).collect::<Vec<Vec<Vec<String>>>>()
    // );
    // println!("Flattened Hands: {:?}", cluster_hands);
    // println!("Cluster Offsets: {:?}", cluster_offsets);
    // println!("Cluster Sizes: {:?}", cluster_sizes);
    
    // println!("hands_preflop.len(): {:?}", hands_preflop.len());
    // println!("cluster_hands.len(): {:?}", cluster_hands.len());


    let src = include_str!("./kernel.cl");
    let kernel_container = KernelContainer::new(src);
    let max_work_group_size = kernel_container.device.max_wg_size().unwrap();
    let gpu_chunk_size = max_work_group_size * 32;
    // let gpu_chunk_size = 10;
    log::info!("Set max group size to {}", gpu_chunk_size);

    for batch_index in 0..hand_loader.total_batches {
        let mut hands_analyzed = 0;
        let current_batch_hands_amount = hand_loader.current_batch_hands.len();

        let hands = &hand_loader.current_batch_hands;
        let mut results: Vec<Vec<u8>> = vec![];

        for (gpu_batch_index, chunk) in hands.chunks(gpu_chunk_size).enumerate() {
            let hands_data_flattened = chunk.to_vec().concat();

            // Number of hands
            let num_hands = chunk.len();

            let mut opponent_cluster_hand_strengths: Vec<i32> = vec![0; num_hands * 8];

            let hands_buffer = BufferBuilder::<u8>::new()
                .flags(ocl::flags::MEM_READ_ONLY)
                .len(hands_data_flattened.len())
                .copy_host_slice(&hands_data_flattened)
                .context(&kernel_container.context)
                .build().unwrap();

            let opponent_cluster_hand_strengths_buffer = BufferBuilder::<i32>::new()
                .flags(ocl::flags::MEM_READ_WRITE)
                .len(opponent_cluster_hand_strengths.len())
                .context(&kernel_container.context)
                .build().unwrap();

            // Make sure we clear the opponent_cluster_hand_strengths buffer since the GPU caches it appaerently
            opponent_cluster_hand_strengths_buffer.cmd()
                .queue(&kernel_container.queue)
                .fill(0, None)
                .enq()
                .unwrap();

            let cluster_hands_buffer = BufferBuilder::<u8>::new()
                .flags(ocl::flags::MEM_READ_ONLY)
                .len(cluster_hands.len())
                .copy_host_slice(&cluster_hands)
                .context(&kernel_container.context)
                .build().unwrap();

            let cluster_offsets_buffer = BufferBuilder::<i32>::new()
                .flags(ocl::flags::MEM_READ_ONLY)
                .len(cluster_offsets.len())
                .copy_host_slice(&cluster_offsets)
                .context(&kernel_container.context)
                .build().unwrap();

            let cluster_sizes_buffer = BufferBuilder::<i32>::new()
                .flags(ocl::flags::MEM_READ_ONLY)
                .len(cluster_sizes.len())
                .copy_host_slice(&cluster_sizes)
                .context(&kernel_container.context)
                .build().unwrap();

            // Setup kernel
            let kernel = KernelBuilder::new()
                .program(&kernel_container.program)
                .name("simulate_poker_hands")
                .arg(&hands_buffer)
                .arg(&opponent_cluster_hand_strengths_buffer)
                .arg(&cluster_hands_buffer)
                .arg(&cluster_offsets_buffer)
                .arg(&cluster_sizes_buffer)
                .arg(num_hands)
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
            opponent_cluster_hand_strengths_buffer.cmd()
                .queue(&kernel_container.queue)
                .read(&mut opponent_cluster_hand_strengths)
                .enq()
                .unwrap();

            
            let opponent_cluster_hand_strengths_unflattened = opponent_cluster_hand_strengths.chunks(8)
                .map(|chunk| chunk.iter().map(|&value| value as u8).collect())
                .collect_vec();
            // println!("hands: {:?}", chunk.iter().map(|hand| hand.iter().map(|&card| card_to_string(card)).collect::<Vec<_>>()).collect::<Vec<_>>());
            // println!("opponent_cluster_hand_strengths_unflattened: {:?}", opponent_cluster_hand_strengths_unflattened);

            hands_analyzed += opponent_cluster_hand_strengths_unflattened.len();
            if gpu_batch_index > 0 && gpu_batch_index % 500 == 0 {
                log::info!(
                    "Finished GPU batch. Round {}, batch {}/{} gpu batch {} hands {}/{} in batch",
                    round,
                    batch_index,
                    hand_loader.total_batches-1,
                    gpu_batch_index,
                    hands_analyzed,
                    current_batch_hands_amount
                );
            }

            results.extend(opponent_cluster_hand_strengths_unflattened);
        }

        save_opponent_cluster_hand_strengths_to_file(&results, round, batch_index, path_export)
            .expect(format!("ERROR: Failed to save HSH for round {} batch #{}", round, batch_index).as_str());

        results.clear();

        hand_loader.load_next_batch();
    }
}
