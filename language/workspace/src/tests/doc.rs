use futures::executor::block_on;

use tspp_doc::{DeclarationKind, PACKAGE_REFERENCE_SCHEMA_VERSION, TextRange};

use crate::command::{CommandOptions, CommandRevision, DocInput};
use crate::tests::harness::TestWorkspace;

/// Build checked package documentation from the configured public exports.
#[test]
fn test_doc_command_builds_checked_package_reference() {
    let test = TestWorkspace::new("doc-command-reference");
    let configuration = r#"{
  "name": "relay",
  "version": "1.2.3",
  "description": "A documentation fixture.",
  "license": "MIT",
  "targets": {
    "default": {
      "entry": ["src/index.tspp"]
    }
  },
  "defaultTarget": "default",
  "exports": {
    ".": {
      "path": "./src/index.tspp"
    },
    "./direct": { "path": "./src/direct.tspp" },
    "./empty": { "path": "./src/empty.tspp" },
    "./nested/binding": { "path": "./src/nested/binding/index.tspp" }
  }
}
"#;
    let configuration_path = test.write_text("destack.json", configuration);
    test.apply_text(&configuration_path, configuration);
    let source = r#"/** Add one to a value. */
export function increment(value: int): int {
    return value + 1;
}

/** The stable service name. */
export const service = "relay";

/** One delivery state. */
export enum DeliveryState {
    /** The delivery is waiting. */
    Pending,

    /** The delivery completed. */
    Delivered = 7,
}

/** Reborrow every field in one region. */
export type BorrowedFields<T, const R: Region> = {
    [K in keyof T]: Borrowed<T[K], R>;
};

/** One fixed-width integer representation. */
export type Representation = `int${1..=128}` | `uint${1..=128}`;

/** Asynchronous byte writer. */
export interface AsyncWriter {
    /** Write one buffer. */
    write(buffer: &readonly [uint8]): usize;

    /** Flush buffered writes. */
    flush(): void;

    /** Call with one value. */
    <T>(value: T): T;

    /** Construct from one value. */
    new <T>(value: T): AsyncWriter;
}
"#;
    let source_path = test.write_text("src/index.tspp", source);
    test.apply_text(&source_path, source);

    for (path, source) in [
        ("src/empty.tspp", ""),
        ("src/nested/binding/index.tspp", "export const value = 1;\n"),
    ] {
        let file = test.write_text(path, source);
        test.apply_text(&file, source);
    }

    // document namespace-only modules without inventing public import paths
    for (path, source) in [
        (
            "src/direct.tspp",
            "export * as tools from \"./tools.tspp\";",
        ),
        (
            "src/tools.tspp",
            "export * as nested from \"./nested-tool.tspp\"; export * as parent from \"./direct.tspp\"; export const answer = 42;",
        ),
        ("src/nested-tool.tspp", "export const value = 1;"),
    ] {
        let file = test.write_text(path, source);
        test.apply_text(&file, source);
    }

    // generate the package artifact through the normal workspace command
    let input = DocInput::from((
        CommandRevision::Current,
        CommandOptions {
            config_inputs: true,
            ..CommandOptions::default()
        },
    ));
    let output = block_on(test.workspace.doc(input, None)).expect("doc command failed");
    let reference = output.data.reference.expect("missing package reference");

    // retain exact package and module identity
    assert_eq!(reference.schema_version, PACKAGE_REFERENCE_SCHEMA_VERSION);
    assert_eq!(reference.toolchain_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(reference.package.name, "relay");
    assert_eq!(reference.package.version.as_deref(), Some("1.2.3"));
    assert_eq!(reference.modules.len(), 4);
    assert!(reference.modules[2].exports.is_empty());
    assert_eq!(reference.modules[0].specifier, "relay");
    assert_eq!(reference.modules[0].path.as_deref(), Some("src/index.tspp"));

    assert_eq!(reference.namespaces.len(), 2);
    let tools = reference
        .namespaces
        .iter()
        .find(|namespace| namespace.path.as_deref() == Some("src/tools.tspp"))
        .expect("exported namespace");
    assert_eq!(
        tools
            .exports
            .iter()
            .map(|export| export.name.as_str())
            .collect::<Vec<_>>(),
        ["answer", "nested", "parent"]
    );
    assert_eq!(
        reference.modules[1].exports[0].namespace.as_deref(),
        Some(tools.module.as_str())
    );

    // reuse public module records when a namespace points back to its parent
    assert_eq!(
        tools.exports[2].namespace.as_deref(),
        Some(reference.modules[1].module.as_str())
    );

    // retain canonical checked signatures, documentation, and source positions
    let exports = &reference.modules[0].exports;
    assert_eq!(exports.len(), 6);
    assert_eq!(
        exports
            .iter()
            .map(|export| export.name.as_str())
            .collect::<Vec<_>>(),
        [
            "AsyncWriter",
            "BorrowedFields",
            "DeliveryState",
            "Representation",
            "increment",
            "service"
        ]
    );
    assert_eq!(
        exports
            .iter()
            .map(|export| export.declarations[0].signature.text.as_str())
            .collect::<Vec<_>>(),
        [
            "export interface AsyncWriter",
            "export type BorrowedFields<T, const R: Region> = {\n    [K in keyof T]: Borrowed<T[K], R>;\n}",
            "export enum DeliveryState",
            "export type Representation = `int${1..=128}` | `uint${1..=128}`",
            "export function increment(value: int64): int64",
            "const service: \"relay\"",
        ]
    );
    assert_eq!(
        exports[4].declarations[0].documentation.as_deref(),
        Some("Add one to a value.")
    );
    assert_eq!(
        exports[4].declarations[0].source.path.as_deref(),
        Some("src/index.tspp")
    );
    assert_eq!(exports[4].declarations[0].source.line, 2);
    assert_eq!(exports[4].declarations[0].source.end_line, 4);
    assert_eq!(
        exports[5].declarations[0].documentation.as_deref(),
        Some("The stable service name.")
    );
    assert_eq!(exports[5].declarations[0].kind, DeclarationKind::Constant);

    // retain public members with checked signatures and authored documentation
    let members = &exports[0].declarations[0].members;
    assert_eq!(members.len(), 4);
    assert_eq!(members[0].name.as_deref(), Some("write"));
    assert_eq!(members[0].kind, DeclarationKind::Method);
    assert_eq!(
        members[0].signature.text,
        "write(buffer: &'a readonly [uint8]): usize"
    );
    assert_eq!(
        members[0].documentation.as_deref(),
        Some("Write one buffer.")
    );
    assert_eq!(members[1].signature.text, "flush(): void");

    // preserve anonymous callable syntax without inventing a declared name
    assert_eq!(members[2].signature.text, "<T>(value: T): T");
    assert_eq!(members[2].signature.name, None);
    assert_eq!(
        members[2].documentation.as_deref(),
        Some("Call with one value.")
    );
    assert_eq!(members[3].signature.text, "new <T>(value: T): AsyncWriter");
    assert_eq!(members[3].signature.name, None);
    assert_eq!(
        members[3].documentation.as_deref(),
        Some("Construct from one value.")
    );

    // represent enum variants as authored variants rather than singleton properties
    let variants = &exports[2].declarations[0].members;
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].kind, DeclarationKind::EnumMember);
    assert_eq!(variants[0].signature.text, "Pending");
    assert_eq!(variants[1].signature.text, "Delivered = 7");
    assert_eq!(
        variants[1].documentation.as_deref(),
        Some("The delivery completed.")
    );

    // retain the declared name interval inside each checked signature
    let signature = &exports[0].declarations[0].signature;
    assert_eq!(signature.name, Some(TextRange { start: 17, end: 28 }));
    assert_eq!(
        members[0].signature.name,
        Some(TextRange { start: 0, end: 5 })
    );
}
