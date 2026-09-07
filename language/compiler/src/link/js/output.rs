use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::{Component, Path, PathBuf};

use crate::link::{SourceMapBuilder, SourceMapMarker};
use crate::{Compiler, CompilerError, CompilerResult, JsLinker};
use base64::Engine;
use destack_artifact::{BundleFile, BundleSection};
use destack_js as js;
use destack_repository::{ProviderContext, SourceMapMode, Target};
use destack_source::{
    ByteRange, FileId, FileType, ModuleId, ProvenanceTable, SourceMap, TextMap, TextNameId, Uri,
};

/// One emitted text extent resolved to a standard source map location.
#[derive(Debug, Clone, Copy)]
struct MappedTextExtent {
    /// The emitted UTF-8 byte range.
    generated: ByteRange,
    /// The mapped source index.
    source_index: usize,
    /// The authored start line.
    start_line: u32,
    /// The authored start column.
    start_column: u32,
    /// The authored identifier name when this is one exact name extent.
    name: Option<TextNameId>,
}

impl MappedTextExtent {
    /// Build one marker at this extent's authored start.
    fn start_marker(self, emitted_byte: u32) -> SourceMapMarker {
        let name = if emitted_byte == self.generated.start {
            self.name.map(|name| name.index() as usize)
        } else {
            None
        };

        SourceMapMarker::Mapped {
            emitted_byte,
            source_index: self.source_index,
            original_line: self.start_line,
            original_column: self.start_column,
            name,
        }
    }
}

/// One emitted text extent resolved for source map output.
#[derive(Debug, Clone, Copy)]
enum ResolvedTextExtent {
    /// Text attributed to one authored source position.
    Mapped(MappedTextExtent),
    /// Text without one direct authored source position.
    Unmapped(ByteRange),
}

impl ResolvedTextExtent {
    /// Return the emitted byte range.
    const fn generated(self) -> ByteRange {
        match self {
            Self::Mapped(extent) => extent.generated,
            Self::Unmapped(generated) => generated,
        }
    }

    /// Return the source map priority among equal ranges.
    const fn mapping_priority(self) -> u8 {
        match self {
            Self::Unmapped(_) => 0,
            Self::Mapped(MappedTextExtent { name: None, .. }) => 1,
            Self::Mapped(MappedTextExtent { name: Some(_), .. }) => 2,
        }
    }

    /// Build one marker at this extent's start.
    fn start_marker(self, emitted_byte: u32) -> SourceMapMarker {
        match self {
            Self::Mapped(extent) => extent.start_marker(emitted_byte),
            Self::Unmapped(_) => SourceMapMarker::Unmapped { emitted_byte },
        }
    }
}

/// One final JS text output derived from one target.
#[derive(Debug, Clone, Copy)]
struct TextOutput<'a> {
    /// The target that drives final output shaping.
    target: &'a Target,
}

impl<'a> TextOutput<'a> {
    /// Create one JS text output.
    fn new(target: &'a Target) -> Self {
        Self { target }
    }

    /// Append one source map reference when the target wants one.
    fn annotate(
        self,
        code: String,
        map: Option<&SourceMap>,
        map_reference: Option<String>,
    ) -> CompilerResult<String> {
        // inline source maps stay in the text payload only
        if self.target.uses_inline_source_maps() {
            let map = map.ok_or_else(|| CompilerError::Internal {
                message: "inline source map output has no source map".to_string(),
            })?;
            let inline_map = self.inline_map_url(map)?;

            return Ok(self.append_map_reference(code, &inline_map));
        }

        // hidden maps emit sidecar outputs but do not annotate the text payload
        if !self.target.emits_source_map_output()
            || matches!(self.target.source_map, Some(SourceMapMode::Hidden))
        {
            return Ok(code);
        }

        let map_reference = map_reference.ok_or_else(|| CompilerError::Internal {
            message: "visible source map output has no map reference".to_string(),
        })?;

        Ok(self.append_map_reference(code, &map_reference))
    }

    /// Apply configured banner and footer text to one final JS payload.
    fn apply_banner_and_footer(self, mut code: String) -> String {
        let banner_prefix = self.banner_prefix();

        // prepend banner text before the emitted module body
        if !banner_prefix.is_empty() {
            let mut with_banner = banner_prefix;

            with_banner.push_str(&code);
            code = with_banner;
        }

        // append footer text before any source map reference
        if let Some(footer) = self.target.js.output.footer.as_deref() {
            if !code.is_empty() && !code.ends_with('\n') {
                code.push('\n');
            }

            code.push_str(footer);

            if !code.is_empty() && !code.ends_with('\n') {
                code.push('\n');
            }
        }

        code
    }

    /// Return the exact banner prefix inserted before mapped JS code.
    fn banner_prefix(self) -> String {
        let Some(banner) = self.target.js.output.banner.as_deref() else {
            return String::new();
        };
        let mut prefix = banner.to_string();

        if !prefix.is_empty() && !prefix.ends_with('\n') {
            prefix.push('\n');
        }

        prefix
    }

    /// Return the mapped byte offset introduced before emitted JS code.
    fn banner_prefix_byte_count(self) -> u32 {
        self.banner_prefix().len() as u32
    }

    /// Return whether the final text includes one source map annotation.
    fn has_map_annotation(self) -> bool {
        self.target.uses_inline_source_maps()
            || self.target.emits_source_map_output()
                && !matches!(self.target.source_map, Some(SourceMapMode::Hidden))
    }

    /// Build one inline source map data URL.
    fn inline_map_url(self, map: &SourceMap) -> CompilerResult<String> {
        let map = serde_json::to_string(map).map_err(|error| CompilerError::Internal {
            message: format!("failed to serialize inline source map: {error}"),
        })?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(map.as_bytes());

        Ok(format!(
            "data:application/json;charset=utf-8;base64,{encoded}"
        ))
    }

    /// Append one source map reference comment to one text payload.
    fn append_map_reference(self, mut code: String, reference: &str) -> String {
        if !code.is_empty() && !code.ends_with('\n') {
            code.push('\n');
        }

        code.push_str(&format!("//# sourceMappingURL={reference}\n"));
        code
    }
}

impl JsLinker<'_> {
    /// Print one JS module with one exact source map marker stream.
    pub(crate) fn print_js_module(
        &self,
        module_id: ModuleId,
        module: &js::Module,
        context: &dyn ProviderContext,
    ) -> CompilerResult<js::PrintedModule> {
        // source file
        let source_module = self.compiler.module(context.revision(), module_id)?;
        let source_file = self.compiler.file(context, source_module.file_id)?;

        // format readable output
        let module = module.clone();
        let printed = module
            .format(source_file.as_ref(), js::FormatOptions::default())
            .map_err(|error| error.to_string());

        printed.map_err(|error| CompilerError::Internal {
            message: format!("failed to print JavaScript module: {error}"),
        })
    }

    /// Build one standard source map from emitted text provenance.
    pub(crate) fn script_source_map(
        &self,
        target: &Target,
        emitted_source_map_path: &Path,
        provenance: &ProvenanceTable,
        text_map: &TextMap,
        context: &dyn ProviderContext,
    ) -> CompilerResult<SourceMapBuilder> {
        let mut source_index_by_file = HashMap::<FileId, usize>::new();
        let mut sources = Vec::new();
        let mut sources_content = (!target.js.output.source_map_exclude_sources).then(Vec::new);
        let mut files = HashMap::new();
        let mut extents = Vec::new();

        // map every emitted extent offset to its authored file position
        for extent in &text_map.extents {
            if extent.generated.start >= extent.generated.end {
                continue;
            }
            provenance
                .attribution(extent.provenance)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "text extent {} is outside the provenance table",
                        extent.provenance.index()
                    ),
                })?;
            let span = match extent.name {
                Some(_) => Some(provenance.primary_span(extent.provenance).ok_or_else(|| {
                    CompilerError::Internal {
                        message: format!(
                            "named text extent {} has no primary authored span",
                            extent.provenance.index()
                        ),
                    }
                })?),
                None => provenance.span(extent.provenance),
            };
            let Some(span) = span else {
                extents.push(ResolvedTextExtent::Unmapped(extent.generated));
                continue;
            };
            let source_file = match files.entry(span.file) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let file = self.compiler.file(context, span.file)?;

                    entry.insert(file)
                }
            };
            let source_index = match source_index_by_file.get(&span.file) {
                Some(index) => *index,
                None => {
                    let path = match &source_file.path {
                        Some(path) => {
                            if let Some(path) = relative_path_between(emitted_source_map_path, path)
                            {
                                let path = path.to_string_lossy().replace('\\', "/");

                                encode_source_map_path(&path)
                            } else {
                                let uri = Uri::from_path(path);

                                encode_source_map_uri(uri.as_ref())
                            }
                        }
                        None => encode_source_map_uri(source_file.uri.as_ref()),
                    };
                    let index = sources.len();
                    sources.push(path);
                    if let Some(sources_content) = &mut sources_content {
                        sources_content.push(Some(source_file.text().to_string()));
                    }
                    source_index_by_file.insert(span.file, index);

                    index
                }
            };
            let (start_line, start_column) = source_file
                .get_utf16_position(span.start)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "authored span start {} is outside file {}",
                        span.start, span.file
                    ),
                })?;
            extents.push(ResolvedTextExtent::Mapped(MappedTextExtent {
                generated: extent.generated,
                source_index,
                start_line,
                start_column,
                name: extent.name,
            }));
        }

        // order starts by range width and source map priority
        extents.sort_by_key(|extent| {
            let generated = extent.generated();

            (
                generated.start,
                Reverse(generated.end),
                extent.mapping_priority(),
            )
        });
        let mut offsets = extents
            .iter()
            .flat_map(|extent| {
                let generated = extent.generated();

                [generated.start, generated.end]
            })
            .collect::<Vec<_>>();
        offsets.sort_unstable();
        offsets.dedup();

        // select the innermost extent after every start and end offset
        let mut active = Vec::<usize>::new();
        let mut markers = Vec::new();
        let mut start_index = 0;
        for emitted_byte in offsets {
            // close extents before opening peers at the same emitted byte
            while let Some(index) = active.last().copied() {
                let end = extents[index].generated().end;
                if end == emitted_byte {
                    active.pop();
                } else if end < emitted_byte {
                    return Err(CompilerError::Internal {
                        message: format!("text extent ended before offset {emitted_byte}"),
                    });
                } else {
                    break;
                }
            }

            // open extents from outermost to innermost
            while start_index < extents.len()
                && extents[start_index].generated().start == emitted_byte
            {
                if let Some(parent) = active.last().copied()
                    && extents[start_index].generated().end > extents[parent].generated().end
                {
                    return Err(CompilerError::Internal {
                        message: format!("text provenance extents cross at byte {emitted_byte}"),
                    });
                }
                active.push(start_index);
                start_index += 1;
            }

            // map following text to the narrowest active extent
            if let Some(index) = active.last() {
                markers.push(extents[*index].start_marker(emitted_byte));
            }
            // end attribution when the complete mapped region closes
            else {
                markers.push(SourceMapMarker::Unmapped { emitted_byte });
            }
        }

        Ok(SourceMapBuilder::new(
            sources,
            sources_content,
            text_map.names.clone(),
            markers,
        ))
    }

    /// Link one final JS text output and any related sidecars.
    pub(crate) fn link_script_text_files(
        &self,
        target: &Target,
        output_path: &Path,
        code: String,
        map: Option<SourceMapBuilder>,
        map_path: Option<&Path>,
        mut text_map: TextMap,
    ) -> CompilerResult<Vec<BundleFile>> {
        if target.emits_source_maps() != map.is_some() {
            return Err(CompilerError::Internal {
                message: "source map configuration differs from the emitted map".to_string(),
            });
        }
        if target.emits_source_map_output() != map_path.is_some() {
            return Err(CompilerError::Internal {
                message: "source map configuration differs from the map output path".to_string(),
            });
        }

        let output = TextOutput::new(target);
        let map_reference = map_path.map(|path| relative_map_reference(output_path, path));
        let mut shaped_code = output.apply_banner_and_footer(code);
        let mut map = map;
        let has_map_annotation = output.has_map_annotation();
        let banner_byte_count = output.banner_prefix_byte_count();

        // include the exact separator inserted before one annotation
        if has_map_annotation && !shaped_code.is_empty() && !shaped_code.ends_with('\n') {
            shaped_code.push('\n');
        }

        // banner bytes shift every emitted marker forward in the final output
        if let Some(map) = &mut map {
            map.shift(banner_byte_count);
        }
        text_map
            .shift(banner_byte_count)
            .map_err(|error| CompilerError::Internal {
                message: error.to_string(),
            })?;
        let map = map
            .map(|map| map.build(&shaped_code, has_map_annotation as usize))
            .transpose()?;

        let code = output.annotate(shaped_code, map.as_ref(), map_reference)?;
        let bytes = Compiler::encode_output_text(code);
        let section = if self.target.emits_assembled_output() {
            BundleSection::Entry
        } else {
            BundleSection::Module
        };
        let mut files = vec![self.compiler.put_output_file(
            section,
            Uri::from_path(output_path),
            FileType::JavaScript,
            &bytes,
            None,
            text_map,
        )?];

        let Some(map) = map.as_ref() else {
            return Ok(files);
        };

        let Some(map_path) = map_path else {
            return Ok(files);
        };

        let bytes = Compiler::encode_source_map(map).map_err(|error| CompilerError::Internal {
            message: format!("failed to serialize source map: {error}"),
        })?;

        files.push(self.compiler.put_output_file(
            BundleSection::SourceMap,
            Uri::from_path(map_path),
            FileType::SourceMap,
            &bytes,
            None,
            TextMap::default(),
        )?);

        Ok(files)
    }
}

/// Build one relative source map reference from one output path.
fn relative_map_reference(output_path: &Path, map_path: &Path) -> String {
    let Some(relative) = relative_path_between(output_path, map_path) else {
        let uri = Uri::from_path(map_path);

        return encode_source_map_uri(uri.as_ref());
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    let relative = encode_source_map_path(&relative);

    if relative.starts_with('.') {
        return relative;
    }

    format!("./{relative}")
}

/// Percent encode one filesystem path used as a source map URL reference.
fn encode_source_map_path(path: &str) -> String {
    percent_encode(path, |byte| is_url_unreserved(byte) || byte == b'/')
}

/// Percent encode one complete source URI while retaining URL delimiters.
fn encode_source_map_uri(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut encoded = String::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        // retain complete percent encoded bytes
        if byte == b'%'
            && bytes
                .get(index + 1..index + 3)
                .is_some_and(|digits| digits.iter().all(|digit| digit.is_ascii_hexdigit()))
        {
            encoded.push('%');
            encoded.push(bytes[index + 1] as char);
            encoded.push(bytes[index + 2] as char);
            index += 3;
            continue;
        }

        // retain URL delimiters
        let is_allowed = is_url_unreserved(byte)
            || matches!(
                byte,
                b':' | b'/'
                    | b'?'
                    | b'#'
                    | b'['
                    | b']'
                    | b'@'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
            );
        push_url_byte(&mut encoded, byte, is_allowed);
        index += 1;
    }

    encoded
}

/// Return whether one byte is unreserved in a URL.
const fn is_url_unreserved(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
    )
}

/// Percent encode bytes rejected by one URL component.
fn percent_encode(text: &str, is_allowed: impl Fn(u8) -> bool) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        push_url_byte(&mut encoded, byte, is_allowed(byte));
    }

    encoded
}

/// Append one allowed or percent encoded URL byte.
fn push_url_byte(encoded: &mut String, byte: u8, is_allowed: bool) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    if is_allowed {
        encoded.push(byte as char);

        return;
    }

    encoded.push('%');
    encoded.push(char::from(HEX[(byte >> 4) as usize]));
    encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
}

/// Return one relative path from one emitted output file to another.
fn relative_path_between(from_output_path: &Path, to_output_path: &Path) -> Option<PathBuf> {
    let from_directory = from_output_path.parent().unwrap_or_else(|| Path::new(""));
    let from_components = from_directory.components().collect::<Vec<_>>();
    let to_components = to_output_path.components().collect::<Vec<_>>();
    let mut shared = 0;

    while shared < from_components.len()
        && shared < to_components.len()
        && from_components[shared] == to_components[shared]
    {
        shared += 1;
    }

    // require both paths to share the same filesystem root
    if shared == 0 && (from_output_path.is_absolute() || to_output_path.is_absolute()) {
        return None;
    }
    if from_components[shared..]
        .iter()
        .chain(&to_components[shared..])
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }

    let mut relative_path = PathBuf::new();

    for component in &from_components[shared..] {
        if matches!(component, Component::Normal(_)) {
            relative_path.push("..");
        }
    }

    for component in &to_components[shared..] {
        if let Component::Normal(segment) = component {
            relative_path.push(segment);
        }
    }

    Some(relative_path)
}
