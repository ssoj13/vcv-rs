//! `vcv` CLI entry.
//!
//! Default features include `cli` and `cuda`. The Windows implementation lives in `cli`;
//! other targets get a stub so `cargo build` with defaults stays green in a mixed workspace.

#[cfg(all(windows, feature = "cli"))]
mod cli;

fn main() {
    #[cfg(all(windows, feature = "cli"))]
    cli::run();

    #[cfg(not(all(windows, feature = "cli")))]
    {
        eprintln!("vcv is a Windows-only CLI (Visual Studio / MSVC environment).");
        std::process::exit(1);
    }
}
