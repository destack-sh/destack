use dyst_source::{FileId, FileType};

#[derive(Debug, Clone)]
pub struct CompilerArtifact {
	/// The file id of the generated artifact.
	pub id: FileId,
	/// The file type of the generated artifact.
	pub ty: FileType,
}