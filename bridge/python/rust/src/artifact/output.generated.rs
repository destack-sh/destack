// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactVersion, ContentId, Module, ProductId, TargetId};

/// Build distribution profile crossing bridge boundaries.
#[pyclass(name = "BuildProfile", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BuildProfile {
    pub(crate) value: bridge::BuildProfile,
}

#[pymethods]
impl BuildProfile {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::BuildProfile::Full => "full",
            bridge::BuildProfile::Minimal => "minimal",
            bridge::BuildProfile::Freestanding => "freestanding",
        }
    }
}

impl BuildProfile {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::BuildProfile) -> Self {
        Self { value }
    }
}

/// Build payload linkage crossing bridge boundaries.
#[pyclass(name = "BuildLinkage", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BuildLinkage {
    pub(crate) value: bridge::BuildLinkage,
}

#[pymethods]
impl BuildLinkage {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::BuildLinkage::Portable => "portable",
            bridge::BuildLinkage::Static => "static",
            bridge::BuildLinkage::Dynamic => "dynamic",
        }
    }
}

impl BuildLinkage {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::BuildLinkage) -> Self {
        Self { value }
    }
}

/// Emitted artifact family crossing bridge boundaries.
#[pyclass(name = "EmitFormat", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct EmitFormat {
    pub(crate) value: bridge::EmitFormat,
}

#[pymethods]
impl EmitFormat {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::EmitFormat::Js => "js",
            bridge::EmitFormat::Ts => "ts",
            bridge::EmitFormat::Wasm => "wasm",
            bridge::EmitFormat::Native => "native",
        }
    }
}

impl EmitFormat {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::EmitFormat) -> Self {
        Self { value }
    }
}

/// Source file type crossing bridge boundaries.
#[pyclass(name = "FileType", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileType {
    pub(crate) value: bridge::FileType,
}

#[pymethods]
impl FileType {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::FileType::Destack => "destack",
            bridge::FileType::DestackDeclaration => "destackDeclaration",
            bridge::FileType::JavaScript => "javaScript",
            bridge::FileType::JavaScriptXml => "javaScriptXml",
            bridge::FileType::TypeScript => "typeScript",
            bridge::FileType::TypeScriptXml => "typeScriptXml",
            bridge::FileType::TypeScriptDeclaration => "typeScriptDeclaration",
            bridge::FileType::Text => "text",
            bridge::FileType::Toml => "toml",
            bridge::FileType::Yaml => "yaml",
            bridge::FileType::Json => "json",
            bridge::FileType::Env => "env",
            bridge::FileType::Html => "html",
            bridge::FileType::Markdown => "markdown",
            bridge::FileType::Css => "css",
            bridge::FileType::Svg => "svg",
            bridge::FileType::Wasm => "wasm",
            bridge::FileType::Node => "node",
            bridge::FileType::SourceMap => "sourceMap",
            bridge::FileType::Object => "object",
            bridge::FileType::Image => "image",
            bridge::FileType::Font => "font",
            bridge::FileType::Audio => "audio",
            bridge::FileType::Video => "video",
            bridge::FileType::Model => "model",
            bridge::FileType::Neural => "neural",
            bridge::FileType::Document => "document",
            bridge::FileType::Binary => "binary",
            bridge::FileType::Unknown => "unknown",
        }
    }
}

impl FileType {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileType) -> Self {
        Self { value }
    }
}

/// One emitted or linked source map crossing bridge boundaries.
#[pyclass(name = "SourceMapSource", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SourceMapSource {
    pub(crate) value: bridge::SourceMapSource,
}

#[pymethods]
impl SourceMapSource {
    /// The mapped source names.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// The embedded source contents when they exist.
    #[getter]
    pub fn content(&self) -> Option<String> {
        self.value.content.clone()
    }
}

#[allow(dead_code)]
impl SourceMapSource {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::SourceMapSource) -> Self {
        Self { value }
    }
}

/// One emitted or linked source map crossing bridge boundaries.
#[pyclass(name = "SourceMap", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SourceMap {
    pub(crate) value: bridge::SourceMap,
}

#[pymethods]
impl SourceMap {
    /// The source map version.
    #[getter]
    pub fn version(&self) -> u32 {
        self.value.version
    }

    /// The emitted file name when one exists.
    #[getter]
    pub fn file(&self) -> Option<String> {
        self.value.file.clone()
    }

    /// The source root when one exists.
    #[getter]
    pub fn source_root(&self) -> Option<String> {
        self.value.source_root.clone()
    }

    /// The mapped sources.
    #[getter]
    pub fn sources(&self) -> Vec<SourceMapSource> {
        self.value
            .sources
            .clone()
            .into_iter()
            .map(SourceMapSource::from_bridge)
            .collect()
    }

    /// The recorded symbol names.
    #[getter]
    pub fn names(&self) -> Vec<String> {
        self.value.names.clone()
    }

    /// The VLQ mapping payload.
    #[getter]
    pub fn mappings(&self) -> String {
        self.value.mappings.clone()
    }

    /// The debug id when one exists.
    #[getter]
    pub fn debug_id(&self) -> Option<String> {
        self.value.debug_id.clone()
    }
}

#[allow(dead_code)]
impl SourceMap {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::SourceMap) -> Self {
        Self { value }
    }
}

/// One emitted declaration crossing bridge boundaries.
#[pyclass(name = "Declaration", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Declaration {
    pub(crate) value: bridge::Declaration,
}

#[pymethods]
impl Declaration {
    /// The declaration text.
    #[getter]
    pub fn text(&self) -> String {
        self.value.text.clone()
    }
}

#[allow(dead_code)]
impl Declaration {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Declaration) -> Self {
        Self { value }
    }
}

/// Structured script language crossing bridge boundaries.
#[pyclass(name = "ScriptLanguage", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ScriptLanguage {
    pub(crate) value: bridge::ScriptLanguage,
}

#[pymethods]
impl ScriptLanguage {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::ScriptLanguage::JavaScript => "javaScript",
            bridge::ScriptLanguage::TypeScript => "typeScript",
        }
    }
}

impl ScriptLanguage {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ScriptLanguage) -> Self {
        Self { value }
    }
}

/// One structured script artifact crossing bridge boundaries.
#[pyclass(name = "Script", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Script {
    pub(crate) value: bridge::Script,
}

#[pymethods]
impl Script {
    /// The target language of this script.
    #[getter]
    pub fn language(&self) -> ScriptLanguage {
        ScriptLanguage::from_bridge(self.value.language)
    }

    /// The emitted declaration when one exists.
    #[getter]
    pub fn declaration(&self) -> Option<Declaration> {
        self.value.declaration.clone().map(Declaration::from_bridge)
    }

    /// The source map when one exists.
    #[getter]
    pub fn map(&self) -> Option<SourceMap> {
        self.value.map.clone().map(SourceMap::from_bridge)
    }

    /// Whether this script has top-level side effects.
    #[getter]
    pub fn has_top_level_side_effects(&self) -> bool {
        self.value.has_top_level_side_effects
    }
}

#[allow(dead_code)]
impl Script {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Script) -> Self {
        Self { value }
    }
}

/// Compiled-code object format crossing bridge boundaries.
#[pyclass(name = "ObjectFormat", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ObjectFormat {
    pub(crate) value: bridge::ObjectFormat,
}

#[pymethods]
impl ObjectFormat {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::ObjectFormat::Object => "object",
            bridge::ObjectFormat::Wasm => "wasm",
        }
    }
}

impl ObjectFormat {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ObjectFormat) -> Self {
        Self { value }
    }
}

/// One compiled-code object artifact crossing bridge boundaries.
#[pyclass(name = "Object", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Object {
    pub(crate) value: bridge::Object,
}

#[pymethods]
impl Object {
    /// The compiled-code object format.
    #[getter]
    pub fn format(&self) -> ObjectFormat {
        ObjectFormat::from_bridge(self.value.format)
    }

    /// The encoded object content identity.
    #[getter]
    pub fn content(&self) -> ContentId {
        ContentId::from_bridge(self.value.content.clone())
    }

    /// The source map when one exists.
    #[getter]
    pub fn map(&self) -> Option<SourceMap> {
        self.value.map.clone().map(SourceMap::from_bridge)
    }
}

#[allow(dead_code)]
impl Object {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Object) -> Self {
        Self { value }
    }
}

/// One opaque asset artifact crossing bridge boundaries.
#[pyclass(name = "Asset", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Asset {
    pub(crate) value: bridge::Asset,
}

#[pymethods]
impl Asset {
    /// The asset file type.
    #[getter]
    pub fn file_type(&self) -> FileType {
        FileType::from_bridge(self.value.file_type)
    }

    /// The asset content identity.
    #[getter]
    pub fn content(&self) -> ContentId {
        ContentId::from_bridge(self.value.content.clone())
    }

    /// The source module URI when one exists.
    #[getter]
    pub fn source(&self) -> Option<String> {
        self.value.source.clone()
    }

    /// The source map when one exists.
    #[getter]
    pub fn map(&self) -> Option<SourceMap> {
        self.value.map.clone().map(SourceMap::from_bridge)
    }
}

#[allow(dead_code)]
impl Asset {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Asset) -> Self {
        Self { value }
    }
}

/// One target-built toolchain payload crossing bridge boundaries.
#[pyclass(name = "Build", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Build {
    pub(crate) value: bridge::Build,
}

#[pymethods]
impl Build {
    /// The build distribution profile.
    #[getter]
    pub fn profile(&self) -> BuildProfile {
        BuildProfile::from_bridge(self.value.profile)
    }

    /// The build linkage.
    #[getter]
    pub fn linkage(&self) -> BuildLinkage {
        BuildLinkage::from_bridge(self.value.linkage)
    }

    /// The encoded build content.
    #[getter]
    pub fn content(&self) -> ContentId {
        ContentId::from_bridge(self.value.content.clone())
    }
}

#[allow(dead_code)]
impl Build {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Build) -> Self {
        Self { value }
    }
}

/// One section of a linked bundle crossing bridge boundaries.
#[pyclass(name = "BundleSection", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BundleSection {
    pub(crate) value: bridge::BundleSection,
}

#[pymethods]
impl BundleSection {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::BundleSection::Module => "module",
            bridge::BundleSection::Entry => "entry",
            bridge::BundleSection::Declaration => "declaration",
            bridge::BundleSection::Asset => "asset",
            bridge::BundleSection::Manifest => "manifest",
            bridge::BundleSection::SourceMap => "sourceMap",
            bridge::BundleSection::Native => "native",
        }
    }
}

impl BundleSection {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::BundleSection) -> Self {
        Self { value }
    }
}

/// Bundle assembly mode crossing bridge boundaries.
#[pyclass(name = "BundleMode", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BundleMode {
    pub(crate) value: bridge::BundleMode,
}

#[pymethods]
impl BundleMode {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::BundleMode::PreserveModules => "preserveModules",
            bridge::BundleMode::SingleFile => "singleFile",
            bridge::BundleMode::Chunked => "chunked",
        }
    }
}

impl BundleMode {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::BundleMode) -> Self {
        Self { value }
    }
}

/// One derived bundle file crossing bridge boundaries.
#[pyclass(name = "BundleFile", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BundleFile {
    pub(crate) value: bridge::BundleFile,
}

#[pymethods]
impl BundleFile {
    /// The bundle section this file belongs to.
    #[getter]
    pub fn section(&self) -> BundleSection {
        BundleSection::from_bridge(self.value.section)
    }

    /// The output URI.
    #[getter]
    pub fn uri(&self) -> String {
        self.value.uri.clone()
    }

    /// The emitted file type.
    #[getter]
    pub fn file_type(&self) -> FileType {
        FileType::from_bridge(self.value.file_type)
    }

    /// The output content identity.
    #[getter]
    pub fn content(&self) -> ContentId {
        ContentId::from_bridge(self.value.content.clone())
    }

    /// The related source URI when one exists.
    #[getter]
    pub fn source(&self) -> Option<String> {
        self.value.source.clone()
    }
}

#[allow(dead_code)]
impl BundleFile {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::BundleFile) -> Self {
        Self { value }
    }
}

/// One linked file graph crossing bridge boundaries.
#[pyclass(name = "Bundle", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Bundle {
    pub(crate) value: bridge::Bundle,
}

#[pymethods]
impl Bundle {
    /// The emitted artifact family.
    #[getter]
    pub fn emit(&self) -> EmitFormat {
        EmitFormat::from_bridge(self.value.emit)
    }

    /// The target-level assembly mode.
    #[getter]
    pub fn mode(&self) -> BundleMode {
        BundleMode::from_bridge(self.value.mode)
    }

    /// The files in this bundle.
    #[getter]
    pub fn files(&self) -> Vec<BundleFile> {
        self.value
            .files
            .clone()
            .into_iter()
            .map(BundleFile::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl Bundle {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Bundle) -> Self {
        Self { value }
    }
}

/// Program executable format crossing bridge boundaries.
#[pyclass(name = "ProgramFormat", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ProgramFormat {
    pub(crate) value: bridge::ProgramFormat,
}

#[pymethods]
impl ProgramFormat {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::ProgramFormat::Vm => "vm",
            bridge::ProgramFormat::Native => "native",
        }
    }
}

impl ProgramFormat {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ProgramFormat) -> Self {
        Self { value }
    }
}

/// Durable program header crossing bridge boundaries.
#[pyclass(name = "ProgramHeader", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ProgramHeader {
    pub(crate) value: bridge::ProgramHeader,
}

#[pymethods]
impl ProgramHeader {
    /// Human-facing program name.
    #[getter]
    pub fn name(&self) -> Option<String> {
        self.value.name.clone()
    }

    /// Build fingerprint that produced this program.
    #[getter]
    pub fn fingerprint(&self) -> Option<String> {
        self.value.fingerprint.clone()
    }

    /// Target triple or equivalent target identity.
    #[getter]
    pub fn target(&self) -> Option<String> {
        self.value.target.clone()
    }
}

#[allow(dead_code)]
impl ProgramHeader {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ProgramHeader) -> Self {
        Self { value }
    }
}

/// Durable executable program crossing bridge boundaries.
#[pyclass(name = "Program", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Program {
    pub(crate) value: bridge::Program,
}

#[pymethods]
impl Program {
    /// The program identity and compatibility header.
    #[getter]
    pub fn header(&self) -> ProgramHeader {
        ProgramHeader::from_bridge(self.value.header.clone())
    }

    /// The executable format.
    #[getter]
    pub fn format(&self) -> ProgramFormat {
        ProgramFormat::from_bridge(self.value.format)
    }

    /// Content blobs referenced by the executable payload.
    #[getter]
    pub fn contents(&self) -> Vec<ContentId> {
        self.value
            .contents
            .clone()
            .into_iter()
            .map(ContentId::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl Program {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Program) -> Self {
        Self { value }
    }
}

/// Semantic runtime contract crossing bridge boundaries.
#[pyclass(name = "Runtime", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Runtime {
    pub(crate) value: bridge::Runtime,
}

#[pymethods]
impl Runtime {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::Runtime::Destack => "destack",
            bridge::Runtime::Js => "js",
        }
    }
}

impl Runtime {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Runtime) -> Self {
        Self { value }
    }
}

/// Host environment crossing bridge boundaries.
#[pyclass(name = "Host", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Host {
    pub(crate) value: bridge::Host,
}

#[pymethods]
impl Host {
    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::Host::Native => "native",
            bridge::Host::Browser => "browser",
            bridge::Host::Wasi => "wasi",
            bridge::Host::Emscripten => "emscripten",
            bridge::Host::Freestanding => "freestanding",
        }
    }
}

impl Host {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Host) -> Self {
        Self { value }
    }
}

/// One linked product target crossing bridge boundaries.
#[pyclass(name = "ProductTarget", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ProductTarget {
    pub(crate) value: bridge::ProductTarget,
}

#[pymethods]
impl ProductTarget {
    /// The configured product target name.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// The repository target assembled into this product.
    #[getter]
    pub fn target(&self) -> TargetId {
        TargetId::from_bridge(self.value.target.clone())
    }

    /// The runtime contract this target expects.
    #[getter]
    pub fn runtime(&self) -> Runtime {
        Runtime::from_bridge(self.value.runtime)
    }

    /// The host environment this target expects.
    #[getter]
    pub fn host(&self) -> Host {
        Host::from_bridge(self.value.host)
    }

    /// The platform this target expects.
    #[getter]
    pub fn platform(&self) -> String {
        self.value.platform.clone()
    }

    /// Whether this product target includes its toolchain build payload.
    #[getter]
    pub fn includes_build(&self) -> bool {
        self.value.includes_build
    }

    /// Whether this product target includes its linked bundle.
    #[getter]
    pub fn includes_bundle(&self) -> bool {
        self.value.includes_bundle
    }

    /// Whether this product target includes its executable program.
    #[getter]
    pub fn includes_program(&self) -> bool {
        self.value.includes_program
    }
}

#[allow(dead_code)]
impl ProductTarget {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ProductTarget) -> Self {
        Self { value }
    }
}

/// One linked product crossing bridge boundaries.
#[pyclass(name = "Product", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Product {
    pub(crate) value: bridge::Product,
}

#[pymethods]
impl Product {
    /// The configured product name.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// The linked targets in deterministic order.
    #[getter]
    pub fn targets(&self) -> Vec<ProductTarget> {
        self.value
            .targets
            .clone()
            .into_iter()
            .map(ProductTarget::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl Product {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Product) -> Self {
        Self { value }
    }
}

/// Module build output family.
#[pyclass(name = "ModuleBuildKind", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ModuleBuildKind {
    pub(crate) value: bridge::ModuleBuildKind,
}

#[pymethods]
impl ModuleBuildKind {
    /// Build the structured script artifact.
    #[staticmethod]
    pub fn script() -> Self {
        Self {
            value: bridge::ModuleBuildKind::Script,
        }
    }

    /// Build the compiled object artifact.
    #[staticmethod]
    pub fn object() -> Self {
        Self {
            value: bridge::ModuleBuildKind::Object,
        }
    }

    /// Build the opaque asset artifact.
    #[staticmethod]
    pub fn asset() -> Self {
        Self {
            value: bridge::ModuleBuildKind::Asset,
        }
    }

    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::ModuleBuildKind::Script => "script",
            bridge::ModuleBuildKind::Object => "object",
            bridge::ModuleBuildKind::Asset => "asset",
        }
    }
}

impl ModuleBuildKind {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ModuleBuildKind {
        self.value
    }
}

/// One language build request.
#[pyclass(name = "BuildRequest", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub(crate) value: bridge::BuildRequest,
}

#[pymethods]
impl BuildRequest {
    /// Build one module artifact.
    #[staticmethod]
    pub fn module(module: Module, target: TargetId, output: ModuleBuildKind) -> Self {
        Self {
            value: bridge::BuildRequest::Module {
                module: module.into_bridge(),
                target: target.into_bridge(),
                output: output.into_bridge(),
            },
        }
    }

    /// Build one target build payload.
    #[staticmethod]
    pub fn build(target: TargetId) -> Self {
        Self {
            value: bridge::BuildRequest::Build {
                target: target.into_bridge(),
            },
        }
    }

    /// Build one package target.
    #[staticmethod]
    pub fn target(target: TargetId) -> Self {
        Self {
            value: bridge::BuildRequest::Target {
                target: target.into_bridge(),
            },
        }
    }

    /// Build one product.
    #[staticmethod]
    pub fn product(product: ProductId) -> Self {
        Self {
            value: bridge::BuildRequest::Product {
                product: product.into_bridge(),
            },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::BuildRequest::Module { .. } => "module",
            bridge::BuildRequest::Build { .. } => "build",
            bridge::BuildRequest::Target { .. } => "target",
            bridge::BuildRequest::Product { .. } => "product",
        }
    }
}

impl BuildRequest {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::BuildRequest {
        self.value
    }
}

/// One language build output.
#[pyclass(name = "BuildOutput", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub(crate) value: bridge::BuildOutput,
}

#[pymethods]
impl BuildOutput {
    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::BuildOutput::Script { .. } => "script",
            bridge::BuildOutput::Object { .. } => "object",
            bridge::BuildOutput::Asset { .. } => "asset",
            bridge::BuildOutput::Build { .. } => "build",
            bridge::BuildOutput::Bundle { .. } => "bundle",
            bridge::BuildOutput::Program { .. } => "program",
            bridge::BuildOutput::Product { .. } => "product",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_asset(&self) -> Option<Asset> {
        match &self.value {
            bridge::BuildOutput::Asset { asset, .. } => Some(Asset::from_bridge(asset.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_build(&self) -> Option<Build> {
        match &self.value {
            bridge::BuildOutput::Build { build, .. } => Some(Build::from_bridge(build.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_bundle(&self) -> Option<Bundle> {
        match &self.value {
            bridge::BuildOutput::Bundle { bundle, .. } => Some(Bundle::from_bridge(bundle.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_object(&self) -> Option<Object> {
        match &self.value {
            bridge::BuildOutput::Object { object, .. } => Some(Object::from_bridge(object.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_product(&self) -> Option<Product> {
        match &self.value {
            bridge::BuildOutput::Product { product, .. } => {
                Some(Product::from_bridge(product.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_program(&self) -> Option<Program> {
        match &self.value {
            bridge::BuildOutput::Program { program, .. } => {
                Some(Program::from_bridge(program.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_script(&self) -> Option<Script> {
        match &self.value {
            bridge::BuildOutput::Script { script, .. } => Some(Script::from_bridge(script.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_version(&self) -> Option<ArtifactVersion> {
        match &self.value {
            bridge::BuildOutput::Script { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            bridge::BuildOutput::Object { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            bridge::BuildOutput::Asset { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            bridge::BuildOutput::Build { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            bridge::BuildOutput::Bundle { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            bridge::BuildOutput::Program { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            bridge::BuildOutput::Product { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
        }
    }
}

impl BuildOutput {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::BuildOutput) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<BuildProfile>()?;
    module.add_class::<BuildLinkage>()?;
    module.add_class::<EmitFormat>()?;
    module.add_class::<FileType>()?;
    module.add_class::<SourceMapSource>()?;
    module.add_class::<SourceMap>()?;
    module.add_class::<Declaration>()?;
    module.add_class::<ScriptLanguage>()?;
    module.add_class::<Script>()?;
    module.add_class::<ObjectFormat>()?;
    module.add_class::<Object>()?;
    module.add_class::<Asset>()?;
    module.add_class::<Build>()?;
    module.add_class::<BundleSection>()?;
    module.add_class::<BundleMode>()?;
    module.add_class::<BundleFile>()?;
    module.add_class::<Bundle>()?;
    module.add_class::<ProgramFormat>()?;
    module.add_class::<ProgramHeader>()?;
    module.add_class::<Program>()?;
    module.add_class::<Runtime>()?;
    module.add_class::<Host>()?;
    module.add_class::<ProductTarget>()?;
    module.add_class::<Product>()?;
    module.add_class::<ModuleBuildKind>()?;
    module.add_class::<BuildRequest>()?;
    module.add_class::<BuildOutput>()?;
    Ok(())
}
