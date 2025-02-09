use htmeta::{kdl, HtmlEmitter, HtmlEmitterBuilder};
use kdl::KdlDocument;
use lexopt::Parser;
use miette::{Context, Diagnostic, IntoDiagnostic};
use std::{
    ffi::OsString,
    io::{BufWriter, Read, Write},
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
        #[cfg(feature = "templates")]
        builder.add_plugin(htmeta_template::TemplatePlugin::default());
        let mut input_filename = None;
        let mut output_filename = None;
        while let Some(arg) = parser.next()? {
            match arg {
                Long("minify") | Short('m') => drop(builder.minify()),
                Long("tab-size") | Short('t') => drop(builder.indent(parser.value()?.parse()?)),
                Long("document-formatting") | Short('D') => drop(builder.follow_original_indent()),
                Value(value) if input_filename.is_none() => {
                    input_filename = Some(PathBuf::from(value))
                }
                Value(value) => output_filename = Some(PathBuf::from(value)),
                _ => return Err(arg.unexpected()),
            }
        }

        Ok({
            Args {
                builder,
                input_filename: input_filename.ok_or("Missing input filename")?,
                output_filename,
            }
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

    if args
        .iter()
        .map(OsString::as_os_str)
        .any(|arg| arg == "-h" || arg == "--help")
    {
        println!("{}", help(&exename));
        return Ok(());
    }

    let Args {
        builder,
        input_filename,
        output_filename,
    } = Args::parse(args).map_err(|cause| CliError { exename, cause })?;

    let mut uses_stdin = false;
    let contents = if input_filename == Path::new("-") {
        uses_stdin = true;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .into_diagnostic()?;
        buf
    } else {
        std::fs::read_to_string(&input_filename)
            .into_diagnostic()
            .with_context(|| format!("Could not open file {}.", input_filename.display()))?
    };
    let doc = contents.parse::<KdlDocument>()?;
    let mut emitter = builder.build(input_filename.clone());

    // Dump to stdio
    let mut file: &mut dyn Write = if uses_stdin || output_filename == Some("-".into()) {
        &mut std::io::stdout()
    // Write to file
    } else {
        let file = std::fs::File::create(
            output_filename.unwrap_or_else(|| input_filename.with_extension("html")),
        )
        .into_diagnostic()?;
        &mut BufWriter::new(file)
    };

    emitter.emit(&doc, &mut file).into_diagnostic()?;
    Ok(())
}
