mod encode;
mod load;
mod logger;
mod kernel;
mod hand_strength_histogram {
    pub mod generate;
    pub mod save;
}
mod opponent_cluster_hand_strength {
    pub mod generate;
    pub mod save;
    pub mod load_labels;
}
mod proto {
    include!("proto/build/_.rs");
}

use dotenv::dotenv;
use hand_strength_histogram::generate::generate_hand_strength_histograms;
use opponent_cluster_hand_strength::generate::generate_opponent_cluster_hand_strengths;

use crate::logger::init_logger;

static PATH_EXPORT: &str = "./exports";
static PATH_OPPONENT_CLUSTER_LABELS: &str = "./imports/labels_round_0_initialization_772.bin";

fn main() {
    init_logger().expect("Failed to initialize logger");
    dotenv().ok();
    
    generate_hand_strength_histograms(1, PATH_EXPORT);

    // generate_opponent_cluster_hand_strengths(3, PATH_EXPORT, PATH_OPPONENT_CLUSTER_LABELS);
}
