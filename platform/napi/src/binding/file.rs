use destack_source as source;
use napi_derive::napi;

/// The format of a source file.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// `.ds`
    Destack,
    /// `.d.ds`
    DestackDeclaration,
    /// `.dst`
    DestackText,
    /// `.dsb`
    DestackBinary,
    /// `.js`
    JavaScript,
    /// `.jsx`
    JavaScriptXml,
    /// `.ts`
    TypeScript,
    /// `.tsx`
    TypeScriptXml,
    /// `.d.ts`
    TypeScriptDeclaration,
    /// `.txt`
    Text,
    /// `.toml`
    Toml,
    /// `.yaml`, `.yml`
    Yaml,
    /// `.json`
    Json,
    /// `.env`
    Env,
    /// `.html`, `.htm`
    Html,
    /// `.md`
    Markdown,
    /// `.css`
    Css,
    /// `.svg`
    Svg,
    /// `.wasm`
    Wasm,
    /// `.node`
    Node,
    /// Source map `.map`.
    SourceMap,
    /// Object file `.o`.
    Object,
    /// Destack AST cache `.ast`.
    DestackAst,
    /// Destack DIR cache `.dir`.
    DestackDir,
    /// Destack MIR cache `.mir`, `.dsmir`.
    DestackMir,
    /// Image files.
    Image,
    /// Font files.
    Font,
    /// Audio files.
    Audio,
    /// Video files.
    Video,
    /// 3D model files.
    Model,
    /// AI and ML model files.
    Neural,
    /// Document files.
    Document,
    /// Binary unknown format.
    Binary,
    /// Unknown file type.
    Unknown,
}

impl From<FileType> for source::FileType {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Destack => source::FileType::Destack,
            FileType::DestackDeclaration => source::FileType::DestackDeclaration,
            FileType::DestackText => source::FileType::DestackText,
            FileType::DestackBinary => source::FileType::DestackBinary,
            FileType::JavaScript => source::FileType::JavaScript,
            FileType::JavaScriptXml => source::FileType::JavaScriptXml,
            FileType::TypeScript => source::FileType::TypeScript,
            FileType::TypeScriptXml => source::FileType::TypeScriptXml,
            FileType::TypeScriptDeclaration => source::FileType::TypeScriptDeclaration,
            FileType::Text => source::FileType::Text,
            FileType::Toml => source::FileType::Toml,
            FileType::Yaml => source::FileType::Yaml,
            FileType::Json => source::FileType::Json,
            FileType::Env => source::FileType::Env,
            FileType::Html => source::FileType::Html,
            FileType::Markdown => source::FileType::Markdown,
            FileType::Css => source::FileType::Css,
            FileType::Svg => source::FileType::Svg,
            FileType::Wasm => source::FileType::Wasm,
            FileType::Node => source::FileType::Node,
            FileType::SourceMap => source::FileType::SourceMap,
            FileType::Object => source::FileType::Object,
            FileType::DestackAst => source::FileType::DestackAst,
            FileType::DestackDir => source::FileType::DestackDir,
            FileType::DestackMir => source::FileType::DestackMir,
            FileType::Image => source::FileType::Image,
            FileType::Font => source::FileType::Font,
            FileType::Audio => source::FileType::Audio,
            FileType::Video => source::FileType::Video,
            FileType::Model => source::FileType::Model,
            FileType::Neural => source::FileType::Neural,
            FileType::Document => source::FileType::Document,
            FileType::Binary => source::FileType::Binary,
            FileType::Unknown => source::FileType::Unknown,
        }
    }
}

impl From<source::FileType> for FileType {
    fn from(file_type: source::FileType) -> Self {
        match file_type {
            source::FileType::Destack => FileType::Destack,
            source::FileType::DestackDeclaration => FileType::DestackDeclaration,
            source::FileType::DestackText => FileType::DestackText,
            source::FileType::DestackBinary => FileType::DestackBinary,
            source::FileType::JavaScript => FileType::JavaScript,
            source::FileType::JavaScriptXml => FileType::JavaScriptXml,
            source::FileType::TypeScript => FileType::TypeScript,
            source::FileType::TypeScriptXml => FileType::TypeScriptXml,
            source::FileType::TypeScriptDeclaration => FileType::TypeScriptDeclaration,
            source::FileType::Text => FileType::Text,
            source::FileType::Toml => FileType::Toml,
            source::FileType::Yaml => FileType::Yaml,
            source::FileType::Json => FileType::Json,
            source::FileType::Env => FileType::Env,
            source::FileType::Html => FileType::Html,
            source::FileType::Markdown => FileType::Markdown,
            source::FileType::Css => FileType::Css,
            source::FileType::Svg => FileType::Svg,
            source::FileType::Wasm => FileType::Wasm,
            source::FileType::Node => FileType::Node,
            source::FileType::SourceMap => FileType::SourceMap,
            source::FileType::Object => FileType::Object,
            source::FileType::DestackAst => FileType::DestackAst,
            source::FileType::DestackDir => FileType::DestackDir,
            source::FileType::DestackMir => FileType::DestackMir,
            source::FileType::Image => FileType::Image,
            source::FileType::Font => FileType::Font,
            source::FileType::Audio => FileType::Audio,
            source::FileType::Video => FileType::Video,
            source::FileType::Model => FileType::Model,
            source::FileType::Neural => FileType::Neural,
            source::FileType::Document => FileType::Document,
            source::FileType::Binary => FileType::Binary,
            source::FileType::Unknown => FileType::Unknown,
        }
    }
}
