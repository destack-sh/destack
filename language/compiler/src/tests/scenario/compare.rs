use std::collections::HashMap;
use std::sync::Arc;

use destack_artifact::{Ast, DehydrationContext, DirPrepared, DirResolved, Image};
use destack_core::{ImmutableStringPool, StringId, StringPool};
use destack_source::{FileId, ModuleId};
use serde::Serialize;

use crate::tests::TestWorkspaceView;

/// Persisted artifact test wrapper with one image-local string table.
#[derive(Debug, Clone, Serialize)]
struct EncodedDir<T> {
    /// The image-local string table.
    strings: ImmutableStringPool,
    /// The artifact payload using image-local string ids.
    payload: T,
}

/// String-id mapping state for deterministic DIR test encodings.
struct TestDehydrationContext<'a> {
    /// The source string pool.
    strings: &'a StringPool,
    /// The encoded string pool.
    encoded_strings: StringPool,
    /// Encoded string ids keyed by source string id.
    encoded_id_by_string_id: HashMap<StringId, StringId>,
}

impl<'a> TestDehydrationContext<'a> {
    /// Create one context for deterministic DIR test encodings.
    fn new(strings: &'a StringPool) -> Self {
        Self {
            strings,
            encoded_strings: StringPool::new(),
            encoded_id_by_string_id: HashMap::new(),
        }
    }

    /// Encode one DIR artifact with an image-local string table.
    fn encode<I>(mut self, payload: &I) -> Vec<u8>
    where
        I: Image<Live = I> + Serialize,
    {
        let payload = I::dehydrate(payload, &mut self);
        let encoded = EncodedDir {
            strings: self.encoded_strings.into_immutable(),
            payload,
        };

        postcard::to_allocvec(&encoded)
            .unwrap_or_else(|error| panic!("failed to encode dir artifact: {error}"))
    }
}

impl DehydrationContext for TestDehydrationContext<'_> {
    fn dehydrate_string_id(&mut self, string_id: StringId) -> StringId {
        *self
            .encoded_id_by_string_id
            .entry(string_id)
            .or_insert_with(|| {
                let string = self.strings.get(string_id);
                self.encoded_strings.intern(&string)
            })
    }
}

/// Build one stable profile key for scenario cache tests.
pub(crate) fn test_profile_key() -> destack_artifact::ProfileKey {
    use destack_artifact::{
        EmitFormat, EnvironmentStamp, Platform, ProfileFlags, ProfileKey, Runtime,
    };

    ProfileKey::new(
        EmitFormat::Js,
        Runtime::Node,
        Platform::Web,
        None,
        None,
        None,
        Vec::new(),
        false,
        false,
        false,
        EnvironmentStamp::Whitelist {
            keys: Vec::new(),
            hash: 0,
        },
        ProfileFlags::default(),
    )
}

/// Append text to one loaded file.
pub(crate) fn append_file_text(
    program: &TestWorkspaceView,
    file_id: destack_source::FileId,
    suffix: &str,
) {
    use destack_source::{File, FileContent};

    // load the current text contents
    let file = program.source_file(file_id);
    let FileContent::Text { content } = file.content.payload() else {
        panic!("expected text file");
    };

    // rebuild the file with the appended text
    let content = format!("{content}{suffix}");
    let file = File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        content,
    );

    program.replace_source_file(file);
}

/// Normalize one AST for stable cross-session comparison.
pub(crate) fn normalize_ast(mut ast: Ast) -> Ast {
    // normalize top level identity
    ast.id = ModuleId::EPHEMERAL;
    ast.tree.source_map.rebind_file(FileId::new(0));

    // normalize primary token spans
    for token in &mut ast.tokens {
        token.span = token.span.with_file(FileId::new(0));
    }

    // normalize side token spans
    for token in &mut ast.side_tokens {
        token.span = token.span.with_file(FileId::new(0));
    }

    ast
}

/// Encode one prepared DIR deterministically for equality assertions.
pub(crate) fn encode_dir_prepared(strings: &StringPool, dir: &DirPrepared) -> Vec<u8> {
    TestDehydrationContext::new(strings).encode(dir)
}

/// Encode one resolved DIR deterministically for equality assertions.
pub(crate) fn encode_dir_resolved(strings: &StringPool, dir: &DirResolved) -> Vec<u8> {
    TestDehydrationContext::new(strings).encode(dir)
}

/// Encode one AST deterministically for equality assertions.
pub(crate) fn encode_ast(ast: Ast) -> Vec<u8> {
    postcard::to_allocvec(&normalize_ast(ast))
        .unwrap_or_else(|error| panic!("failed to encode ast: {error}"))
}

/// Assert two AST values are equivalent.
pub(crate) fn assert_ast_eq(expected: Ast, actual: Ast) {
    // compare normalized encodings
    let expected = encode_ast(expected);
    let actual = encode_ast(actual);

    assert_eq!(actual, expected);
}

/// Assert two prepared DIR values are equivalent.
pub(crate) fn assert_dir_prepared_eq(
    expected_strings: &Arc<StringPool>,
    actual_strings: &Arc<StringPool>,
    expected: &DirPrepared,
    actual: &DirPrepared,
) {
    // compare deterministic artifact encodings
    let expected = encode_dir_prepared(expected_strings, expected);
    let actual = encode_dir_prepared(actual_strings, actual);

    assert_eq!(actual, expected);
}

/// Assert two resolved DIR values are equivalent.
pub(crate) fn assert_dir_resolved_eq(
    expected_strings: &Arc<StringPool>,
    actual_strings: &Arc<StringPool>,
    expected: &DirResolved,
    actual: &DirResolved,
) {
    // compare deterministic artifact encodings
    let expected = encode_dir_resolved(expected_strings, expected);
    let actual = encode_dir_resolved(actual_strings, actual);

    assert_eq!(actual, expected);
}
