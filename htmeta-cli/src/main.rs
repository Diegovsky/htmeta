use std::{io::BufWriter, path::{Path, PathBuf}};
use htmeta::HtmlEmitter;
use kdl::KdlDocument;
use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() <= 1 {
        eprintln!(
            "USAGE: {} <input.kdl> [output.html]",
            Path::new(&args[0])
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        return Ok(());
    }
    let filename = Path::new(&args[1]);
    let contents = std::fs::read_to_string(filename).into_diagnostic()?;
    let doc = contents.parse::<KdlDocument>()?;
    let file = std::fs::File::create(
        args.get(2)
            .map(PathBuf::from)
            .unwrap_or_else(|| filename.with_extension("html")),
    )
    .into_diagnostic()?;
    let mut file = BufWriter::new(file);
    let mut emitter = HtmlEmitter::new(&doc, 4);
    emitter.emit(&mut file).into_diagnostic()?;
    Ok(())
}
