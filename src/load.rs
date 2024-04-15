
use std::{fs, error::Error};
use std::fs::File;
use std::io::BufReader;
use bincode;
use itertools::Itertools;

use crate::encode::decode_cards;

fn load_data(file_path: &str) -> Result<Vec<i64>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let hands: Vec<i64> = bincode::deserialize_from(reader)?;
    Ok(hands)
}

pub struct HandLoader {
    pub batch_size: usize,
    pub total_batches: usize,
    pub current_batch: usize,
    pub folder_path: String,
    pub file_names: Vec<String>,
    pub round: usize,
    pub current_batch_hands: Vec<i64>,
}

impl HandLoader {
    pub fn new(round: usize) -> Result<Self, Box<dyn Error>> {
        let folder_path = std::env::var("CANONICAL_HANDS_FOLDER_PATH")?;

        let entries = fs::read_dir(&folder_path)?;
        let mut first_batch_file_path = "";
        let file_names: Vec<String> = entries.map(|entry| {
            if let Ok(entry) = entry {
                if let Ok(file_name) = entry.file_name().into_string() {
                    return file_name;
                }
            }

            return "".to_string();
        })
            .filter(|file_name| file_name != "")
            .collect_vec();

        let round_file_names: Vec<String> = file_names.iter()
            .cloned()
            .filter(|file_name| file_name.starts_with(format!("round_{}_batch_", round).as_str()))
            .collect();

        let total_batches = round_file_names.len();
        let first_batch_file_name = round_file_names.iter()
            .find(|&file_name| file_name == &format!("round_{}_batch_0.bin", round));

        if let Some(first_batch_file_name) = first_batch_file_name {
            let file_path = format!("{}/{}", &folder_path, first_batch_file_name);
            let current_batch_hands = load_data(&file_path)?;

            return Ok(Self {
                batch_size: current_batch_hands.len(),
                total_batches,
                current_batch: 0,
                folder_path,
                file_names: round_file_names,
                round,
                current_batch_hands
            })
        }

        return Err("Failed".into());
    }

    pub fn load_next_batch(&mut self) {
        if self.current_batch < self.total_batches-1 {
            let new_batch = self.current_batch + 1;
            let new_file_name = self.file_names.iter()
                .find(|&file_name| file_name == &format!("round_{}_batch_{}.bin", self.round, new_batch))
                .expect(format!("Could not find file for round {} batch {}", self.round, new_batch).as_str());
            let file_path = format!("{}/{}", &self.folder_path, new_file_name);
            let current_batch_hands = load_data(&file_path)
                .expect(format!("Could not load file for round {} batch {}", self.round, new_batch).as_str());

            self.current_batch += 1;
            self.current_batch_hands = current_batch_hands;
        }
    }
}
