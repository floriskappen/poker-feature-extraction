use std::fs::File;
use std::io::BufWriter;
use serde::Serialize;
use rmp_serde::Serializer;

pub fn save_hand_strength_histograms_to_file(hand_strength_histograms: &Vec<Vec<u8>>, round: usize, batch: usize, export_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = format!("{}/round_{}_batch_{}.bin", export_path, round, batch);

    let output_file = File::create(filepath)?;

    let mut writer = BufWriter::new(output_file);
    hand_strength_histograms.serialize(&mut Serializer::new(&mut writer))?;

    Ok(())
}

