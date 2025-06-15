# Poker Feature Extraction

This project generates poker hand-strength histograms and opponent-cluster features for a Pluribus-style information abstraction. It relies on GPU OpenCL kernels to simulate large numbers of hands and output precomputed features used in training poker AIs.

## Prerequisites

- Rust toolchain with `cargo`
- A GPU with OpenCL support
- The `libs/hand-isomorphism-rust` submodule

## Initializing the Submodule

Clone the repository with submodules or initialize them after cloning:

```bash
git submodule update --init --recursive
```

## Environment Setup

Set the following environment variable so the loader can find the canonical hand batches:

```bash
export CANONICAL_HANDS_FOLDER_PATH=/path/to/canonical/hands
```

The opponent-cluster label file should be available at:

```
imports/labels_round_0_initialization_237.bin
```

## Building and Running

Build the project with cargo:

```bash
cargo build
```

Run the generation program (edit `main.rs` to choose which feature generator to call):

```bash
cargo run --release
```

## Output

Generated histograms and opponent-cluster strength files are written to the `exports/` directory. Example command for producing histograms for round one is simply running the binary as above, which calls `generate_hand_strength_histograms(1, "./exports")` by default.