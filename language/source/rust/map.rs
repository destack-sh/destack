use crate::SourceFile;

/// Map of files in the current compilation unit.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
	/// The files in the source map.
	pub files: Vec<SourceFile>,
}
