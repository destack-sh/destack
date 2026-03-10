//! Types of Language Server Index Format (LSIF). LSIF is a standard format
//! for language servers or other programming tools to dump their knowledge
//! about a workspace.
//!
//! Based on <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>

use crate::{Range, Uri};
use serde::{Deserialize, Serialize};

/// An identifier used in LSIF, can be either a number or a string.
pub type Id = crate::NumberOrString;

/// A location or a range ID reference.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocationOrRangeId {
    /// A full location with URI and range.
    Location(crate::Location),
    /// A reference to a range by its ID.
    RangeId(Id),
}

/// An LSIF entry, which is a vertex or edge with an ID.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// The unique identifier for this entry.
    pub id: Id,
    /// The element data (vertex or edge).
    #[serde(flatten)]
    pub data: Element,
}

/// An LSIF element, either a vertex or an edge.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum Element {
    /// A vertex in the LSIF graph.
    Vertex(Vertex),
    /// An edge in the LSIF graph.
    Edge(Edge),
}

/// Information about the tool that created the LSIF dump.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolInfo {
    /// The name of the tool.
    pub name: String,
    /// Command line arguments passed to the tool.
    #[serde(default = "Default::default")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// The version of the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// The encoding used for positions and ranges in LSIF.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub enum Encoding {
    /// Currently only 'utf-16' is supported due to the limitations in LSP.
    #[serde(rename = "utf-16")]
    Utf16,
}

/// A document symbol represented by a range ID.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RangeBasedDocumentSymbol {
    /// The ID of the range representing this symbol.
    pub id: Id,
    /// Child symbols nested within this symbol.
    #[serde(default = "Default::default")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<RangeBasedDocumentSymbol>,
}

/// Document symbols as either full symbols or range-based references.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum DocumentSymbolOrRangeBasedVec {
    /// Full document symbol information.
    DocumentSymbol(Vec<crate::DocumentSymbol>),
    /// Range-based document symbol references.
    RangeBased(Vec<RangeBasedDocumentSymbol>),
}

/// A tag for definition ranges containing symbol information.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionTag {
    /// The text covered by the range.
    text: String,
    /// The symbol kind.
    kind: crate::SymbolKind,
    /// Indicates if this symbol is deprecated.
    #[serde(default)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    deprecated: bool,
    /// The full range of the definition not including leading/trailing whitespace but everything else, e.g comments and code.
    /// The range must be included in fullRange.
    full_range: Range,
    /// Optional detail information for the definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// A tag for declaration ranges containing symbol information.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationTag {
    /// The text covered by the range.
    text: String,
    /// The symbol kind.
    kind: crate::SymbolKind,
    /// Indicates if this symbol is deprecated.
    #[serde(default)]
    deprecated: bool,
    /// The full range of the definition not including leading/trailing whitespace but everything else, e.g comments and code.
    /// The range must be included in fullRange.
    full_range: Range,
    /// Optional detail information for the definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// A tag for reference ranges.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceTag {
    /// The text covered by the range.
    text: String,
}

/// A tag for ranges with unknown semantics.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownTag {
    /// The text covered by the range.
    text: String,
}

/// A tag that describes the semantics of a range.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum RangeTag {
    /// The range represents a definition.
    Definition(DefinitionTag),
    /// The range represents a declaration.
    Declaration(DeclarationTag),
    /// The range represents a reference.
    Reference(ReferenceTag),
    /// The range has unknown semantics.
    Unknown(UnknownTag),
}

/// A vertex in the LSIF graph representing various language entities.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "label")]
pub enum Vertex {
    /// Metadata about the LSIF dump.
    MetaData(MetaData),
    /// <https://github.com/Microsoft/language-server-protocol/blob/master/indexFormat/specification.md#the-project-vertex>
    Project(Project),
    /// A document in the project.
    Document(Document),
    /// <https://github.com/Microsoft/language-server-protocol/blob/master/indexFormat/specification.md#ranges>
    Range {
        /// The range in the document.
        #[serde(flatten)]
        range: Range,
        /// Optional tag describing the range semantics.
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<RangeTag>,
    },
    /// <https://github.com/Microsoft/language-server-protocol/blob/master/indexFormat/specification.md#result-set>
    ResultSet(ResultSet),
    /// A moniker for cross-repository navigation.
    Moniker(crate::Moniker),
    /// Information about a package.
    PackageInformation(PackageInformation),

    /// An event marking the start or end of a scope.
    #[serde(rename = "$event")]
    Event(Event),

    /// A definition result vertex.
    DefinitionResult,
    /// A declaration result vertex.
    DeclarationResult,
    /// A type definition result vertex.
    TypeDefinitionResult,
    /// A reference result vertex.
    ReferenceResult,
    /// An implementation result vertex.
    ImplementationResult,
    /// Folding range results for a document.
    FoldingRangeResult {
        /// The folding ranges.
        result: Vec<crate::FoldingRange>,
    },
    /// Hover information result.
    HoverResult {
        /// The hover content.
        result: crate::Hover,
    },
    /// Document symbol results.
    DocumentSymbolResult {
        /// The document symbols.
        result: DocumentSymbolOrRangeBasedVec,
    },
    /// Document link results.
    DocumentLinkResult {
        /// The document links.
        result: Vec<crate::DocumentLink>,
    },
    /// Diagnostic results for a document.
    DiagnosticResult {
        /// The diagnostics.
        result: Vec<crate::Diagnostic>,
    },
}

/// The kind of an LSIF event.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    /// The beginning of a scope.
    Begin,
    /// The end of a scope.
    End,
}

/// The scope of an LSIF event.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventScope {
    /// Document-level scope.
    Document,
    /// Project-level scope.
    Project,
}

/// An event marking the start or end of a document or project scope.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// The kind of event (begin or end).
    pub kind: EventKind,
    /// The scope of the event (document or project).
    pub scope: EventScope,
    /// The ID of the document or project this event refers to.
    pub data: Id,
}

/// An edge in the LSIF graph connecting vertices.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "label")]
pub enum Edge {
    /// A contains edge linking a document to its ranges.
    Contains(EdgeDataMultiIn),
    /// A moniker edge linking a range to its moniker.
    Moniker(EdgeData),
    /// A next moniker edge for moniker chaining.
    NextMoniker(EdgeData),
    /// A next edge linking a range to a result set.
    Next(EdgeData),
    /// A package information edge.
    PackageInformation(EdgeData),
    /// An item edge for reference results.
    Item(Item),

    // method edges
    /// A definition edge.
    #[serde(rename = "textDocument/definition")]
    Definition(EdgeData),
    /// A declaration edge.
    #[serde(rename = "textDocument/declaration")]
    Declaration(EdgeData),
    /// A hover edge.
    #[serde(rename = "textDocument/hover")]
    Hover(EdgeData),
    /// A references edge.
    #[serde(rename = "textDocument/references")]
    References(EdgeData),
    /// An implementation edge.
    #[serde(rename = "textDocument/implementation")]
    Implementation(EdgeData),
    /// A type definition edge.
    #[serde(rename = "textDocument/typeDefinition")]
    TypeDefinition(EdgeData),
    /// A folding range edge.
    #[serde(rename = "textDocument/foldingRange")]
    FoldingRange(EdgeData),
    /// A document link edge.
    #[serde(rename = "textDocument/documentLink")]
    DocumentLink(EdgeData),
    /// A document symbol edge.
    #[serde(rename = "textDocument/documentSymbol")]
    DocumentSymbol(EdgeData),
    /// A diagnostic edge.
    #[serde(rename = "textDocument/diagnostic")]
    Diagnostic(EdgeData),
}

/// Edge data for edges with a single inbound vertex.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeData {
    /// The ID of the inbound vertex.
    pub in_v: Id,
    /// The ID of the outbound vertex.
    pub out_v: Id,
}

/// Edge data for edges with multiple inbound vertices.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDataMultiIn {
    /// The IDs of the inbound vertices.
    pub in_vs: Vec<Id>,
    /// The ID of the outbound vertex.
    pub out_v: Id,
}

/// The type of a definition result.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DefinitionResultType {
    /// A single definition location.
    Scalar(LocationOrRangeId),
    /// Multiple definition locations.
    Array(LocationOrRangeId),
}

/// The kind of items in an item edge.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    /// Declaration items.
    Declarations,
    /// Definition items.
    Definitions,
    /// Reference items.
    References,
    /// Reference result items.
    ReferenceResults,
    /// Implementation result items.
    ImplementationResults,
}

/// An item edge linking reference results to their locations.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// The document containing the items.
    pub document: Id,
    /// The kind of items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<ItemKind>,
    /// The edge data with multiple inbound vertices.
    #[serde(flatten)]
    pub edge_data: EdgeDataMultiIn,
}

/// A document vertex representing a source file.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    /// The URI of the document.
    pub uri: Uri,
    /// The language identifier of the document.
    pub language_id: String,
}

/// A result set vertex that groups related results.
///
/// <https://github.com/Microsoft/language-server-protocol/blob/master/indexFormat/specification.md#result-set>
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultSet {
    /// An optional key for the result set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// A project vertex representing a code project.
///
/// <https://github.com/Microsoft/language-server-protocol/blob/master/indexFormat/specification.md#the-project-vertex>
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// The resource URI of the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<Uri>,
    /// Optional content of the project file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The kind of the project (e.g., "typescript").
    pub kind: String,
}

/// Metadata about the LSIF dump.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaData {
    /// The version of the LSIF format using semver notation. See <https://semver.org/>. Please note
    /// the version numbers starting with 0 don't adhere to semver and adopters have to assume
    /// that each new version is breaking.
    pub version: String,

    /// The project root (in form of an URI) used to compute this dump.
    pub project_root: Uri,

    /// The string encoding used to compute line and character values in
    /// positions and ranges.
    pub position_encoding: Encoding,

    /// Information about the tool that created the dump.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_info: Option<ToolInfo>,
}

/// Repository information for a package.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    /// The type of repository (e.g., "git").
    pub r#type: String,
    /// The URL of the repository.
    pub url: String,
    /// The commit ID if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,
}

/// Information about a package for cross-repository navigation.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInformation {
    /// The name of the package.
    pub name: String,
    /// The package manager (e.g., "npm").
    pub manager: String,
    /// The URI of the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<Uri>,
    /// Optional content of the package descriptor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Repository information for the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,
    /// The version of the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
