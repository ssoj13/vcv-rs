//! Windows and Linux CLI orchestration. Render fully before writing executable output.

mod args;
mod platform;
mod sdks;

use args::{Args, Format};
use clap::Parser;
use std::io::{self, Write};
use vcv_rs::format;

pub fn run() {
    let args = Args::parse();
    let mut output = match execute(&args) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
    };
    if !output.ends_with('\n') {
        output.push('\n');
    }
    if let Err(error) = io::stdout().lock().write_all(output.as_bytes()) {
        eprintln!("Error: cannot write environment output: {error}");
        std::process::exit(1);
    }
}

fn execute(args: &Args) -> Result<String, String> {
    let (host, target) = platform::architectures(args)?;
    let format = match args.format {
        Format::Auto => args::detect_shell(),
        format => format,
    };
    if !cfg!(windows) && format == Format::Cmd {
        return Err("CMD output is supported only on Windows".into());
    }
    let selected = sdks::resolve(args, target)?;
    if args.list {
        let mut output = platform::inventory(args, selected.cuda_bounds());
        output.push_str(&sdks::inventory(args, &selected, target));
        return Ok(output);
    }
    let mut base = platform::build(args, host, target, selected.cuda_bounds())?;
    selected.apply(&mut base.env)?;
    let rendered = match format {
        Format::Cmd => format::fmt_cmd(&base.env),
        Format::Ps | Format::Powershell => format::fmt_ps(&base.env),
        Format::Sh | Format::Bash => format::fmt_sh(&base.env),
        Format::Json => format::fmt_json(&base.env),
        Format::Auto => return Err("shell detection did not select an output format".into()),
    }
    .map_err(|error| error.to_string())?;
    if !args.quiet {
        for info in base.info.into_iter().chain(selected.info(target)) {
            eprintln!("# {info}");
        }
    }
    Ok(rendered)
}
