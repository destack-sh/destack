use std::path::Path;

use serde::{Deserialize, Serialize};

/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    // destack
    /// `.ds`
    Destack,
    /// `.d.ds`
    DestackDeclaration,

    // javascript/typescript compatibility
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

    // data
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

    // markup/styling
    /// `.html`, `.htm`
    Html,
    /// `.md`
    Markdown,
    /// `.css`
    Css,
    /// `.svg`
    Svg,

    // binary/system
    /// `.wasm`
    Wasm,
    /// `.node`
    Node,
    /// Source map `.map`
    SourceMap,
    /// Object file `.o`
    Object,

    // media: coarse categories (pass-through)
    /// Image files (png, jpg, gif, webp, avif, ico, bmp, tiff, dds, tga, exr, hdr, psd)
    Image,
    /// Font files (woff, woff2, ttf, otf, eot)
    Font,
    /// Audio files (mp3, wav, ogg, flac, aac, m4a, opus, mid, midi)
    Audio,
    /// Video files (mp4, webm, mov, avi, mkv, flv)
    Video,
    /// 3D model files (gltf, glb, obj, fbx, dae, stl, blend, 3ds)
    Model,
    /// AI/ML model weights (onnx, safetensors, pt, pth, h5, tflite, mlmodel, gguf, ggml)
    Neural,
    /// Document files (pdf, doc, docx, xls, xlsx, ppt, pptx, odt, ods, odp, rtf, epub, mobi)
    Document,

    // fallback
    /// Binary (unknown binary format).
    Binary,
    /// Unknown file type.
    Unknown,
}

/// File types that should be watched for source changes.
/// NOTE #Architecture: do we need WATCHABLE_FILE_TYPES?
pub const WATCHABLE_FILE_TYPES: &[FileType] = &[
    FileType::Destack,
    FileType::DestackDeclaration,
    FileType::JavaScript,
    FileType::JavaScriptXml,
    FileType::TypeScript,
    FileType::TypeScriptXml,
    FileType::TypeScriptDeclaration,
    FileType::Text,
    FileType::Toml,
    FileType::Yaml,
    FileType::Json,
    FileType::Env,
    FileType::Markdown,
    FileType::Html,
    FileType::Css,
    FileType::Svg,
    FileType::SourceMap,
];

impl FileType {
    /// The source code file types used for extensionless module resolution.
    pub const CODE_MODULE_EXTENSION_CANDIDATES: &'static [Self] = &[
        Self::Destack,
        Self::DestackDeclaration,
        Self::TypeScript,
        Self::TypeScriptXml,
        Self::JavaScript,
        Self::JavaScriptXml,
    ];

    /// Get a source format from a file extension.
    pub fn from_extension(s: &str) -> Option<Self> {
        let ty = match s {
            // destack
            "ds" => FileType::Destack,
            "d.ds" => FileType::DestackDeclaration,

            // javascript/typescript
            "js" => FileType::JavaScript,
            "jsx" => FileType::JavaScriptXml,
            "ts" => FileType::TypeScript,
            "tsx" => FileType::TypeScriptXml,
            "d.ts" => FileType::TypeScriptDeclaration,

            // data formats
            "txt" => FileType::Text,
            "toml" => FileType::Toml,
            "yaml" | "yml" => FileType::Yaml,
            "json" => FileType::Json,
            "env" => FileType::Env,

            // markup/styling
            "html" | "htm" => FileType::Html,
            "md" => FileType::Markdown,
            "css" => FileType::Css,
            "svg" => FileType::Svg,

            // binary/system
            "wasm" => FileType::Wasm,
            "node" => FileType::Node,
            "map" => FileType::SourceMap,
            "o" => FileType::Object,

            // images
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "ico" | "bmp" | "tiff" | "tif"
            | "dds" | "tga" | "exr" | "hdr" | "psd" => FileType::Image,

            // fonts
            "woff" | "woff2" | "ttf" | "otf" | "eot" => FileType::Font,

            // audio
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "opus" | "mid" | "midi" => {
                FileType::Audio
            }

            // video
            "mp4" | "webm" | "mov" | "avi" | "mkv" | "flv" => FileType::Video,

            // 3D models
            "gltf" | "glb" | "obj" | "fbx" | "dae" | "stl" | "blend" | "3ds" => FileType::Model,

            // AI/ML weights
            "onnx" | "safetensors" | "pt" | "pth" | "h5" | "tflite" | "mlmodel" | "gguf"
            | "ggml" => FileType::Neural,

            // documents
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
            | "rtf" | "epub" | "mobi" | "pages" | "numbers" | "keynote" => FileType::Document,

            _ => return None,
        };

        Some(ty)
    }

    /// Get the file extension from an extension or unknown.
    pub fn from_extension_or_unknown(s: &str) -> Self {
        Self::from_extension(s).unwrap_or(FileType::Unknown)
    }

    /// Get a source format from a file path.
    pub fn from_path(path: &Path) -> Option<Self> {
        // detect compound extensions first
        if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
            if file_name.ends_with(".d.ds") {
                return Some(FileType::DestackDeclaration);
            }
            if file_name.ends_with(".d.ts") {
                return Some(FileType::TypeScriptDeclaration);
            }
        }

        // fall back to the simple extension
        let extension = path.extension().and_then(|ext| ext.to_str());
        extension.and_then(FileType::from_extension)
    }

    /// Get a source format from a file path, defaulting to unknown.
    pub fn from_path_or_unknown(path: &Path) -> Self {
        Self::from_path(path).unwrap_or(FileType::Unknown)
    }
}

impl FileType {
    /// Get the file extension for a source format.
    ///
    /// For coarse categories (Image, Font, etc.) returns None since they map to multiple extensions.
    pub fn extension(&self) -> Option<&str> {
        let extension = match self {
            // destack
            FileType::Destack => "ds",
            FileType::DestackDeclaration => "d.ds",

            // javascript/typescript
            FileType::JavaScript => "js",
            FileType::JavaScriptXml => "jsx",
            FileType::TypeScript => "ts",
            FileType::TypeScriptXml => "tsx",
            FileType::TypeScriptDeclaration => "d.ts",

            // data formats
            FileType::Text => "txt",
            FileType::Toml => "toml",
            FileType::Yaml => "yaml",
            FileType::Json => "json",
            FileType::Env => "env",

            // markup/styling
            FileType::Html => "html",
            FileType::Markdown => "md",
            FileType::Css => "css",
            FileType::Svg => "svg",

            // binary/system
            FileType::Wasm => "wasm",
            FileType::Node => "node",
            FileType::SourceMap => "map",
            FileType::Object => "o",

            // coarse categories have no single extension
            FileType::Image
            | FileType::Font
            | FileType::Audio
            | FileType::Video
            | FileType::Model
            | FileType::Neural
            | FileType::Document
            | FileType::Binary
            | FileType::Unknown => return None,
        };
        Some(extension)
    }

    /// Get the glob pattern for a source format.
    ///
    /// For multi-extension types, this returns the first canonical pattern.
    /// Prefer [`FileType::globs`] when enumerating all variants.
    pub fn glob(&self) -> Option<&str> {
        self.globs().first().copied()
    }

    /// Whether this file type is a code file (can be parsed as code).
    pub fn is_code(&self) -> bool {
        matches!(
            self,
            FileType::Destack
                | FileType::DestackDeclaration
                | FileType::JavaScript
                | FileType::JavaScriptXml
                | FileType::TypeScript
                | FileType::TypeScriptXml
                | FileType::TypeScriptDeclaration
        )
    }

    /// Whether this file type is a data file (JSON, TOML, YAML, etc.).
    pub fn is_data(&self) -> bool {
        matches!(self, FileType::Json | FileType::Toml | FileType::Yaml)
    }

    /// Whether this file type is a text file.
    pub fn is_text(&self) -> bool {
        matches!(
            self,
            FileType::Text
                | FileType::Markdown
                | FileType::Html
                | FileType::Css
                | FileType::Svg
                | FileType::Env
                | FileType::SourceMap
        )
    }

    /// Whether this file type is a binary file.
    pub fn is_binary(&self) -> bool {
        matches!(
            self,
            FileType::Wasm
                | FileType::Node
                | FileType::Object
                | FileType::Image
                | FileType::Font
                | FileType::Audio
                | FileType::Video
                | FileType::Model
                | FileType::Neural
                | FileType::Document
                | FileType::Binary
        )
    }

    /// Get glob patterns for a file type.
    ///
    /// Some file types expand into multiple glob patterns.
    pub fn globs(&self) -> &'static [&'static str] {
        match self {
            FileType::Destack => &["**/*.ds"],
            FileType::DestackDeclaration => &["**/*.d.ds"],
            FileType::JavaScript => &["**/*.js"],
            FileType::JavaScriptXml => &["**/*.jsx"],
            FileType::TypeScript => &["**/*.ts"],
            FileType::TypeScriptXml => &["**/*.tsx"],
            FileType::TypeScriptDeclaration => &["**/*.d.ts"],
            FileType::Text => &["**/*.txt"],
            FileType::Toml => &["**/*.toml"],
            FileType::Yaml => &["**/*.yaml", "**/*.yml"],
            FileType::Json => &["**/*.json"],
            FileType::Env => &["**/*.env"],
            FileType::Html => &["**/*.html", "**/*.htm"],
            FileType::Markdown => &["**/*.md"],
            FileType::Css => &["**/*.css"],
            FileType::Svg => &["**/*.svg"],
            FileType::Wasm => &["**/*.wasm"],
            FileType::Node => &["**/*.node"],
            FileType::SourceMap => &["**/*.map"],
            FileType::Object => &["**/*.o"],
            FileType::Image
            | FileType::Font
            | FileType::Audio
            | FileType::Video
            | FileType::Model
            | FileType::Neural
            | FileType::Document
            | FileType::Binary
            | FileType::Unknown => &[],
        }
    }
}
