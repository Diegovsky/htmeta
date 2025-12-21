///! A collection of useful trinkets for plugins.

/// A [`std::io::Write`]r that doesn't output anything.
///
/// Equivalent to `/dev/null` but multiplatform.
///
/// This can be useful for plugins that need to execute a file but not
/// emit its contents.
#[derive(Clone, Copy,Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NilWriter;

impl std::io::Write for NilWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
