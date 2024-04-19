use std::fs::File;
use std::io::{BufWriter, Write};
use prost::Message;

use crate::proto::OpponentClusterHandStrengthHistograms;

pub fn save_opponent_cluster_hand_strengths_to_file(opponent_cluster_hand_strengths: Vec<Vec<u8>>, round: usize, batch: usize, export_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = format!("{}/round_{}_batch_{}.bin", export_path, round, batch);

    let data = OpponentClusterHandStrengthHistograms {
        data: opponent_cluster_hand_strengths,
    };

    let mut buf = Vec::new();
    data.encode(&mut buf)?;

    let mut file = BufWriter::new(File::create(filepath)?);
    file.write_all(&buf)?;
    Ok(())
}
