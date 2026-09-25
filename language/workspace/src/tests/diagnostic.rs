use tspp_source::Diagnostic;

use crate::tests::harness::TestWorkspace;

/// Read diagnostics from each exact source revision.
#[test]
fn test_diagnose_successive_source_revisions() {
    let test = TestWorkspace::with_entry("diagnose-source-revisions", "main.tspp");
    let source = r#"export function run(): void {
  const value: float64 = false;
}
"#;
    let mut file = test.file("main.tspp", source);
    let mut branch = test.create_branch("editor");
    let first_revision = branch.revision();

    // read the complete diagnostic result for the first revision
    let first = branch.diagnose(&file);
    let expected = Diagnostic::error(
        "not-assignable",
        "type 'false' is not assignable to type 'float64'",
        file.label("false"),
    )
    .label(file.message("float64", "expected due to this annotation"));

    assert_eq!(first.revision, first_revision);
    assert_eq!(first.diagnostics, [expected]);

    // replace the source and require diagnostics from only the new revision
    branch.edit([file.replace("false", "1")]);
    let second_revision = branch.revision();
    let second = branch.diagnose(&file);

    assert_ne!(first_revision, second_revision);
    assert_eq!(second.revision, second_revision);
    assert_eq!(second.diagnostics, Vec::new());
}

/// Move primary and declaration labels with editor revisions.
#[test]
fn test_diagnose_moves_cross_file_labels() {
    let test = TestWorkspace::with_entry("diagnose-cross-file-labels", "src/main.tspp");
    let mut library = test.file(
        "src/library.tspp",
        "export const helper: int32 = 1;\nexport const sibling: int32 = 2;\n",
    );
    let mut main = test.file(
        "src/main.tspp",
        r#"import { helper } from "./library.tspp";

const first = helper;
const second = sibling;
"#,
    );
    let mut branch = test.create_branch("editor");

    // build diagnostics for the initial source revision
    let diagnostics = branch.diagnose(&main);
    let mut expected = Diagnostic::error(
        "unresolved-reference",
        "cannot find 'sibling'",
        main.label("sibling"),
    )
    .label(library.message("sibling", "'sibling' is declared here"))
    .help("import 'sibling' from its module");

    assert_eq!(diagnostics.diagnostics, [expected.clone()]);

    // move the primary source and its exact label
    branch.edit([main.replace("const second", "// shifted\nconst second")]);
    let diagnostics = branch.diagnose(&main);
    expected.primary = main.label("sibling");

    assert_eq!(diagnostics.diagnostics, [expected.clone()]);

    // move the declaration label with its foreign source
    branch.edit([library.replace("export const sibling", "// shifted\nexport const sibling")]);
    let diagnostics = branch.diagnose(&main);
    expected.labels[0] = library.message("sibling", "'sibling' is declared here");

    assert_eq!(diagnostics.diagnostics, [expected.clone()]);

    // remove the declaration label when the foreign name changes
    branch.edit([library.replace("sibling", "other")]);
    let diagnostics = branch.diagnose(&main);
    expected.labels.clear();
    expected.helps.clear();

    assert_eq!(diagnostics.diagnostics, [expected]);
}
