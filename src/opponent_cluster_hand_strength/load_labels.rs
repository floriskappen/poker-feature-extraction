use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use prost::Message;
use crate::proto::ClusteredDataLabels;

pub fn load_opponent_cluster_labels(labels_filepath: &str,) -> Result<Vec<u32>, Box<dyn Error>> {
    let mut file = BufReader::new(File::open(labels_filepath)?);
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let data = ClusteredDataLabels::decode(&*buf)?;
    Ok(data.data)
}
