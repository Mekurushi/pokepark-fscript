mod cli;

use std::fs;
use std::process::ExitCode;

use clap::Parser;
use cli::Cli;

fn main() -> ExitCode {
    if run(Cli::parse()).is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run(cli: Cli) -> Result<(), ()> {
    let input = cli.input;
    let output = cli.output.unwrap_or_else(|| input.with_extension("fsb"));
    let source = fs::read_to_string(&input).map_err(|error| {
        eprintln!("error: could not read {}: {error}", input.display());
    })?;
    let Some(script_name) = input.file_stem().and_then(|name| name.to_str()) else {
        eprintln!("error: input path does not have a valid UTF-8 file stem");
        return Err(());
    };

    let script = fsa::parse(&source).map_err(|error| {
        eprintln!("error: could not parse {}: {error}", input.display());
    })?;
    let program = fsa::lower(&script).map_err(|error| {
        eprintln!("error: could not lower {}: {error}", input.display());
    })?;
    let unit = fsb_assembler::assemble_program(&program).map_err(|error| {
        eprintln!("error: could not assemble {}: {error}", input.display());
    })?;
    let binary = unit.into_binary(script_name.to_owned()).map_err(|error| {
        eprintln!("error: could not finalize {}: {error}", input.display());
    })?;
    let bytes = binary.serialize().map_err(|error| {
        eprintln!("error: could not serialize {}: {error}", input.display());
    })?;

    fs::write(&output, &bytes).map_err(|error| {
        eprintln!("error: could not write {}: {error}", output.display());
    })?;
    eprintln!(
        "assembled {} → {}  ({} bytes)",
        input.display(),
        output.display(),
        bytes.len(),
    );
    Ok(())
}
