use std::fs::File;
use std::io::BufWriter;
use serde::{Deserialize, Serialize};
use rmp_serde::Serializer;

#[derive(Serialize, Deserialize)]
struct Histograms(Vec<Vec<u8>>);

pub fn save_hand_strength_histograms_to_file(hand_strength_histograms: &Vec<Vec<u8>>, round: usize, batch: usize, export_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = format!("{}/round_{}_batch_{}.bin", export_path, round, batch);

    let output_file = File::create(filepath)?;

    let mut writer = BufWriter::new(output_file);
    let histograms = Histograms(hand_strength_histograms.clone());
    histograms.serialize(&mut Serializer::new(&mut writer))?;

    Ok(())
}

