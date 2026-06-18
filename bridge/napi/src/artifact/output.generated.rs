// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ArtifactVersion, ContentId, Module, ProductId, TargetId};

/// One emitted or linked source map crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "SourceMapSource")]
pub struct SourceMapSource {
    /// The mapped source names.
    pub name: String,
    /// The embedded source contents when they exist.
    pub content: Option<String>,
}

impl SourceMapSource {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::SourceMapSource) -> Self {
        Self {
            name: value.name,
            content: value.content,
        }
    }
}

/// One emitted or linked source map crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "SourceMap")]
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

impl SourceMap {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "Declaration")]
pub struct Declaration {
    /// The declaration text.
    pub text: String,
}

impl Declaration {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Declaration) -> Self {
        Self { text: value.text }
    }
}

/// One structured script artifact crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "Script")]
pub struct Script {
    /// The target language of this script.
    pub language: String,
    /// The emitted declaration when one exists.
    pub declaration: Option<Declaration>,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
    /// Whether this script has top-level side effects.
    pub has_top_level_side_effects: bool,
}

impl Script {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "BridgeObject")]
pub struct BridgeObject {
    /// The compiled-code object format.
    pub format: String,
    /// The encoded object content identity.
    pub content: ContentId,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
}

impl BridgeObject {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Object) -> Self {
        Self {
            format: object_format_label(value.format),
            content: ContentId::from_bridge(value.content),
            map: value.map.map(|item| SourceMap::from_bridge(item)),
        }
    }
}

/// One opaque asset artifact crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "Asset")]
pub struct Asset {
    /// The asset file type.
    pub file_type: String,
    /// The asset content identity.
    pub content: ContentId,
    /// The source module URI when one exists.
    pub source: Option<String>,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
}

impl Asset {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "Build")]
pub struct Build {
    /// The build distribution profile.
    pub profile: String,
    /// The build linkage.
    pub linkage: String,
    /// The encoded build content.
    pub content: ContentId,
}

impl Build {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Build) -> Self {
        Self {
            profile: build_profile_label(value.profile),
            linkage: build_linkage_label(value.linkage),
            content: ContentId::from_bridge(value.content),
        }
    }
}

/// One derived bundle file crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "BundleFile")]
pub struct BundleFile {
    /// The bundle section this file belongs to.
    pub section: String,
    /// The output URI.
    pub uri: String,
    /// The emitted file type.
    pub file_type: String,
    /// The output content identity.
    pub content: ContentId,
    /// The related source URI when one exists.
    pub source: Option<String>,
}

impl BundleFile {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "Bundle")]
pub struct Bundle {
    /// The emitted artifact family.
    pub emit: String,
    /// The target-level assembly mode.
    pub mode: String,
    /// The files in this bundle.
    pub files: Vec<BundleFile>,
}

impl Bundle {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "ProgramHeader")]
pub struct ProgramHeader {
    /// Human-facing program name.
    pub name: Option<String>,
    /// Build fingerprint that produced this program.
    pub fingerprint: Option<String>,
    /// Target triple or equivalent target identity.
    pub target: Option<String>,
}

impl ProgramHeader {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ProgramHeader) -> Self {
        Self {
            name: value.name,
            fingerprint: value.fingerprint,
            target: value.target,
        }
    }
}

/// Durable executable program crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "Program")]
pub struct Program {
    /// The program identity and compatibility header.
    pub header: ProgramHeader,
    /// The executable format.
    pub format: String,
    /// Content blobs referenced by the executable payload.
    pub contents: Vec<ContentId>,
}

impl Program {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "ProductTarget")]
pub struct ProductTarget {
    /// The configured product target name.
    pub name: String,
    /// The repository target assembled into this product.
    pub target: TargetId,
    /// The runtime contract this target expects.
    pub runtime: String,
    /// The host environment this target expects.
    pub host: String,
    /// The platform this target expects.
    pub platform: String,
    /// Whether this product target includes its toolchain build payload.
    pub includes_build: bool,
    /// Whether this product target includes its linked bundle.
    pub includes_bundle: bool,
    /// Whether this product target includes its executable program.
    pub includes_program: bool,
}

impl ProductTarget {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "Product")]
pub struct Product {
    /// The configured product name.
    pub name: String,
    /// The linked targets in deterministic order.
    pub targets: Vec<ProductTarget>,
}

impl Product {
    /// Convert one bridge value into one NAPI value.
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
#[derive(Debug)]
#[napi(object, js_name = "BuildRequest")]
pub struct BuildRequest {
    /// Payload variant label.
    pub kind: String,
    /// Source module.
    pub module_module: Option<Module>,
    /// Build target.
    pub target: Option<TargetId>,
    /// Requested module artifact family.
    pub output: Option<String>,
    /// Build target.
    pub target_target: Option<TargetId>,
    /// Product id.
    pub product_product: Option<ProductId>,
}

impl BuildRequest {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::BuildRequest> {
        match self.kind.as_str() {
            "module" => {
                if self.target_target.is_some() {
                    return Err(unexpected_payload("target_target"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module_module else {
                    return Err(missing_payload("moduleModule"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                let Some(value) = self.output else {
                    return Err(missing_payload("output"));
                };
                let output = parse_module_build_kind(value.as_str())?;
                Ok(bridge::BuildRequest::Module {
                    module,
                    target,
                    output,
                })
            }
            "build" => {
                if self.module_module.is_some() {
                    return Err(unexpected_payload("module_module"));
                }
                if self.output.is_some() {
                    return Err(unexpected_payload("output"));
                }
                if self.target_target.is_some() {
                    return Err(unexpected_payload("target_target"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::BuildRequest::Build { target })
            }
            "target" => {
                if self.module_module.is_some() {
                    return Err(unexpected_payload("module_module"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.output.is_some() {
                    return Err(unexpected_payload("output"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.target_target else {
                    return Err(missing_payload("targetTarget"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::BuildRequest::Target { target })
            }
            "product" => {
                if self.module_module.is_some() {
                    return Err(unexpected_payload("module_module"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.output.is_some() {
                    return Err(unexpected_payload("output"));
                }
                if self.target_target.is_some() {
                    return Err(unexpected_payload("target_target"));
                }
                let Some(value) = self.product_product else {
                    return Err(missing_payload("productProduct"));
                };
                let product = value.into_bridge()?;
                Ok(bridge::BuildRequest::Product { product })
            }
            _ => Err(napi::Error::from_reason(format!(
                "unknown {}: {}",
                stringify!(BuildRequest),
                self.kind
            ))),
        }
    }
}

/// Return one missing payload error.
fn missing_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is missing"))
}

/// Return one unexpected payload error.
fn unexpected_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is unexpected"))
}

/// One language build output.
#[derive(Debug)]
#[napi(object, js_name = "BuildOutput")]
pub struct BuildOutput {
    /// Payload variant label.
    pub kind: String,
    /// Exact artifact version.
    pub version: Option<ArtifactVersion>,
    /// Script payload.
    pub script_script: Option<Script>,
    /// Object payload.
    pub object_object: Option<BridgeObject>,
    /// Asset payload.
    pub asset_asset: Option<Asset>,
    /// Build payload.
    pub build_build: Option<Build>,
    /// Bundle payload.
    pub bundle_bundle: Option<Bundle>,
    /// Program payload.
    pub program_program: Option<Program>,
    /// Product payload.
    pub product_product: Option<Product>,
}

impl BuildOutput {
    /// Convert one bridge payload enum into one NAPI payload enum.
    pub(crate) fn from_bridge(value: bridge::BuildOutput) -> Self {
        match value {
            bridge::BuildOutput::Script { version, script } => Self {
                kind: "script".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                script_script: Some(Script::from_bridge(script)),
                object_object: None,
                asset_asset: None,
                build_build: None,
                bundle_bundle: None,
                program_program: None,
                product_product: None,
            },
            bridge::BuildOutput::Object { version, object } => Self {
                kind: "object".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                object_object: Some(BridgeObject::from_bridge(object)),
                script_script: None,
                asset_asset: None,
                build_build: None,
                bundle_bundle: None,
                program_program: None,
                product_product: None,
            },
            bridge::BuildOutput::Asset { version, asset } => Self {
                kind: "asset".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                asset_asset: Some(Asset::from_bridge(asset)),
                script_script: None,
                object_object: None,
                build_build: None,
                bundle_bundle: None,
                program_program: None,
                product_product: None,
            },
            bridge::BuildOutput::Build { version, build } => Self {
                kind: "build".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                build_build: Some(Build::from_bridge(build)),
                script_script: None,
                object_object: None,
                asset_asset: None,
                bundle_bundle: None,
                program_program: None,
                product_product: None,
            },
            bridge::BuildOutput::Bundle { version, bundle } => Self {
                kind: "bundle".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                bundle_bundle: Some(Bundle::from_bridge(bundle)),
                script_script: None,
                object_object: None,
                asset_asset: None,
                build_build: None,
                program_program: None,
                product_product: None,
            },
            bridge::BuildOutput::Program { version, program } => Self {
                kind: "program".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                program_program: Some(Program::from_bridge(program)),
                script_script: None,
                object_object: None,
                asset_asset: None,
                build_build: None,
                bundle_bundle: None,
                product_product: None,
            },
            bridge::BuildOutput::Product { version, product } => Self {
                kind: "product".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                product_product: Some(Product::from_bridge(product)),
                script_script: None,
                object_object: None,
                asset_asset: None,
                build_build: None,
                bundle_bundle: None,
                program_program: None,
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

/// Parse one target enum label.
fn parse_module_build_kind(value: &str) -> napi::Result<bridge::ModuleBuildKind> {
    match value {
        "script" => Ok(bridge::ModuleBuildKind::Script),
        "object" => Ok(bridge::ModuleBuildKind::Object),
        "asset" => Ok(bridge::ModuleBuildKind::Asset),
        _ => Err(napi::Error::from_reason(format!(
            "unknown {}: {value}",
            stringify!(ModuleBuildKind),
        ))),
    }
}
