//! Windows and Linux environment CLI entry. The binary requires the `cli` feature.

mod cli;

fn main() {
    cli::run();
}
