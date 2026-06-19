use destack_artifact as artifact;
use destack_program as program;
use destack_source as source;

use crate::{ArtifactVersion, ContentId, Module, ProductId, TargetId, bridge};

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

/// Program executable format crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgramFormat {
    /// VM executable program.
    Vm,
    /// Native executable program.
    Native,
}

/// Durable program header crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramHeader {
    /// Human-facing program name.
    pub name: Option<String>,
    /// Build fingerprint that produced this program.
    pub fingerprint: Option<String>,
    /// Target triple or equivalent target identity.
    pub target: Option<String>,
}

/// Durable executable program crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The program identity and compatibility header.
    pub header: ProgramHeader,
    /// The executable format.
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

/// Module build output family.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleBuildKind {
    /// Build the structured script artifact.
    Script,
    /// Build the compiled object artifact.
    Object,
    /// Build the opaque asset artifact.
    Asset,
}

/// One language build request.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildRequest {
    /// Build one module artifact.
    Module {
        /// Source module.
        module: Module,
        /// Build target.
        target: TargetId,
        /// Requested module artifact family.
        output: ModuleBuildKind,
    },
    /// Build one target build payload.
    Build {
        /// Build target.
        target: TargetId,
    },
    /// Build one package target.
    Target {
        /// Build target.
        target: TargetId,
    },
    /// Build one product.
    Product {
        /// Product id.
        product: ProductId,
    },
}

/// One language build output.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildOutput {
    /// Built script artifact.
    Script {
        /// Exact artifact version.
        version: ArtifactVersion,
        /// Script payload.
        script: Script,
    },
    /// Built object artifact.
    Object {
        /// Exact artifact version.
        version: ArtifactVersion,
        /// Object payload.
        object: Object,
    },
    /// Built asset artifact.
    Asset {
        /// Exact artifact version.
        version: ArtifactVersion,
        /// Asset payload.
        asset: Asset,
    },
    /// Built toolchain payload artifact.
    Build {
        /// Exact artifact version.
        version: ArtifactVersion,
        /// Build payload.
        build: Build,
    },
    /// Built bundle artifact.
    Bundle {
        /// Exact artifact version.
        version: ArtifactVersion,
        /// Bundle payload.
        bundle: Bundle,
    },
    /// Built program artifact.
    Program {
        /// Exact artifact version.
        version: ArtifactVersion,
        /// Program payload.
        program: Program,
    },
    /// Built product artifact.
    Product {
        /// Exact artifact version.
        version: ArtifactVersion,
        /// Product payload.
        product: Product,
    },
}

impl BuildRequest {
    /// Build one module artifact.
    pub fn module(module: Module, target: TargetId, output: ModuleBuildKind) -> Self {
        Self::Module {
            module,
            target,
            output,
        }
    }

    /// Build one target build payload.
    pub fn build(target: TargetId) -> Self {
        Self::Build { target }
    }

    /// Build one package target.
    pub fn target(target: TargetId) -> Self {
        Self::Target { target }
    }

    /// Build one product.
    pub fn product(product: ProductId) -> Self {
        Self::Product { product }
    }
}

impl From<artifact::BuildProfile> for BuildProfile {
    /// Convert one artifact build profile into one bridge build profile.
    fn from(profile: artifact::BuildProfile) -> Self {
        match profile {
            artifact::BuildProfile::Full => Self::Full,
            artifact::BuildProfile::Minimal => Self::Minimal,
            artifact::BuildProfile::Freestanding => Self::Freestanding,
        }
    }
}

impl From<artifact::BuildLinkage> for BuildLinkage {
    /// Convert one artifact build linkage into one bridge build linkage.
    fn from(linkage: artifact::BuildLinkage) -> Self {
        match linkage {
            artifact::BuildLinkage::Portable => Self::Portable,
            artifact::BuildLinkage::Static => Self::Static,
            artifact::BuildLinkage::Dynamic => Self::Dynamic,
        }
    }
}

impl From<artifact::EmitFormat> for EmitFormat {
    /// Convert one artifact emit format into one bridge emit format.
    fn from(format: artifact::EmitFormat) -> Self {
        match format {
            artifact::EmitFormat::Js => Self::Js,
            artifact::EmitFormat::Ts => Self::Ts,
            artifact::EmitFormat::Wasm => Self::Wasm,
            artifact::EmitFormat::Native => Self::Native,
        }
    }
}

impl From<source::FileType> for FileType {
    /// Convert one source file type into one bridge file type.
    fn from(file_type: source::FileType) -> Self {
        match file_type {
            source::FileType::Destack => Self::Destack,
            source::FileType::DestackDeclaration => Self::DestackDeclaration,
            source::FileType::JavaScript => Self::JavaScript,
            source::FileType::JavaScriptXml => Self::JavaScriptXml,
            source::FileType::TypeScript => Self::TypeScript,
            source::FileType::TypeScriptXml => Self::TypeScriptXml,
            source::FileType::TypeScriptDeclaration => Self::TypeScriptDeclaration,
            source::FileType::Text => Self::Text,
            source::FileType::Toml => Self::Toml,
            source::FileType::Yaml => Self::Yaml,
            source::FileType::Json => Self::Json,
            source::FileType::Env => Self::Env,
            source::FileType::Html => Self::Html,
            source::FileType::Markdown => Self::Markdown,
            source::FileType::Css => Self::Css,
            source::FileType::Svg => Self::Svg,
            source::FileType::Wasm => Self::Wasm,
            source::FileType::Node => Self::Node,
            source::FileType::SourceMap => Self::SourceMap,
            source::FileType::Object => Self::Object,
            source::FileType::Image => Self::Image,
            source::FileType::Font => Self::Font,
            source::FileType::Audio => Self::Audio,
            source::FileType::Video => Self::Video,
            source::FileType::Model => Self::Model,
            source::FileType::Neural => Self::Neural,
            source::FileType::Document => Self::Document,
            source::FileType::Binary => Self::Binary,
            source::FileType::Unknown => Self::Unknown,
        }
    }
}

impl From<FileType> for source::FileType {
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

impl From<artifact::SourceMap> for SourceMap {
    /// Convert one artifact source map into one bridge source map.
    fn from(map: artifact::SourceMap) -> Self {
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

impl From<artifact::Declaration> for Declaration {
    /// Convert one artifact declaration into one bridge declaration.
    fn from(declaration: artifact::Declaration) -> Self {
        Self {
            text: declaration.text,
        }
    }
}

impl From<artifact::ScriptLanguage> for ScriptLanguage {
    /// Convert one artifact script language into one bridge script language.
    fn from(language: artifact::ScriptLanguage) -> Self {
        match language {
            artifact::ScriptLanguage::JavaScript => Self::JavaScript,
            artifact::ScriptLanguage::TypeScript => Self::TypeScript,
        }
    }
}

impl From<&artifact::Script> for Script {
    /// Convert one artifact script into one bridge script.
    fn from(script: &artifact::Script) -> Self {
        Self {
            language: script.language.into(),
            declaration: script.declaration.clone().map(Into::into),
            map: script.map.clone().map(Into::into),
            has_top_level_side_effects: script.has_top_level_side_effects,
        }
    }
}

impl From<artifact::ObjectFormat> for ObjectFormat {
    /// Convert one artifact object format into one bridge object format.
    fn from(format: artifact::ObjectFormat) -> Self {
        match format {
            artifact::ObjectFormat::Object => Self::Object,
            artifact::ObjectFormat::Wasm => Self::Wasm,
        }
    }
}

impl From<&artifact::Object> for Object {
    /// Convert one artifact object into one bridge object.
    fn from(object: &artifact::Object) -> Self {
        Self {
            format: object.format.into(),
            content: object.content.into(),
            map: object.map.clone().map(Into::into),
        }
    }
}

impl From<&artifact::Asset> for Asset {
    /// Convert one artifact asset into one bridge asset.
    fn from(asset: &artifact::Asset) -> Self {
        Self {
            file_type: asset.file_type.into(),
            content: asset.content.into(),
            source: asset.source.as_ref().map(ToString::to_string),
            map: asset.map.clone().map(Into::into),
        }
    }
}

impl From<&artifact::Build> for Build {
    /// Convert one artifact build payload into one bridge build payload.
    fn from(build: &artifact::Build) -> Self {
        Self {
            profile: build.profile.into(),
            linkage: build.linkage.into(),
            content: build.content.into(),
        }
    }
}

impl From<artifact::BundleSection> for BundleSection {
    /// Convert one artifact bundle section into one bridge bundle section.
    fn from(section: artifact::BundleSection) -> Self {
        match section {
            artifact::BundleSection::Module => Self::Module,
            artifact::BundleSection::Entry => Self::Entry,
            artifact::BundleSection::Declaration => Self::Declaration,
            artifact::BundleSection::Asset => Self::Asset,
            artifact::BundleSection::Manifest => Self::Manifest,
            artifact::BundleSection::SourceMap => Self::SourceMap,
            artifact::BundleSection::Native => Self::Native,
        }
    }
}

impl From<artifact::BundleMode> for BundleMode {
    /// Convert one artifact bundle mode into one bridge bundle mode.
    fn from(mode: artifact::BundleMode) -> Self {
        match mode {
            artifact::BundleMode::PreserveModules => Self::PreserveModules,
            artifact::BundleMode::SingleFile => Self::SingleFile,
            artifact::BundleMode::Chunked => Self::Chunked,
        }
    }
}

impl From<&artifact::BundleFile> for BundleFile {
    /// Convert one artifact bundle file into one bridge bundle file.
    fn from(file: &artifact::BundleFile) -> Self {
        Self {
            section: file.section.into(),
            uri: file.uri.to_string(),
            file_type: file.file_type.into(),
            content: file.content.into(),
            source: file.source.as_ref().map(ToString::to_string),
        }
    }
}

impl From<&artifact::Bundle> for Bundle {
    /// Convert one artifact bundle into one bridge bundle.
    fn from(bundle: &artifact::Bundle) -> Self {
        Self {
            emit: bundle.emit.into(),
            mode: bundle.assembly.into(),
            files: bundle.files.iter().map(Into::into).collect(),
        }
    }
}

impl From<program::ProgramFormat> for ProgramFormat {
    /// Convert one program format into one bridge program format.
    fn from(format: program::ProgramFormat) -> Self {
        match format {
            program::ProgramFormat::Vm => Self::Vm,
            program::ProgramFormat::Native => Self::Native,
        }
    }
}

impl From<&program::ProgramHeader> for ProgramHeader {
    /// Convert one program header into one bridge program header.
    fn from(header: &program::ProgramHeader) -> Self {
        Self {
            name: header.name.clone(),
            fingerprint: header.fingerprint.clone(),
            target: header.target.clone(),
        }
    }
}

impl From<&program::Program> for Program {
    /// Convert one program into one bridge program.
    fn from(program: &program::Program) -> Self {
        Self {
            header: (&program.header).into(),
            format: program.format().into(),
            contents: program.content_ids().into_iter().map(Into::into).collect(),
        }
    }
}

impl From<artifact::Runtime> for Runtime {
    /// Convert one artifact runtime into one bridge runtime.
    fn from(runtime: artifact::Runtime) -> Self {
        match runtime {
            artifact::Runtime::Destack => Self::Destack,
            artifact::Runtime::Js => Self::Js,
        }
    }
}

impl From<artifact::Host> for Host {
    /// Convert one artifact host into one bridge host.
    fn from(host: artifact::Host) -> Self {
        match host {
            artifact::Host::Native => Self::Native,
            artifact::Host::Browser => Self::Browser,
            artifact::Host::Wasi => Self::Wasi,
            artifact::Host::Emscripten => Self::Emscripten,
            artifact::Host::Freestanding => Self::Freestanding,
        }
    }
}

impl From<&artifact::ProductTarget> for ProductTarget {
    /// Convert one artifact product target into one bridge product target.
    fn from(target: &artifact::ProductTarget) -> Self {
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

impl From<&artifact::Product> for Product {
    /// Convert one artifact product into one bridge product.
    fn from(product: &artifact::Product) -> Self {
        let targets = product.targets.values().map(Into::into).collect();

        Self {
            name: product.name.clone(),
            targets,
        }
    }
}
