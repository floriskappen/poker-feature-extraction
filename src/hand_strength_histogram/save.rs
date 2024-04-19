use std::fs::File;
use std::io::{BufWriter, Write};
use prost::Message;

use crate::proto::HandStrengthHistograms;

pub fn save_hand_strength_histograms_to_file(hand_strength_histograms: Vec<Vec<u8>>, round: usize, batch: usize, export_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = format!("{}/round_{}_batch_{}.bin", export_path, round, batch);

    let data = HandStrengthHistograms {
        data: hand_strength_histograms,
    };

    let mut buf = Vec::new();
    data.encode(&mut buf)?;

    let mut file = BufWriter::new(File::create(filepath)?);
    file.write_all(&buf)?;
    Ok(())
}
