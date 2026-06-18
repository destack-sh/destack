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
    /// Full Destack build.
    #[staticmethod]
    pub fn full() -> Self {
        Self {
            value: bridge::BuildProfile::Full,
        }
    }

    /// Smaller Destack build with optional services omitted.
    #[staticmethod]
    pub fn minimal() -> Self {
        Self {
            value: bridge::BuildProfile::Minimal,
        }
    }

    /// Freestanding output without the normal Destack runtime contract.
    #[staticmethod]
    pub fn freestanding() -> Self {
        Self {
            value: bridge::BuildProfile::Freestanding,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::BuildProfile {
        self.value
    }

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
    /// Ship a portable Destack payload consumed by a runtime.
    #[staticmethod]
    pub fn portable() -> Self {
        Self {
            value: bridge::BuildLinkage::Portable,
        }
    }

    /// Link the build payload into the produced platform binary.
    #[staticmethod]
    pub fn r#static() -> Self {
        Self {
            value: bridge::BuildLinkage::Static,
        }
    }

    /// Ship the build payload as a dynamic library.
    #[staticmethod]
    pub fn dynamic() -> Self {
        Self {
            value: bridge::BuildLinkage::Dynamic,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::BuildLinkage {
        self.value
    }

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
    /// JavaScript output.
    #[staticmethod]
    pub fn js() -> Self {
        Self {
            value: bridge::EmitFormat::Js,
        }
    }

    /// TypeScript output.
    #[staticmethod]
    pub fn ts() -> Self {
        Self {
            value: bridge::EmitFormat::Ts,
        }
    }

    /// WebAssembly output.
    #[staticmethod]
    pub fn wasm() -> Self {
        Self {
            value: bridge::EmitFormat::Wasm,
        }
    }

    /// Native binary output.
    #[staticmethod]
    pub fn native() -> Self {
        Self {
            value: bridge::EmitFormat::Native,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::EmitFormat {
        self.value
    }

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
    /// `.ds`.
    #[staticmethod]
    pub fn destack() -> Self {
        Self {
            value: bridge::FileType::Destack,
        }
    }

    /// `.d.ds`.
    #[staticmethod]
    pub fn destack_declaration() -> Self {
        Self {
            value: bridge::FileType::DestackDeclaration,
        }
    }

    /// `.js`.
    #[staticmethod]
    pub fn java_script() -> Self {
        Self {
            value: bridge::FileType::JavaScript,
        }
    }

    /// `.jsx`.
    #[staticmethod]
    pub fn java_script_xml() -> Self {
        Self {
            value: bridge::FileType::JavaScriptXml,
        }
    }

    /// `.ts`.
    #[staticmethod]
    pub fn type_script() -> Self {
        Self {
            value: bridge::FileType::TypeScript,
        }
    }

    /// `.tsx`.
    #[staticmethod]
    pub fn type_script_xml() -> Self {
        Self {
            value: bridge::FileType::TypeScriptXml,
        }
    }

    /// `.d.ts`.
    #[staticmethod]
    pub fn type_script_declaration() -> Self {
        Self {
            value: bridge::FileType::TypeScriptDeclaration,
        }
    }

    /// Text file.
    #[staticmethod]
    pub fn text() -> Self {
        Self {
            value: bridge::FileType::Text,
        }
    }

    /// TOML file.
    #[staticmethod]
    pub fn toml() -> Self {
        Self {
            value: bridge::FileType::Toml,
        }
    }

    /// YAML file.
    #[staticmethod]
    pub fn yaml() -> Self {
        Self {
            value: bridge::FileType::Yaml,
        }
    }

    /// JSON file.
    #[staticmethod]
    pub fn json() -> Self {
        Self {
            value: bridge::FileType::Json,
        }
    }

    /// Environment file.
    #[staticmethod]
    pub fn env() -> Self {
        Self {
            value: bridge::FileType::Env,
        }
    }

    /// HTML file.
    #[staticmethod]
    pub fn html() -> Self {
        Self {
            value: bridge::FileType::Html,
        }
    }

    /// Markdown file.
    #[staticmethod]
    pub fn markdown() -> Self {
        Self {
            value: bridge::FileType::Markdown,
        }
    }

    /// CSS file.
    #[staticmethod]
    pub fn css() -> Self {
        Self {
            value: bridge::FileType::Css,
        }
    }

    /// SVG file.
    #[staticmethod]
    pub fn svg() -> Self {
        Self {
            value: bridge::FileType::Svg,
        }
    }

    /// WebAssembly payload.
    #[staticmethod]
    pub fn wasm() -> Self {
        Self {
            value: bridge::FileType::Wasm,
        }
    }

    /// Node native module.
    #[staticmethod]
    pub fn node() -> Self {
        Self {
            value: bridge::FileType::Node,
        }
    }

    /// Source map file.
    #[staticmethod]
    pub fn source_map() -> Self {
        Self {
            value: bridge::FileType::SourceMap,
        }
    }

    /// Native object file.
    #[staticmethod]
    pub fn object() -> Self {
        Self {
            value: bridge::FileType::Object,
        }
    }

    /// Image asset.
    #[staticmethod]
    pub fn image() -> Self {
        Self {
            value: bridge::FileType::Image,
        }
    }

    /// Font asset.
    #[staticmethod]
    pub fn font() -> Self {
        Self {
            value: bridge::FileType::Font,
        }
    }

    /// Audio asset.
    #[staticmethod]
    pub fn audio() -> Self {
        Self {
            value: bridge::FileType::Audio,
        }
    }

    /// Video asset.
    #[staticmethod]
    pub fn video() -> Self {
        Self {
            value: bridge::FileType::Video,
        }
    }

    /// 3D model asset.
    #[staticmethod]
    pub fn model() -> Self {
        Self {
            value: bridge::FileType::Model,
        }
    }

    /// AI model asset.
    #[staticmethod]
    pub fn neural() -> Self {
        Self {
            value: bridge::FileType::Neural,
        }
    }

    /// Document asset.
    #[staticmethod]
    pub fn document() -> Self {
        Self {
            value: bridge::FileType::Document,
        }
    }

    /// Unknown binary file.
    #[staticmethod]
    pub fn binary() -> Self {
        Self {
            value: bridge::FileType::Binary,
        }
    }

    /// Unknown file type.
    #[staticmethod]
    pub fn unknown() -> Self {
        Self {
            value: bridge::FileType::Unknown,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileType {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(name: String, content: Option<String>) -> Self {
        Self {
            value: bridge::SourceMapSource { name, content },
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceMapSource {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(
        version: u32,
        file: Option<String>,
        source_root: Option<String>,
        sources: Vec<SourceMapSource>,
        names: Vec<String>,
        mappings: String,
        debug_id: Option<String>,
    ) -> Self {
        Self {
            value: bridge::SourceMap {
                version,
                file,
                source_root,
                sources: sources.into_iter().map(|item| item.into_bridge()).collect(),
                names,
                mappings,
                debug_id,
            },
        }
    }

    /// The source map version.
    #[getter]
    pub fn version(&self) -> u32 {
        self.value.version.clone()
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
            .map(|item| SourceMapSource::from_bridge(item))
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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceMap {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(text: String) -> Self {
        Self {
            value: bridge::Declaration { text },
        }
    }

    /// The declaration text.
    #[getter]
    pub fn text(&self) -> String {
        self.value.text.clone()
    }
}

#[allow(dead_code)]
impl Declaration {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Declaration {
        self.value
    }

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
    /// JavaScript output.
    #[staticmethod]
    pub fn java_script() -> Self {
        Self {
            value: bridge::ScriptLanguage::JavaScript,
        }
    }

    /// TypeScript output.
    #[staticmethod]
    pub fn type_script() -> Self {
        Self {
            value: bridge::ScriptLanguage::TypeScript,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ScriptLanguage {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(
        language: ScriptLanguage,
        declaration: Option<Declaration>,
        map: Option<SourceMap>,
        has_top_level_side_effects: bool,
    ) -> Self {
        Self {
            value: bridge::Script {
                language: language.into_bridge(),
                declaration: declaration.map(|item| item.into_bridge()),
                map: map.map(|item| item.into_bridge()),
                has_top_level_side_effects,
            },
        }
    }

    /// The target language of this script.
    #[getter]
    pub fn language(&self) -> ScriptLanguage {
        ScriptLanguage::from_bridge(self.value.language.clone())
    }

    /// The emitted declaration when one exists.
    #[getter]
    pub fn declaration(&self) -> Option<Declaration> {
        self.value
            .declaration
            .clone()
            .map(|item| Declaration::from_bridge(item))
    }

    /// The source map when one exists.
    #[getter]
    pub fn map(&self) -> Option<SourceMap> {
        self.value
            .map
            .clone()
            .map(|item| SourceMap::from_bridge(item))
    }

    /// Whether this script has top-level side effects.
    #[getter]
    pub fn has_top_level_side_effects(&self) -> bool {
        self.value.has_top_level_side_effects.clone()
    }
}

#[allow(dead_code)]
impl Script {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Script {
        self.value
    }

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
    /// Native relocatable object file.
    #[staticmethod]
    pub fn object() -> Self {
        Self {
            value: bridge::ObjectFormat::Object,
        }
    }

    /// WebAssembly object or module payload.
    #[staticmethod]
    pub fn wasm() -> Self {
        Self {
            value: bridge::ObjectFormat::Wasm,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ObjectFormat {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(format: ObjectFormat, content: ContentId, map: Option<SourceMap>) -> Self {
        Self {
            value: bridge::Object {
                format: format.into_bridge(),
                content: content.into_bridge(),
                map: map.map(|item| item.into_bridge()),
            },
        }
    }

    /// The compiled-code object format.
    #[getter]
    pub fn format(&self) -> ObjectFormat {
        ObjectFormat::from_bridge(self.value.format.clone())
    }

    /// The encoded object content identity.
    #[getter]
    pub fn content(&self) -> ContentId {
        ContentId::from_bridge(self.value.content.clone())
    }

    /// The source map when one exists.
    #[getter]
    pub fn map(&self) -> Option<SourceMap> {
        self.value
            .map
            .clone()
            .map(|item| SourceMap::from_bridge(item))
    }
}

#[allow(dead_code)]
impl Object {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Object {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(
        file_type: FileType,
        content: ContentId,
        source: Option<String>,
        map: Option<SourceMap>,
    ) -> Self {
        Self {
            value: bridge::Asset {
                file_type: file_type.into_bridge(),
                content: content.into_bridge(),
                source,
                map: map.map(|item| item.into_bridge()),
            },
        }
    }

    /// The asset file type.
    #[getter]
    pub fn file_type(&self) -> FileType {
        FileType::from_bridge(self.value.file_type.clone())
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
        self.value
            .map
            .clone()
            .map(|item| SourceMap::from_bridge(item))
    }
}

#[allow(dead_code)]
impl Asset {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Asset {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(profile: BuildProfile, linkage: BuildLinkage, content: ContentId) -> Self {
        Self {
            value: bridge::Build {
                profile: profile.into_bridge(),
                linkage: linkage.into_bridge(),
                content: content.into_bridge(),
            },
        }
    }

    /// The build distribution profile.
    #[getter]
    pub fn profile(&self) -> BuildProfile {
        BuildProfile::from_bridge(self.value.profile.clone())
    }

    /// The build linkage.
    #[getter]
    pub fn linkage(&self) -> BuildLinkage {
        BuildLinkage::from_bridge(self.value.linkage.clone())
    }

    /// The encoded build content.
    #[getter]
    pub fn content(&self) -> ContentId {
        ContentId::from_bridge(self.value.content.clone())
    }
}

#[allow(dead_code)]
impl Build {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Build {
        self.value
    }

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
    /// Per-module library output.
    #[staticmethod]
    pub fn module() -> Self {
        Self {
            value: bridge::BundleSection::Module,
        }
    }

    /// Primary runnable entry output.
    #[staticmethod]
    pub fn entry() -> Self {
        Self {
            value: bridge::BundleSection::Entry,
        }
    }

    /// Declaration or type surface.
    #[staticmethod]
    pub fn declaration() -> Self {
        Self {
            value: bridge::BundleSection::Declaration,
        }
    }

    /// Asset collection emitted by this target.
    #[staticmethod]
    pub fn asset() -> Self {
        Self {
            value: bridge::BundleSection::Asset,
        }
    }

    /// Build manifest or output index.
    #[staticmethod]
    pub fn manifest() -> Self {
        Self {
            value: bridge::BundleSection::Manifest,
        }
    }

    /// Source maps or debug maps.
    #[staticmethod]
    pub fn source_map() -> Self {
        Self {
            value: bridge::BundleSection::SourceMap,
        }
    }

    /// Native object or wasm payload.
    #[staticmethod]
    pub fn native() -> Self {
        Self {
            value: bridge::BundleSection::Native,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::BundleSection {
        self.value
    }

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
    /// Per-module assets without target-level assembly.
    #[staticmethod]
    pub fn preserve_modules() -> Self {
        Self {
            value: bridge::BundleMode::PreserveModules,
        }
    }

    /// One assembled output file.
    #[staticmethod]
    pub fn single_file() -> Self {
        Self {
            value: bridge::BundleMode::SingleFile,
        }
    }

    /// Multiple assembled output files.
    #[staticmethod]
    pub fn chunked() -> Self {
        Self {
            value: bridge::BundleMode::Chunked,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::BundleMode {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(
        section: BundleSection,
        uri: String,
        file_type: FileType,
        content: ContentId,
        source: Option<String>,
    ) -> Self {
        Self {
            value: bridge::BundleFile {
                section: section.into_bridge(),
                uri,
                file_type: file_type.into_bridge(),
                content: content.into_bridge(),
                source,
            },
        }
    }

    /// The bundle section this file belongs to.
    #[getter]
    pub fn section(&self) -> BundleSection {
        BundleSection::from_bridge(self.value.section.clone())
    }

    /// The output URI.
    #[getter]
    pub fn uri(&self) -> String {
        self.value.uri.clone()
    }

    /// The emitted file type.
    #[getter]
    pub fn file_type(&self) -> FileType {
        FileType::from_bridge(self.value.file_type.clone())
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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::BundleFile {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(emit: EmitFormat, mode: BundleMode, files: Vec<BundleFile>) -> Self {
        Self {
            value: bridge::Bundle {
                emit: emit.into_bridge(),
                mode: mode.into_bridge(),
                files: files.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// The emitted artifact family.
    #[getter]
    pub fn emit(&self) -> EmitFormat {
        EmitFormat::from_bridge(self.value.emit.clone())
    }

    /// The target-level assembly mode.
    #[getter]
    pub fn mode(&self) -> BundleMode {
        BundleMode::from_bridge(self.value.mode.clone())
    }

    /// The files in this bundle.
    #[getter]
    pub fn files(&self) -> Vec<BundleFile> {
        self.value
            .files
            .clone()
            .into_iter()
            .map(|item| BundleFile::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl Bundle {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Bundle {
        self.value
    }

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
    /// VM executable program.
    #[staticmethod]
    pub fn vm() -> Self {
        Self {
            value: bridge::ProgramFormat::Vm,
        }
    }

    /// Native executable program.
    #[staticmethod]
    pub fn native() -> Self {
        Self {
            value: bridge::ProgramFormat::Native,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ProgramFormat {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(name: Option<String>, fingerprint: Option<String>, target: Option<String>) -> Self {
        Self {
            value: bridge::ProgramHeader {
                name,
                fingerprint,
                target,
            },
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ProgramHeader {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(header: ProgramHeader, format: ProgramFormat, contents: Vec<ContentId>) -> Self {
        Self {
            value: bridge::Program {
                header: header.into_bridge(),
                format: format.into_bridge(),
                contents: contents
                    .into_iter()
                    .map(|item| item.into_bridge())
                    .collect(),
            },
        }
    }

    /// The program identity and compatibility header.
    #[getter]
    pub fn header(&self) -> ProgramHeader {
        ProgramHeader::from_bridge(self.value.header.clone())
    }

    /// The executable format.
    #[getter]
    pub fn format(&self) -> ProgramFormat {
        ProgramFormat::from_bridge(self.value.format.clone())
    }

    /// Content blobs referenced by the executable payload.
    #[getter]
    pub fn contents(&self) -> Vec<ContentId> {
        self.value
            .contents
            .clone()
            .into_iter()
            .map(|item| ContentId::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl Program {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Program {
        self.value
    }

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
    /// Destack native runtime.
    #[staticmethod]
    pub fn destack() -> Self {
        Self {
            value: bridge::Runtime::Destack,
        }
    }

    /// JavaScript host runtime.
    #[staticmethod]
    pub fn js() -> Self {
        Self {
            value: bridge::Runtime::Js,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Runtime {
        self.value
    }

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
    /// Native host environment.
    #[staticmethod]
    pub fn native() -> Self {
        Self {
            value: bridge::Host::Native,
        }
    }

    /// Browser host environment.
    #[staticmethod]
    pub fn browser() -> Self {
        Self {
            value: bridge::Host::Browser,
        }
    }

    /// WASI host environment.
    #[staticmethod]
    pub fn wasi() -> Self {
        Self {
            value: bridge::Host::Wasi,
        }
    }

    /// Emscripten host environment.
    #[staticmethod]
    pub fn emscripten() -> Self {
        Self {
            value: bridge::Host::Emscripten,
        }
    }

    /// Freestanding target without host imports.
    #[staticmethod]
    pub fn freestanding() -> Self {
        Self {
            value: bridge::Host::Freestanding,
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Host {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(
        name: String,
        target: TargetId,
        runtime: Runtime,
        host: Host,
        platform: String,
        includes_build: bool,
        includes_bundle: bool,
        includes_program: bool,
    ) -> Self {
        Self {
            value: bridge::ProductTarget {
                name,
                target: target.into_bridge(),
                runtime: runtime.into_bridge(),
                host: host.into_bridge(),
                platform,
                includes_build,
                includes_bundle,
                includes_program,
            },
        }
    }

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
        Runtime::from_bridge(self.value.runtime.clone())
    }

    /// The host environment this target expects.
    #[getter]
    pub fn host(&self) -> Host {
        Host::from_bridge(self.value.host.clone())
    }

    /// The platform this target expects.
    #[getter]
    pub fn platform(&self) -> String {
        self.value.platform.clone()
    }

    /// Whether this product target includes its toolchain build payload.
    #[getter]
    pub fn includes_build(&self) -> bool {
        self.value.includes_build.clone()
    }

    /// Whether this product target includes its linked bundle.
    #[getter]
    pub fn includes_bundle(&self) -> bool {
        self.value.includes_bundle.clone()
    }

    /// Whether this product target includes its executable program.
    #[getter]
    pub fn includes_program(&self) -> bool {
        self.value.includes_program.clone()
    }
}

#[allow(dead_code)]
impl ProductTarget {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ProductTarget {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(name: String, targets: Vec<ProductTarget>) -> Self {
        Self {
            value: bridge::Product {
                name,
                targets: targets.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

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
            .map(|item| ProductTarget::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl Product {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Product {
        self.value
    }

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

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ModuleBuildKind) -> Self {
        Self { value }
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

    /// Return this payload field when present.
    #[getter]
    pub fn get_module_module(&self) -> Option<Module> {
        match &self.value {
            bridge::BuildRequest::Module { module, .. } => {
                Some(Module::from_bridge(module.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_output(&self) -> Option<ModuleBuildKind> {
        match &self.value {
            bridge::BuildRequest::Module { output, .. } => {
                Some(ModuleBuildKind::from_bridge(output.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_product_product(&self) -> Option<ProductId> {
        match &self.value {
            bridge::BuildRequest::Product { product, .. } => {
                Some(ProductId::from_bridge(product.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_target(&self) -> Option<TargetId> {
        match &self.value {
            bridge::BuildRequest::Module { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            bridge::BuildRequest::Build { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_target_target(&self) -> Option<TargetId> {
        match &self.value {
            bridge::BuildRequest::Target { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            _ => None,
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
    /// Built script artifact.
    #[staticmethod]
    pub fn script(version: ArtifactVersion, script: Script) -> Self {
        Self {
            value: bridge::BuildOutput::Script {
                version: version.into_bridge(),
                script: script.into_bridge(),
            },
        }
    }

    /// Built object artifact.
    #[staticmethod]
    pub fn object(version: ArtifactVersion, object: Object) -> Self {
        Self {
            value: bridge::BuildOutput::Object {
                version: version.into_bridge(),
                object: object.into_bridge(),
            },
        }
    }

    /// Built asset artifact.
    #[staticmethod]
    pub fn asset(version: ArtifactVersion, asset: Asset) -> Self {
        Self {
            value: bridge::BuildOutput::Asset {
                version: version.into_bridge(),
                asset: asset.into_bridge(),
            },
        }
    }

    /// Built toolchain payload artifact.
    #[staticmethod]
    pub fn build(version: ArtifactVersion, build: Build) -> Self {
        Self {
            value: bridge::BuildOutput::Build {
                version: version.into_bridge(),
                build: build.into_bridge(),
            },
        }
    }

    /// Built bundle artifact.
    #[staticmethod]
    pub fn bundle(version: ArtifactVersion, bundle: Bundle) -> Self {
        Self {
            value: bridge::BuildOutput::Bundle {
                version: version.into_bridge(),
                bundle: bundle.into_bridge(),
            },
        }
    }

    /// Built program artifact.
    #[staticmethod]
    pub fn program(version: ArtifactVersion, program: Program) -> Self {
        Self {
            value: bridge::BuildOutput::Program {
                version: version.into_bridge(),
                program: program.into_bridge(),
            },
        }
    }

    /// Built product artifact.
    #[staticmethod]
    pub fn product(version: ArtifactVersion, product: Product) -> Self {
        Self {
            value: bridge::BuildOutput::Product {
                version: version.into_bridge(),
                product: product.into_bridge(),
            },
        }
    }

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
    pub fn get_asset_asset(&self) -> Option<Asset> {
        match &self.value {
            bridge::BuildOutput::Asset { asset, .. } => Some(Asset::from_bridge(asset.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_build_build(&self) -> Option<Build> {
        match &self.value {
            bridge::BuildOutput::Build { build, .. } => Some(Build::from_bridge(build.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_bundle_bundle(&self) -> Option<Bundle> {
        match &self.value {
            bridge::BuildOutput::Bundle { bundle, .. } => Some(Bundle::from_bridge(bundle.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_object_object(&self) -> Option<Object> {
        match &self.value {
            bridge::BuildOutput::Object { object, .. } => Some(Object::from_bridge(object.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_product_product(&self) -> Option<Product> {
        match &self.value {
            bridge::BuildOutput::Product { product, .. } => {
                Some(Product::from_bridge(product.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_program_program(&self) -> Option<Program> {
        match &self.value {
            bridge::BuildOutput::Program { program, .. } => {
                Some(Program::from_bridge(program.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_script_script(&self) -> Option<Script> {
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
