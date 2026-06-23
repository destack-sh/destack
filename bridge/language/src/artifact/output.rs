use crate::{ContentId, TargetId, bridge};

/// Build distribution profile crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildProfile {
    /// Full Destack build.
    Full,
    /// Smaller Destack build with optional services omitted.
    Minimal,
    /// Freestanding output without the normal Destack runtime contract.
    Freestanding,
}

/// Build payload linkage crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildLinkage {
    /// Ship a portable Destack payload consumed by a runtime.
    Portable,
    /// Link the build payload into the produced platform binary.
    Static,
    /// Ship the build payload as a dynamic library.
    Dynamic,
}

/// Emitted artifact family crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmitFormat {
    /// JavaScript output.
    Js,
    /// TypeScript output.
    Ts,
    /// WebAssembly output.
    Wasm,
    /// Native binary output.
    Native,
}

/// Source file type crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// `.ds`.
    Destack,
    /// `.d.ds`.
    DestackDeclaration,
    /// `.js`.
    JavaScript,
    /// `.jsx`.
    JavaScriptXml,
    /// `.ts`.
    TypeScript,
    /// `.tsx`.
    TypeScriptXml,
    /// `.d.ts`.
    TypeScriptDeclaration,
    /// Text file.
    Text,
    /// TOML file.
    Toml,
    /// YAML file.
    Yaml,
    /// JSON file.
    Json,
    /// Environment file.
    Env,
    /// HTML file.
    Html,
    /// Markdown file.
    Markdown,
    /// CSS file.
    Css,
    /// SVG file.
    Svg,
    /// WebAssembly payload.
    Wasm,
    /// Node native module.
    Node,
    /// Source map file.
    SourceMap,
    /// Native object file.
    Object,
    /// Image asset.
    Image,
    /// Font asset.
    Font,
    /// Audio asset.
    Audio,
    /// Video asset.
    Video,
    /// 3D model asset.
    Model,
    /// AI model asset.
    Neural,
    /// Document asset.
    Document,
    /// Unknown binary file.
    Binary,
    /// Unknown file type.
    Unknown,
}

/// One emitted or linked source map crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapSource {
    /// The mapped source names.
    pub name: String,
    /// The embedded source contents when they exist.
    pub content: Option<String>,
}

/// One emitted or linked source map crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    /// The source map version.
    pub version: u32,
    /// The emitted file name when one exists.
    pub file: Option<String>,
    /// The source root when one exists.
    pub source_root: Option<String>,
    /// The mapped sources.
    pub sources: Vec<SourceMapSource>,
    /// The recorded symbol names.
    pub names: Vec<String>,
    /// The VLQ mapping payload.
    pub mappings: String,
    /// The debug id when one exists.
    pub debug_id: Option<String>,
}

/// One emitted declaration crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The declaration text.
    pub text: String,
}

/// Structured script language crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptLanguage {
    /// JavaScript output.
    JavaScript,
    /// TypeScript output.
    TypeScript,
}

/// One structured script artifact crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// The target language of this script.
    pub language: ScriptLanguage,
    /// The emitted declaration when one exists.
    pub declaration: Option<Declaration>,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
    /// Whether this script has top-level side effects.
    pub has_top_level_side_effects: bool,
}

/// Compiled-code object format crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectFormat {
    /// Native relocatable object file.
    Object,
    /// WebAssembly object or module payload.
    Wasm,
}

/// One compiled-code object artifact crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    /// The compiled-code object format.
    pub format: ObjectFormat,
    /// The encoded object content identity.
    pub content: ContentId,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
}

/// One opaque asset artifact crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// The asset file type.
    pub file_type: FileType,
    /// The asset content identity.
    pub content: ContentId,
    /// The source module URI when one exists.
    pub source: Option<String>,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
}

/// One target-built toolchain payload crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    /// The build distribution profile.
    pub profile: BuildProfile,
    /// The build linkage.
    pub linkage: BuildLinkage,
    /// The encoded build content.
    pub content: ContentId,
}

/// One section of a linked bundle crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BundleSection {
    /// Per-module library output.
    Module,
    /// Primary runnable entry output.
    Entry,
    /// Declaration or type surface.
    Declaration,
    /// Asset collection emitted by this target.
    Asset,
    /// Build manifest or output index.
    Manifest,
    /// Source maps or debug maps.
    SourceMap,
    /// Native object or wasm payload.
    Native,
}

/// Bundle assembly mode crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BundleMode {
    /// Per-module assets without target-level assembly.
    PreserveModules,
    /// One assembled output file.
    SingleFile,
    /// Multiple assembled output files.
    Chunked,
}

/// One derived bundle file crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFile {
    /// The bundle section this file belongs to.
    pub section: BundleSection,
    /// The output URI.
    pub uri: String,
    /// The emitted file type.
    pub file_type: FileType,
    /// The output content identity.
    pub content: ContentId,
    /// The related source URI when one exists.
    pub source: Option<String>,
}

/// One linked file graph crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    /// The emitted artifact family.
    pub emit: EmitFormat,
    /// The target-level assembly mode.
    pub mode: BundleMode,
    /// The files in this bundle.
    pub files: Vec<BundleFile>,
}

/// Preferred program execution format crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgramFormat {
    /// VM execution.
    Vm,
    /// Native execution.
    Native,
}

/// Durable program header crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramHeader {
    /// Pointer byte width required by this program.
    pub pointer_bytes: u32,
}

/// Durable program crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The program identity and compatibility header.
    pub header: ProgramHeader,
    /// The preferred execution format.
    pub format: ProgramFormat,
    /// Content blobs referenced by the executable payload.
    pub contents: Vec<ContentId>,
}

/// Semantic runtime contract crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Runtime {
    /// Destack native runtime.
    Destack,
    /// JavaScript host runtime.
    Js,
}

/// Host environment crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Host {
    /// Native host environment.
    Native,
    /// Browser host environment.
    Browser,
    /// WASI host environment.
    Wasi,
    /// Emscripten host environment.
    Emscripten,
    /// Freestanding target without host imports.
    Freestanding,
}

/// One linked product target crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductTarget {
    /// The configured product target name.
    pub name: String,
    /// The repository target assembled into this product.
    pub target: TargetId,
    /// The runtime contract this target expects.
    pub runtime: Runtime,
    /// The host environment this target expects.
    pub host: Host,
    /// The platform this target expects.
    pub platform: String,
    /// Whether this product target includes its toolchain build payload.
    pub includes_build: bool,
    /// Whether this product target includes its linked bundle.
    pub includes_bundle: bool,
    /// Whether this product target includes its executable program.
    pub includes_program: bool,
}

/// One linked product crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    /// The configured product name.
    pub name: String,
    /// The linked targets in deterministic order.
    pub targets: Vec<ProductTarget>,
}

impl From<destack_artifact::BuildProfile> for BuildProfile {
    /// Convert one artifact build profile into one bridge build profile.
    fn from(profile: destack_artifact::BuildProfile) -> Self {
        match profile {
            destack_artifact::BuildProfile::Full => Self::Full,
            destack_artifact::BuildProfile::Minimal => Self::Minimal,
            destack_artifact::BuildProfile::Freestanding => Self::Freestanding,
        }
    }
}

impl From<destack_artifact::BuildLinkage> for BuildLinkage {
    /// Convert one artifact build linkage into one bridge build linkage.
    fn from(linkage: destack_artifact::BuildLinkage) -> Self {
        match linkage {
            destack_artifact::BuildLinkage::Portable => Self::Portable,
            destack_artifact::BuildLinkage::Static => Self::Static,
            destack_artifact::BuildLinkage::Dynamic => Self::Dynamic,
        }
    }
}

impl From<destack_artifact::EmitFormat> for EmitFormat {
    /// Convert one artifact emit format into one bridge emit format.
    fn from(format: destack_artifact::EmitFormat) -> Self {
        match format {
            destack_artifact::EmitFormat::Js => Self::Js,
            destack_artifact::EmitFormat::Ts => Self::Ts,
            destack_artifact::EmitFormat::Wasm => Self::Wasm,
            destack_artifact::EmitFormat::Native => Self::Native,
        }
    }
}

impl From<destack_source::FileType> for FileType {
    /// Convert one source file type into one bridge file type.
    fn from(file_type: destack_source::FileType) -> Self {
        match file_type {
            destack_source::FileType::Destack => Self::Destack,
            destack_source::FileType::DestackDeclaration => Self::DestackDeclaration,
            destack_source::FileType::JavaScript => Self::JavaScript,
            destack_source::FileType::JavaScriptXml => Self::JavaScriptXml,
            destack_source::FileType::TypeScript => Self::TypeScript,
            destack_source::FileType::TypeScriptXml => Self::TypeScriptXml,
            destack_source::FileType::TypeScriptDeclaration => Self::TypeScriptDeclaration,
            destack_source::FileType::Text => Self::Text,
            destack_source::FileType::Toml => Self::Toml,
            destack_source::FileType::Yaml => Self::Yaml,
            destack_source::FileType::Json => Self::Json,
            destack_source::FileType::Env => Self::Env,
            destack_source::FileType::Html => Self::Html,
            destack_source::FileType::Markdown => Self::Markdown,
            destack_source::FileType::Css => Self::Css,
            destack_source::FileType::Svg => Self::Svg,
            destack_source::FileType::Wasm => Self::Wasm,
            destack_source::FileType::Node => Self::Node,
            destack_source::FileType::SourceMap => Self::SourceMap,
            destack_source::FileType::Object => Self::Object,
            destack_source::FileType::Image => Self::Image,
            destack_source::FileType::Font => Self::Font,
            destack_source::FileType::Audio => Self::Audio,
            destack_source::FileType::Video => Self::Video,
            destack_source::FileType::Model => Self::Model,
            destack_source::FileType::Neural => Self::Neural,
            destack_source::FileType::Document => Self::Document,
            destack_source::FileType::Binary => Self::Binary,
            destack_source::FileType::Unknown => Self::Unknown,
        }
    }
}

impl From<FileType> for destack_source::FileType {
    /// Convert one bridge file type into one source file type.
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Destack => Self::Destack,
            FileType::DestackDeclaration => Self::DestackDeclaration,
            FileType::JavaScript => Self::JavaScript,
            FileType::JavaScriptXml => Self::JavaScriptXml,
            FileType::TypeScript => Self::TypeScript,
            FileType::TypeScriptXml => Self::TypeScriptXml,
            FileType::TypeScriptDeclaration => Self::TypeScriptDeclaration,
            FileType::Text => Self::Text,
            FileType::Toml => Self::Toml,
            FileType::Yaml => Self::Yaml,
            FileType::Json => Self::Json,
            FileType::Env => Self::Env,
            FileType::Html => Self::Html,
            FileType::Markdown => Self::Markdown,
            FileType::Css => Self::Css,
            FileType::Svg => Self::Svg,
            FileType::Wasm => Self::Wasm,
            FileType::Node => Self::Node,
            FileType::SourceMap => Self::SourceMap,
            FileType::Object => Self::Object,
            FileType::Image => Self::Image,
            FileType::Font => Self::Font,
            FileType::Audio => Self::Audio,
            FileType::Video => Self::Video,
            FileType::Model => Self::Model,
            FileType::Neural => Self::Neural,
            FileType::Document => Self::Document,
            FileType::Binary => Self::Binary,
            FileType::Unknown => Self::Unknown,
        }
    }
}

impl From<destack_artifact::SourceMap> for SourceMap {
    /// Convert one artifact source map into one bridge source map.
    fn from(map: destack_artifact::SourceMap) -> Self {
        let mut source_contents = map.sources_content.unwrap_or_default();
        let sources = map
            .sources
            .into_iter()
            .enumerate()
            .map(|(index, name)| SourceMapSource {
                name,
                content: source_contents.get_mut(index).and_then(Option::take),
            })
            .collect();

        Self {
            version: map.version,
            file: map.file,
            source_root: map.source_root,
            sources,
            names: map.names,
            mappings: map.mappings,
            debug_id: map.debug_id,
        }
    }
}

impl From<destack_artifact::Declaration> for Declaration {
    /// Convert one artifact declaration into one bridge declaration.
    fn from(declaration: destack_artifact::Declaration) -> Self {
        Self {
            text: declaration.text,
        }
    }
}

impl From<destack_artifact::ScriptLanguage> for ScriptLanguage {
    /// Convert one artifact script language into one bridge script language.
    fn from(language: destack_artifact::ScriptLanguage) -> Self {
        match language {
            destack_artifact::ScriptLanguage::JavaScript => Self::JavaScript,
            destack_artifact::ScriptLanguage::TypeScript => Self::TypeScript,
        }
    }
}

impl From<&destack_artifact::Script> for Script {
    /// Convert one artifact script into one bridge script.
    fn from(script: &destack_artifact::Script) -> Self {
        Self {
            language: script.language.into(),
            declaration: script.declaration.clone().map(Into::into),
            map: script.map.clone().map(Into::into),
            has_top_level_side_effects: script.has_top_level_side_effects,
        }
    }
}

impl From<destack_artifact::ObjectFormat> for ObjectFormat {
    /// Convert one artifact object format into one bridge object format.
    fn from(format: destack_artifact::ObjectFormat) -> Self {
        match format {
            destack_artifact::ObjectFormat::Object => Self::Object,
            destack_artifact::ObjectFormat::Wasm => Self::Wasm,
        }
    }
}

impl From<&destack_artifact::Object> for Object {
    /// Convert one artifact object into one bridge object.
    fn from(object: &destack_artifact::Object) -> Self {
        Self {
            format: object.format.into(),
            content: object.content.into(),
            map: object.map.clone().map(Into::into),
        }
    }
}

impl From<&destack_artifact::Asset> for Asset {
    /// Convert one artifact asset into one bridge asset.
    fn from(asset: &destack_artifact::Asset) -> Self {
        Self {
            file_type: asset.file_type.into(),
            content: asset.content.into(),
            source: asset.source.as_ref().map(ToString::to_string),
            map: asset.map.clone().map(Into::into),
        }
    }
}

impl From<&destack_artifact::Build> for Build {
    /// Convert one artifact build payload into one bridge build payload.
    fn from(build: &destack_artifact::Build) -> Self {
        Self {
            profile: build.profile.into(),
            linkage: build.linkage.into(),
            content: build.content.into(),
        }
    }
}

impl From<destack_artifact::BundleSection> for BundleSection {
    /// Convert one artifact bundle section into one bridge bundle section.
    fn from(section: destack_artifact::BundleSection) -> Self {
        match section {
            destack_artifact::BundleSection::Module => Self::Module,
            destack_artifact::BundleSection::Entry => Self::Entry,
            destack_artifact::BundleSection::Declaration => Self::Declaration,
            destack_artifact::BundleSection::Asset => Self::Asset,
            destack_artifact::BundleSection::Manifest => Self::Manifest,
            destack_artifact::BundleSection::SourceMap => Self::SourceMap,
            destack_artifact::BundleSection::Native => Self::Native,
        }
    }
}

impl From<destack_artifact::BundleMode> for BundleMode {
    /// Convert one artifact bundle mode into one bridge bundle mode.
    fn from(mode: destack_artifact::BundleMode) -> Self {
        match mode {
            destack_artifact::BundleMode::PreserveModules => Self::PreserveModules,
            destack_artifact::BundleMode::SingleFile => Self::SingleFile,
            destack_artifact::BundleMode::Chunked => Self::Chunked,
        }
    }
}

impl From<&destack_artifact::BundleFile> for BundleFile {
    /// Convert one artifact bundle file into one bridge bundle file.
    fn from(file: &destack_artifact::BundleFile) -> Self {
        Self {
            section: file.section.into(),
            uri: file.uri.to_string(),
            file_type: file.file_type.into(),
            content: file.content.into(),
            source: file.source.as_ref().map(ToString::to_string),
        }
    }
}

impl From<&destack_artifact::Bundle> for Bundle {
    /// Convert one artifact bundle into one bridge bundle.
    fn from(bundle: &destack_artifact::Bundle) -> Self {
        Self {
            emit: bundle.emit.into(),
            mode: bundle.assembly.into(),
            files: bundle.files.iter().map(Into::into).collect(),
        }
    }
}

impl From<&destack_program::ProgramHeader> for ProgramHeader {
    /// Convert one program header into one bridge program header.
    fn from(header: &destack_program::ProgramHeader) -> Self {
        Self {
            pointer_bytes: u32::from(header.pointer_bytes),
        }
    }
}

impl From<&destack_program::Program> for Program {
    /// Convert one program into one bridge program.
    fn from(program: &destack_program::Program) -> Self {
        let format = if program.native.is_some() {
            ProgramFormat::Native
        } else {
            ProgramFormat::Vm
        };

        Self {
            header: (&program.header).into(),
            format,
            contents: program.content_ids().into_iter().map(Into::into).collect(),
        }
    }
}

impl From<destack_artifact::Runtime> for Runtime {
    /// Convert one artifact runtime into one bridge runtime.
    fn from(runtime: destack_artifact::Runtime) -> Self {
        match runtime {
            destack_artifact::Runtime::Destack => Self::Destack,
            destack_artifact::Runtime::Js => Self::Js,
        }
    }
}

impl From<destack_artifact::Host> for Host {
    /// Convert one artifact host into one bridge host.
    fn from(host: destack_artifact::Host) -> Self {
        match host {
            destack_artifact::Host::Native => Self::Native,
            destack_artifact::Host::Browser => Self::Browser,
            destack_artifact::Host::Wasi => Self::Wasi,
            destack_artifact::Host::Emscripten => Self::Emscripten,
            destack_artifact::Host::Freestanding => Self::Freestanding,
        }
    }
}

impl From<&destack_artifact::ProductTarget> for ProductTarget {
    /// Convert one artifact product target into one bridge product target.
    fn from(target: &destack_artifact::ProductTarget) -> Self {
        Self {
            name: target.name.clone(),
            target: target.target.into(),
            runtime: target.runtime.into(),
            host: target.host.into(),
            platform: target.platform.canonical_tag().to_string(),
            includes_build: target.includes_build,
            includes_bundle: target.includes_bundle,
            includes_program: target.includes_program,
        }
    }
}

impl From<&destack_artifact::Product> for Product {
    /// Convert one artifact product into one bridge product.
    fn from(product: &destack_artifact::Product) -> Self {
        let targets = product.targets.values().map(Into::into).collect();

        Self {
            name: product.name.clone(),
            targets,
        }
    }
}
