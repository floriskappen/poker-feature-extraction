use std::fs::File;
use std::io::BufWriter;
use serde::{Deserialize, Serialize};
use rmp_serde::Serializer;

#[derive(Serialize, Deserialize)]
struct OpponentClusterHandStrengths(Vec<Vec<u8>>);

pub fn save_opponent_cluster_hand_strengths_to_file(opponent_cluster_hand_strengths: &Vec<Vec<u8>>, round: usize, batch: usize, export_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = format!("{}/round_{}_batch_{}.bin", export_path, round, batch);

    let output_file = File::create(filepath)?;

    let mut writer = BufWriter::new(output_file);
    let opponent_cluster_hand_strengths = OpponentClusterHandStrengths(opponent_cluster_hand_strengths.clone());
    opponent_cluster_hand_strengths.serialize(&mut Serializer::new(&mut writer))?;

    Ok(())
}
