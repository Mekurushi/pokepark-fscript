mod cli;

use clap::Parser;
use cli::{BuildArgs, CheckArgs, Cli, Command, ConfigArgs, DiagnosticFormat, PatchArgs};
use fsc_compiler::{
    CheckMode, CompileRequest, ConfigRequirement, ConfigType, check, compile, required_configs,
};
use fsc_diagnostics::{Diagnostic, render_diagnostics, render_diagnostics_json};
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    if run(Cli::parse()).is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run(cli: Cli) -> Result<(), ()> {
    match cli.command {
        Command::Build(args) => build(args),
        Command::Check(args) => check_file(args),
        Command::Configs(args) => configs(args),
        Command::Patch(args) => patch(args),
    }
}

fn configs(args: ConfigArgs) -> Result<(), ()> {
    //TODO: JSON output?
    let source = fs::read_to_string(&args.input).map_err(|error| {
        eprintln!("error: could not read {}: {error}", args.input.display());
    })?;
    let Some(source_name) = args.input.file_name().and_then(|name| name.to_str()) else {
        eprintln!("error: input path does not have a valid UTF-8 file name");
        return Err(());
    };

    match required_configs(&source) {
        Ok(requirements) => {
            render_config_requirements(&requirements);
            Ok(())
        }
        Err(failure) => {
            render_check_diagnostics(
                DiagnosticFormat::Human,
                failure.diagnostics(),
                source_name,
                &source,
            )?;
            Err(())
        }
    }
}

fn render_config_requirements(requirements: &[ConfigRequirement]) {
    for requirement in requirements {
        println!("{}: {}", requirement.name, config_type_name(requirement.ty));
    }
}

const fn config_type_name(ty: ConfigType) -> &'static str {
    match ty {
        ConfigType::Int => "int",
        ConfigType::Float => "float",
        ConfigType::Bool => "bool",
        ConfigType::String => "string",
    }
}

fn check_file(args: CheckArgs) -> Result<(), ()> {
    let source = fs::read_to_string(&args.input).map_err(|error| {
        eprintln!("error: could not read {}: {error}", args.input.display());
    })?;
    let Some(source_name) = args.input.file_name().and_then(|name| name.to_str()) else {
        eprintln!("error: input path does not have a valid UTF-8 file name");
        return Err(());
    };

    let output = check(&source, CheckMode::Source);
    render_check_diagnostics(
        args.diagnostic_format,
        output.diagnostics(),
        source_name,
        &source,
    )?;
    if output.has_errors() {
        Err(())
    } else {
        Ok(())
    }
}

fn render_check_diagnostics(
    format: DiagnosticFormat,
    diagnostics: &[Diagnostic],
    source_name: &str,
    source: &str,
) -> Result<(), ()> {
    match format {
        DiagnosticFormat::Human => {
            eprint!("{}", render_diagnostics(diagnostics, source_name, source));
        }
        DiagnosticFormat::Json => {
            let rendered = render_diagnostics_json(diagnostics, source_name).map_err(|error| {
                eprintln!("error: could not serialize diagnostics as JSON: {error}");
            })?;
            print!("{rendered}");
        }
    }
    Ok(())
}

fn patch(args: PatchArgs) -> Result<(), ()> {
    let patch_source = fs::read_to_string(&args.patch).map_err(|error| {
        eprintln!(
            "error: could not read patch source {}: {error}",
            args.patch.display()
        );
    })?;
    let original_binary = fs::read(&args.original).map_err(|error| {
        eprintln!(
            "error: could not read original FSB {}: {error}",
            args.original.display()
        );
    })?;
    let symbols_source = fs::read_to_string(&args.symbols).map_err(|error| {
        eprintln!(
            "error: could not read symbol table {}: {error}",
            args.symbols.display()
        );
    })?;
    let symbols = fsb_patcher::parse_symbol_table(&symbols_source).map_err(|error| {
        eprintln!(
            "error: could not parse symbol table {}: {error}",
            args.symbols.display()
        );
    })?;

    // TODO: add config support
    let config_values = fsb_patcher::ConfigValues::new();
    let request =
        fsb_patcher::PatchRequest::new(&patch_source, &original_binary, &symbols, &config_values);
    let artifact = fsb_patcher::patch(request).map_err(|error| {
        eprintln!("error: {error}");
    })?;
    let symbols_output =
        fsb_patcher::serialize_symbol_table(artifact.symbols()).map_err(|error| {
            eprintln!("error: could not serialize output symbol table: {error}");
        })?;

    fs::write(&args.output, artifact.binary()).map_err(|error| {
        eprintln!(
            "error: could not write patched FSB {}: {error}",
            args.output.display()
        );
    })?;
    fs::write(&args.symbols_output, symbols_output).map_err(|error| {
        eprintln!(
            "error: could not write output symbol table {}: {error}",
            args.symbols_output.display()
        );
    })?;

    Ok(())
}

fn build(args: BuildArgs) -> Result<(), ()> {
    let input = args.input;
    let output = args.output.unwrap_or_else(|| input.with_extension("fsb"));
    let source = fs::read_to_string(&input).map_err(|error| {
        eprintln!("error: could not read {}: {error}", input.display());
    })?;
    let Some(source_name) = input.file_name().and_then(|name| name.to_str()) else {
        eprintln!("error: input path does not have a valid UTF-8 file name");
        return Err(());
    };
    let Some(script_name) = input.file_stem().and_then(|name| name.to_str()) else {
        eprintln!("error: input path does not have a valid UTF-8 file stem");
        return Err(());
    };

    // TODO: add config support
    let config_values = fsc_compiler::ConfigValues::new();
    let request = CompileRequest::new(&source, script_name, &config_values);
    let artifact = compile(request).map_err(|failure| {
        eprint!(
            "{}",
            render_diagnostics(failure.diagnostics(), source_name, &source)
        );
    })?;

    let rendered = render_diagnostics(artifact.diagnostics(), source_name, &source);
    if !rendered.is_empty() {
        eprint!("{rendered}");
    }
    fs::write(&output, artifact.bytes()).map_err(|error| {
        eprintln!("error: could not write {}: {error}", output.display());
    })?;
    eprintln!(
        "compiled {} → {}  ({} bytes)",
        input.display(),
        output.display(),
        artifact.bytes().len(),
    );
    Ok(())
}
