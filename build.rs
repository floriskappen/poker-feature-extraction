fn main() {
    prost_build::Config::new()
        .out_dir("src/proto/build")
        .compile_protos(&[
            "src/proto/clustered_data_labels.proto",
            "src/proto/hand_strength_histograms.proto",
            "src/proto/opponent_cluster_hand_strength_histograms.proto"
            ], &["src/"])
        .unwrap();
}
