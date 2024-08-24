use htmeta::{HtmlEmitter, HtmlEmitterBuilder};
use kdl::KdlDocument;
use lexopt::Parser;
use miette::{Context, Diagnostic, IntoDiagnostic};
use std::{
    ffi::OsString,
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Debug)]
struct CliError {
    cause: lexopt::Error,
    exename: OsString,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse cli args.")
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

impl Diagnostic for CliError {
    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(help(&self.exename)))
    }
}

struct Args {
    builder: HtmlEmitterBuilder,
    input_filename: PathBuf,
    output_filename: Option<PathBuf>,
}

impl Args {
    fn parse(args: Vec<OsString>) -> Result<Args, lexopt::Error> {
        use lexopt::prelude::*;

        let mut parser = Parser::from_args(args);
        let mut builder = HtmlEmitter::builder();
        let mut input_filename = None;
        let mut output_filename = None;
        while let Some(arg) = parser.next()? {
            match arg {
                Long("minify")|Short('m') => drop(builder.minify()),
                Value(value) if input_filename.is_none() => input_filename = Some(PathBuf::from(value)),
                Value(value)  => output_filename = Some(PathBuf::from(value)),
                _ => return Err(arg.unexpected())
            }
        }

        Ok({
            Args { builder, input_filename: input_filename.ok_or("Missing input filename")?, output_filename }
        })
    }
}

fn help(exename: &OsString) -> String {
    format!(
        include_str!("help.txt"),
        Path::new(exename)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    )
}

fn main() -> miette::Result<()> {
    let mut args: Vec<_> = std::env::args_os().collect();
    let exename = args.remove(0);

    if args.iter().map(OsString::as_os_str).any(|arg| arg == "-h" || arg == "--help") {
        println!("{}", help(&exename));
        return Ok(())
    }

    let Args {
        builder,
        input_filename,
        output_filename,
    } = Args::parse(args).map_err(|cause| CliError { exename, cause })?;

    let contents = std::fs::read_to_string(&input_filename)
        .into_diagnostic()
        .with_context(|| format!("Could not open file {}.", input_filename.display()))?;
    let doc = contents.parse::<KdlDocument>()?;
    let file = std::fs::File::create(
        output_filename.unwrap_or_else(|| input_filename.with_extension("html")),
    )
    .into_diagnostic()?;
    let mut file = BufWriter::new(file);
    let mut emitter = builder.build(&doc);
    emitter.emit(&mut file).into_diagnostic()?;
    Ok(())
}
