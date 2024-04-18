mod encode;
mod load;
mod logger;
mod kernel;
mod hand_strength_histogram {
    pub mod generate;
    pub mod save;
}

use dotenv::dotenv;
use hand_strength_histogram::generate::generate_hand_strength_histograms;

use crate::logger::init_logger;

static EXPORT_PATH: &str = "./exports";

fn main() {
    init_logger().expect("Failed to initialize logger");
    dotenv().ok();
    
    generate_hand_strength_histograms(0, EXPORT_PATH);
}
