// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ArtifactVersion, ContentId, Module, ProductId, TargetId};

/// One emitted or linked source map crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceMapSource {
    name: String,
    content: Option<String>,
}

#[wasm_bindgen]
impl SourceMapSource {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, content: Option<String>) -> Self {
        Self { name, content }
    }

    /// The mapped source names.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The embedded source contents when they exist.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> Option<String> {
        self.content.clone()
    }
}

impl SourceMapSource {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::SourceMapSource) -> Self {
        Self {
            name: value.name,
            content: value.content,
        }
    }
}

/// One emitted or linked source map crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct SourceMap {
    version: u32,
    file: Option<String>,
    source_root: Option<String>,
    sources: Vec<SourceMapSource>,
    names: Vec<String>,
    mappings: String,
    debug_id: Option<String>,
}

#[wasm_bindgen]
impl SourceMap {
    /// Create one value.
    #[wasm_bindgen(constructor)]
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
            version,
            file,
            source_root,
            sources,
            names,
            mappings,
            debug_id,
        }
    }

    /// The source map version.
    #[wasm_bindgen(getter, js_name = "version")]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// The emitted file name when one exists.
    #[wasm_bindgen(getter, js_name = "file")]
    pub fn file(&self) -> Option<String> {
        self.file.clone()
    }

    /// The source root when one exists.
    #[wasm_bindgen(getter, js_name = "sourceRoot")]
    pub fn source_root(&self) -> Option<String> {
        self.source_root.clone()
    }

    /// The mapped sources.
    #[wasm_bindgen(getter, js_name = "sources")]
    pub fn sources(&self) -> Vec<SourceMapSource> {
        self.sources.clone()
    }

    /// The recorded symbol names.
    #[wasm_bindgen(getter, js_name = "names")]
    pub fn names(&self) -> Vec<String> {
        self.names.clone()
    }

    /// The VLQ mapping payload.
    #[wasm_bindgen(getter, js_name = "mappings")]
    pub fn mappings(&self) -> String {
        self.mappings.clone()
    }

    /// The debug id when one exists.
    #[wasm_bindgen(getter, js_name = "debugId")]
    pub fn debug_id(&self) -> Option<String> {
        self.debug_id.clone()
    }
}

impl SourceMap {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::SourceMap) -> Self {
        Self {
            version: value.version,
            file: value.file,
            source_root: value.source_root,
            sources: value
                .sources
                .into_iter()
                .map(|item| SourceMapSource::from_bridge(item))
                .collect(),
            names: value.names,
            mappings: value.mappings,
            debug_id: value.debug_id,
        }
    }
}

/// One emitted declaration crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Declaration {
    text: String,
}

#[wasm_bindgen]
impl Declaration {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(text: String) -> Self {
        Self { text }
    }

    /// The declaration text.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> String {
        self.text.clone()
    }
}

impl Declaration {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Declaration) -> Self {
        Self { text: value.text }
    }
}

/// One structured script artifact crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Script {
    language: String,
    declaration: Option<Declaration>,
    map: Option<SourceMap>,
    has_top_level_side_effects: bool,
}

#[wasm_bindgen]
impl Script {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        language: String,
        declaration: Option<Declaration>,
        map: Option<SourceMap>,
        has_top_level_side_effects: bool,
    ) -> Self {
        Self {
            language,
            declaration,
            map,
            has_top_level_side_effects,
        }
    }

    /// The target language of this script.
    #[wasm_bindgen(getter, js_name = "language")]
    pub fn language(&self) -> String {
        self.language.clone()
    }

    /// The emitted declaration when one exists.
    #[wasm_bindgen(getter, js_name = "declaration")]
    pub fn declaration(&self) -> Option<Declaration> {
        self.declaration.clone()
    }

    /// The source map when one exists.
    #[wasm_bindgen(getter, js_name = "map")]
    pub fn map(&self) -> Option<SourceMap> {
        self.map.clone()
    }

    /// Whether this script has top-level side effects.
    #[wasm_bindgen(getter, js_name = "hasTopLevelSideEffects")]
    pub fn has_top_level_side_effects(&self) -> bool {
        self.has_top_level_side_effects
    }
}

impl Script {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Script) -> Self {
        Self {
            language: script_language_label(value.language),
            declaration: value.declaration.map(|item| Declaration::from_bridge(item)),
            map: value.map.map(|item| SourceMap::from_bridge(item)),
            has_top_level_side_effects: value.has_top_level_side_effects,
        }
    }
}

/// One compiled-code object artifact crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Object {
    format: String,
    content: ContentId,
    map: Option<SourceMap>,
}

#[wasm_bindgen]
impl Object {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(format: String, content: ContentId, map: Option<SourceMap>) -> Self {
        Self {
            format,
            content,
            map,
        }
    }

    /// The compiled-code object format.
    #[wasm_bindgen(getter, js_name = "format")]
    pub fn format(&self) -> String {
        self.format.clone()
    }

    /// The encoded object content identity.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> ContentId {
        self.content.clone()
    }

    /// The source map when one exists.
    #[wasm_bindgen(getter, js_name = "map")]
    pub fn map(&self) -> Option<SourceMap> {
        self.map.clone()
    }
}

impl Object {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Object) -> Self {
        Self {
            format: object_format_label(value.format),
            content: ContentId::from_bridge(value.content),
            map: value.map.map(|item| SourceMap::from_bridge(item)),
        }
    }
}

/// One opaque asset artifact crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Asset {
    file_type: String,
    content: ContentId,
    source: Option<String>,
    map: Option<SourceMap>,
}

#[wasm_bindgen]
impl Asset {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        file_type: String,
        content: ContentId,
        source: Option<String>,
        map: Option<SourceMap>,
    ) -> Self {
        Self {
            file_type,
            content,
            source,
            map,
        }
    }

    /// The asset file type.
    #[wasm_bindgen(getter, js_name = "fileType")]
    pub fn file_type(&self) -> String {
        self.file_type.clone()
    }

    /// The asset content identity.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> ContentId {
        self.content.clone()
    }

    /// The source module URI when one exists.
    #[wasm_bindgen(getter, js_name = "source")]
    pub fn source(&self) -> Option<String> {
        self.source.clone()
    }

    /// The source map when one exists.
    #[wasm_bindgen(getter, js_name = "map")]
    pub fn map(&self) -> Option<SourceMap> {
        self.map.clone()
    }
}

impl Asset {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Asset) -> Self {
        Self {
            file_type: file_type_label(value.file_type),
            content: ContentId::from_bridge(value.content),
            source: value.source,
            map: value.map.map(|item| SourceMap::from_bridge(item)),
        }
    }
}

/// One target-built toolchain payload crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Build {
    profile: String,
    linkage: String,
    content: ContentId,
}

#[wasm_bindgen]
impl Build {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(profile: String, linkage: String, content: ContentId) -> Self {
        Self {
            profile,
            linkage,
            content,
        }
    }

    /// The build distribution profile.
    #[wasm_bindgen(getter, js_name = "profile")]
    pub fn profile(&self) -> String {
        self.profile.clone()
    }

    /// The build linkage.
    #[wasm_bindgen(getter, js_name = "linkage")]
    pub fn linkage(&self) -> String {
        self.linkage.clone()
    }

    /// The encoded build content.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> ContentId {
        self.content.clone()
    }
}

impl Build {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Build) -> Self {
        Self {
            profile: build_profile_label(value.profile),
            linkage: build_linkage_label(value.linkage),
            content: ContentId::from_bridge(value.content),
        }
    }
}

/// One derived bundle file crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct BundleFile {
    section: String,
    uri: String,
    file_type: String,
    content: ContentId,
    source: Option<String>,
}

#[wasm_bindgen]
impl BundleFile {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        section: String,
        uri: String,
        file_type: String,
        content: ContentId,
        source: Option<String>,
    ) -> Self {
        Self {
            section,
            uri,
            file_type,
            content,
            source,
        }
    }

    /// The bundle section this file belongs to.
    #[wasm_bindgen(getter, js_name = "section")]
    pub fn section(&self) -> String {
        self.section.clone()
    }

    /// The output URI.
    #[wasm_bindgen(getter, js_name = "uri")]
    pub fn uri(&self) -> String {
        self.uri.clone()
    }

    /// The emitted file type.
    #[wasm_bindgen(getter, js_name = "fileType")]
    pub fn file_type(&self) -> String {
        self.file_type.clone()
    }

    /// The output content identity.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> ContentId {
        self.content.clone()
    }

    /// The related source URI when one exists.
    #[wasm_bindgen(getter, js_name = "source")]
    pub fn source(&self) -> Option<String> {
        self.source.clone()
    }
}

impl BundleFile {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::BundleFile) -> Self {
        Self {
            section: bundle_section_label(value.section),
            uri: value.uri,
            file_type: file_type_label(value.file_type),
            content: ContentId::from_bridge(value.content),
            source: value.source,
        }
    }
}

/// One linked file graph crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Bundle {
    emit: String,
    mode: String,
    files: Vec<BundleFile>,
}

#[wasm_bindgen]
impl Bundle {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(emit: String, mode: String, files: Vec<BundleFile>) -> Self {
        Self { emit, mode, files }
    }

    /// The emitted artifact family.
    #[wasm_bindgen(getter, js_name = "emit")]
    pub fn emit(&self) -> String {
        self.emit.clone()
    }

    /// The target-level assembly mode.
    #[wasm_bindgen(getter, js_name = "mode")]
    pub fn mode(&self) -> String {
        self.mode.clone()
    }

    /// The files in this bundle.
    #[wasm_bindgen(getter, js_name = "files")]
    pub fn files(&self) -> Vec<BundleFile> {
        self.files.clone()
    }
}

impl Bundle {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Bundle) -> Self {
        Self {
            emit: emit_format_label(value.emit),
            mode: bundle_mode_label(value.mode),
            files: value
                .files
                .into_iter()
                .map(|item| BundleFile::from_bridge(item))
                .collect(),
        }
    }
}

/// Durable program header crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ProgramHeader {
    name: Option<String>,
    fingerprint: Option<String>,
    target: Option<String>,
}

#[wasm_bindgen]
impl ProgramHeader {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: Option<String>, fingerprint: Option<String>, target: Option<String>) -> Self {
        Self {
            name,
            fingerprint,
            target,
        }
    }

    /// Human-facing program name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Build fingerprint that produced this program.
    #[wasm_bindgen(getter, js_name = "fingerprint")]
    pub fn fingerprint(&self) -> Option<String> {
        self.fingerprint.clone()
    }

    /// Target triple or equivalent target identity.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> Option<String> {
        self.target.clone()
    }
}

impl ProgramHeader {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ProgramHeader) -> Self {
        Self {
            name: value.name,
            fingerprint: value.fingerprint,
            target: value.target,
        }
    }
}

/// Durable executable program crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Program {
    header: ProgramHeader,
    format: String,
    contents: Vec<ContentId>,
}

#[wasm_bindgen]
impl Program {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(header: ProgramHeader, format: String, contents: Vec<ContentId>) -> Self {
        Self {
            header,
            format,
            contents,
        }
    }

    /// The program identity and compatibility header.
    #[wasm_bindgen(getter, js_name = "header")]
    pub fn header(&self) -> ProgramHeader {
        self.header.clone()
    }

    /// The executable format.
    #[wasm_bindgen(getter, js_name = "format")]
    pub fn format(&self) -> String {
        self.format.clone()
    }

    /// Content blobs referenced by the executable payload.
    #[wasm_bindgen(getter, js_name = "contents")]
    pub fn contents(&self) -> Vec<ContentId> {
        self.contents.clone()
    }
}

impl Program {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Program) -> Self {
        Self {
            header: ProgramHeader::from_bridge(value.header),
            format: program_format_label(value.format),
            contents: value
                .contents
                .into_iter()
                .map(|item| ContentId::from_bridge(item))
                .collect(),
        }
    }
}

/// One linked product target crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ProductTarget {
    name: String,
    target: TargetId,
    runtime: String,
    host: String,
    platform: String,
    includes_build: bool,
    includes_bundle: bool,
    includes_program: bool,
}

#[wasm_bindgen]
impl ProductTarget {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: String,
        target: TargetId,
        runtime: String,
        host: String,
        platform: String,
        includes_build: bool,
        includes_bundle: bool,
        includes_program: bool,
    ) -> Self {
        Self {
            name,
            target,
            runtime,
            host,
            platform,
            includes_build,
            includes_bundle,
            includes_program,
        }
    }

    /// The configured product target name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The repository target assembled into this product.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> TargetId {
        self.target.clone()
    }

    /// The runtime contract this target expects.
    #[wasm_bindgen(getter, js_name = "runtime")]
    pub fn runtime(&self) -> String {
        self.runtime.clone()
    }

    /// The host environment this target expects.
    #[wasm_bindgen(getter, js_name = "host")]
    pub fn host(&self) -> String {
        self.host.clone()
    }

    /// The platform this target expects.
    #[wasm_bindgen(getter, js_name = "platform")]
    pub fn platform(&self) -> String {
        self.platform.clone()
    }

    /// Whether this product target includes its toolchain build payload.
    #[wasm_bindgen(getter, js_name = "includesBuild")]
    pub fn includes_build(&self) -> bool {
        self.includes_build
    }

    /// Whether this product target includes its linked bundle.
    #[wasm_bindgen(getter, js_name = "includesBundle")]
    pub fn includes_bundle(&self) -> bool {
        self.includes_bundle
    }

    /// Whether this product target includes its executable program.
    #[wasm_bindgen(getter, js_name = "includesProgram")]
    pub fn includes_program(&self) -> bool {
        self.includes_program
    }
}

impl ProductTarget {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ProductTarget) -> Self {
        Self {
            name: value.name,
            target: TargetId::from_bridge(value.target),
            runtime: runtime_label(value.runtime),
            host: host_label(value.host),
            platform: value.platform,
            includes_build: value.includes_build,
            includes_bundle: value.includes_bundle,
            includes_program: value.includes_program,
        }
    }
}

/// One linked product crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Product {
    name: String,
    targets: Vec<ProductTarget>,
}

#[wasm_bindgen]
impl Product {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, targets: Vec<ProductTarget>) -> Self {
        Self { name, targets }
    }

    /// The configured product name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The linked targets in deterministic order.
    #[wasm_bindgen(getter, js_name = "targets")]
    pub fn targets(&self) -> Vec<ProductTarget> {
        self.targets.clone()
    }
}

impl Product {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Product) -> Self {
        Self {
            name: value.name,
            targets: value
                .targets
                .into_iter()
                .map(|item| ProductTarget::from_bridge(item))
                .collect(),
        }
    }
}

/// One language build request.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct BuildRequest {
    content: BuildRequestContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum BuildRequestContent {
    /// Build one module artifact.
    Module {
        /// Source module.
        module: Module,
        /// Build target.
        target: TargetId,
        /// Requested module artifact family.
        output: String,
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

#[wasm_bindgen]
impl BuildRequest {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "module")]
    pub fn module(module: Module, target: TargetId, output: String) -> Self {
        Self {
            content: BuildRequestContent::Module {
                module,
                target,
                output,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "build")]
    pub fn build(target: TargetId) -> Self {
        Self {
            content: BuildRequestContent::Build { target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "target")]
    pub fn target(target: TargetId) -> Self {
        Self {
            content: BuildRequestContent::Target { target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "product")]
    pub fn product(product: ProductId) -> Self {
        Self {
            content: BuildRequestContent::Product { product },
        }
    }
}

impl BuildRequest {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::BuildRequest {
        match self.content {
            BuildRequestContent::Module {
                module,
                target,
                output,
            } => bridge::BuildRequest::Module {
                module: module.into_bridge(),
                target: target.into_bridge(),
                output: parse_module_build_kind(output.as_str()),
            },
            BuildRequestContent::Build { target } => bridge::BuildRequest::Build {
                target: target.into_bridge(),
            },
            BuildRequestContent::Target { target } => bridge::BuildRequest::Target {
                target: target.into_bridge(),
            },
            BuildRequestContent::Product { product } => bridge::BuildRequest::Product {
                product: product.into_bridge(),
            },
        }
    }
}

/// One language build output.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct BuildOutput {
    content: BuildOutputContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum BuildOutputContent {
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

#[wasm_bindgen]
impl BuildOutput {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "script")]
    pub fn script(version: ArtifactVersion, script: Script) -> Self {
        Self {
            content: BuildOutputContent::Script { version, script },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "object")]
    pub fn object(version: ArtifactVersion, object: Object) -> Self {
        Self {
            content: BuildOutputContent::Object { version, object },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "asset")]
    pub fn asset(version: ArtifactVersion, asset: Asset) -> Self {
        Self {
            content: BuildOutputContent::Asset { version, asset },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "build")]
    pub fn build(version: ArtifactVersion, build: Build) -> Self {
        Self {
            content: BuildOutputContent::Build { version, build },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "bundle")]
    pub fn bundle(version: ArtifactVersion, bundle: Bundle) -> Self {
        Self {
            content: BuildOutputContent::Bundle { version, bundle },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "program")]
    pub fn program(version: ArtifactVersion, program: Program) -> Self {
        Self {
            content: BuildOutputContent::Program { version, program },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "product")]
    pub fn product(version: ArtifactVersion, product: Product) -> Self {
        Self {
            content: BuildOutputContent::Product { version, product },
        }
    }

    /// Payload variant label.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        let label = match &self.content {
            BuildOutputContent::Script { .. } => "script",
            BuildOutputContent::Object { .. } => "object",
            BuildOutputContent::Asset { .. } => "asset",
            BuildOutputContent::Build { .. } => "build",
            BuildOutputContent::Bundle { .. } => "bundle",
            BuildOutputContent::Program { .. } => "program",
            BuildOutputContent::Product { .. } => "product",
        };
        label.to_string()
    }

    /// Exact artifact version.
    #[wasm_bindgen(js_name = "getVersion")]
    pub fn get_version(&self) -> Option<ArtifactVersion> {
        match &self.content {
            BuildOutputContent::Script { version: value, .. } => Some(value.clone()),
            BuildOutputContent::Object { version: value, .. } => Some(value.clone()),
            BuildOutputContent::Asset { version: value, .. } => Some(value.clone()),
            BuildOutputContent::Build { version: value, .. } => Some(value.clone()),
            BuildOutputContent::Bundle { version: value, .. } => Some(value.clone()),
            BuildOutputContent::Program { version: value, .. } => Some(value.clone()),
            BuildOutputContent::Product { version: value, .. } => Some(value.clone()),
        }
    }

    /// Script payload.
    #[wasm_bindgen(js_name = "getScriptScript")]
    pub fn get_script_script(&self) -> Option<Script> {
        match &self.content {
            BuildOutputContent::Script { script: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Object payload.
    #[wasm_bindgen(js_name = "getObjectObject")]
    pub fn get_object_object(&self) -> Option<Object> {
        match &self.content {
            BuildOutputContent::Object { object: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Asset payload.
    #[wasm_bindgen(js_name = "getAssetAsset")]
    pub fn get_asset_asset(&self) -> Option<Asset> {
        match &self.content {
            BuildOutputContent::Asset { asset: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Build payload.
    #[wasm_bindgen(js_name = "getBuildBuild")]
    pub fn get_build_build(&self) -> Option<Build> {
        match &self.content {
            BuildOutputContent::Build { build: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Bundle payload.
    #[wasm_bindgen(js_name = "getBundleBundle")]
    pub fn get_bundle_bundle(&self) -> Option<Bundle> {
        match &self.content {
            BuildOutputContent::Bundle { bundle: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Program payload.
    #[wasm_bindgen(js_name = "getProgramProgram")]
    pub fn get_program_program(&self) -> Option<Program> {
        match &self.content {
            BuildOutputContent::Program { program: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Product payload.
    #[wasm_bindgen(js_name = "getProductProduct")]
    pub fn get_product_product(&self) -> Option<Product> {
        match &self.content {
            BuildOutputContent::Product { product: value, .. } => Some(value.clone()),
            _ => None,
        }
    }
}

impl BuildOutput {
    /// Convert one bridge payload enum into one WASM payload enum.
    pub(crate) fn from_bridge(value: bridge::BuildOutput) -> Self {
        match value {
            bridge::BuildOutput::Script { version, script } => Self {
                content: BuildOutputContent::Script {
                    version: ArtifactVersion::from_bridge(version),
                    script: Script::from_bridge(script),
                },
            },
            bridge::BuildOutput::Object { version, object } => Self {
                content: BuildOutputContent::Object {
                    version: ArtifactVersion::from_bridge(version),
                    object: Object::from_bridge(object),
                },
            },
            bridge::BuildOutput::Asset { version, asset } => Self {
                content: BuildOutputContent::Asset {
                    version: ArtifactVersion::from_bridge(version),
                    asset: Asset::from_bridge(asset),
                },
            },
            bridge::BuildOutput::Build { version, build } => Self {
                content: BuildOutputContent::Build {
                    version: ArtifactVersion::from_bridge(version),
                    build: Build::from_bridge(build),
                },
            },
            bridge::BuildOutput::Bundle { version, bundle } => Self {
                content: BuildOutputContent::Bundle {
                    version: ArtifactVersion::from_bridge(version),
                    bundle: Bundle::from_bridge(bundle),
                },
            },
            bridge::BuildOutput::Program { version, program } => Self {
                content: BuildOutputContent::Program {
                    version: ArtifactVersion::from_bridge(version),
                    program: Program::from_bridge(program),
                },
            },
            bridge::BuildOutput::Product { version, product } => Self {
                content: BuildOutputContent::Product {
                    version: ArtifactVersion::from_bridge(version),
                    product: Product::from_bridge(product),
                },
            },
        }
    }
}

/// Return one target enum label.
fn build_profile_label(value: bridge::BuildProfile) -> String {
    let label = match value {
        bridge::BuildProfile::Full => "full",
        bridge::BuildProfile::Minimal => "minimal",
        bridge::BuildProfile::Freestanding => "freestanding",
    };
    label.to_string()
}

/// Return one target enum label.
fn build_linkage_label(value: bridge::BuildLinkage) -> String {
    let label = match value {
        bridge::BuildLinkage::Portable => "portable",
        bridge::BuildLinkage::Static => "static",
        bridge::BuildLinkage::Dynamic => "dynamic",
    };
    label.to_string()
}

/// Return one target enum label.
fn emit_format_label(value: bridge::EmitFormat) -> String {
    let label = match value {
        bridge::EmitFormat::Js => "js",
        bridge::EmitFormat::Ts => "ts",
        bridge::EmitFormat::Wasm => "wasm",
        bridge::EmitFormat::Native => "native",
    };
    label.to_string()
}

/// Return one target enum label.
fn file_type_label(value: bridge::FileType) -> String {
    let label = match value {
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
    };
    label.to_string()
}

/// Return one target enum label.
fn script_language_label(value: bridge::ScriptLanguage) -> String {
    let label = match value {
        bridge::ScriptLanguage::JavaScript => "javaScript",
        bridge::ScriptLanguage::TypeScript => "typeScript",
    };
    label.to_string()
}

/// Return one target enum label.
fn object_format_label(value: bridge::ObjectFormat) -> String {
    let label = match value {
        bridge::ObjectFormat::Object => "object",
        bridge::ObjectFormat::Wasm => "wasm",
    };
    label.to_string()
}

/// Return one target enum label.
fn bundle_section_label(value: bridge::BundleSection) -> String {
    let label = match value {
        bridge::BundleSection::Module => "module",
        bridge::BundleSection::Entry => "entry",
        bridge::BundleSection::Declaration => "declaration",
        bridge::BundleSection::Asset => "asset",
        bridge::BundleSection::Manifest => "manifest",
        bridge::BundleSection::SourceMap => "sourceMap",
        bridge::BundleSection::Native => "native",
    };
    label.to_string()
}

/// Return one target enum label.
fn bundle_mode_label(value: bridge::BundleMode) -> String {
    let label = match value {
        bridge::BundleMode::PreserveModules => "preserveModules",
        bridge::BundleMode::SingleFile => "singleFile",
        bridge::BundleMode::Chunked => "chunked",
    };
    label.to_string()
}

/// Return one target enum label.
fn program_format_label(value: bridge::ProgramFormat) -> String {
    let label = match value {
        bridge::ProgramFormat::Vm => "vm",
        bridge::ProgramFormat::Native => "native",
    };
    label.to_string()
}

/// Return one target enum label.
fn runtime_label(value: bridge::Runtime) -> String {
    let label = match value {
        bridge::Runtime::Destack => "destack",
        bridge::Runtime::Js => "js",
    };
    label.to_string()
}

/// Return one target enum label.
fn host_label(value: bridge::Host) -> String {
    let label = match value {
        bridge::Host::Native => "native",
        bridge::Host::Browser => "browser",
        bridge::Host::Wasi => "wasi",
        bridge::Host::Emscripten => "emscripten",
        bridge::Host::Freestanding => "freestanding",
    };
    label.to_string()
}

/// Parse one unit enum label.
fn parse_module_build_kind(value: &str) -> bridge::ModuleBuildKind {
    match value {
        "script" => bridge::ModuleBuildKind::Script,
        "object" => bridge::ModuleBuildKind::Object,
        "asset" => bridge::ModuleBuildKind::Asset,
        _ => wasm_bindgen::throw_str(&format!("unknown {}: {value}", stringify!(ModuleBuildKind))),
    }
}
