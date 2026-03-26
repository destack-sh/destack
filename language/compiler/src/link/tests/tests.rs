use std::path::{Path, PathBuf};

use destack_artifact::{
    ArtifactKey, BuildManifest, OutputContent, OutputFile, PackageAssembly, PackageOutput,
    SourceMapArtifact, TargetOutputName,
};
use destack_source::{
    DiagnosticSeverity, DiffOptions, FileType, ModuleId, PackageId, Uri, print_diff,
};
use destack_workspace::{SourceMapMode, Target, TargetDiscovery, TargetId};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub(super) use crate::tests::{ExpectedDiagnostic, TestProgram};

/// One normalized text output file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedTextFile {
    /// The package-relative output path.
    pub path: String,
    /// The emitted file type.
    pub file_type: FileType,
    /// The emitted text.
    pub text: String,
}

/// One normalized JSON output file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedJsonFile<T> {
    /// The package-relative output path.
    pub path: String,
    /// The emitted file type.
    pub file_type: FileType,
    /// The serialized JSON text.
    pub text: String,
    /// The parsed JSON value.
    pub value: T,
}

/// One normalized linked script target assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedScriptTarget {
    /// The target assembly mode.
    pub assembly: PackageAssembly,
    /// The exact output groups and package-relative paths.
    pub output_groups: IndexMap<TargetOutputName, Vec<String>>,
    /// The primary entry output.
    pub entry: LinkedTextFile,
    /// The emitted manifest output.
    pub manifest: LinkedJsonFile<BuildManifest>,
    /// The emitted source map output.
    pub map: LinkedJsonFile<SourceMapArtifact>,
}

impl TestProgram {
    /// Configure one single-file JavaScript target for linker tests.
    pub(super) fn configure_single_file_js_target(&self, module_id: ModuleId, name: &str) {
        self.configure_target(module_id, name, |target| {
            // entry based linking
            target.discovery = TargetDiscovery::Entry;
            target.entry = vec![PathBuf::from("main.ts")];

            // single-file bundle surface
            target.out_file = Some(PathBuf::from(format!("dist/{name}.js")));

            // manifest and external maps
            target.bundle.output.manifest = true;
            target.source_map_mode = Some(SourceMapMode::External);
        });
    }

    /// Return the package-relative path for one module.
    pub(super) fn module_relative_path(&self, module_id: ModuleId) -> String {
        let module = self.program.modules.get(module_id);

        self.normalize_uri_path(module.package_id, &module.uri)
    }

    /// Build one single-file JavaScript target and return the normalized linked assembly.
    pub(super) fn link_single_file_js_target(
        &self,
        module_id: ModuleId,
        name: &str,
    ) -> LinkedScriptTarget {
        self.link_single_file_js_target_with(module_id, name, |_| {})
    }

    /// Build one single-file JavaScript target with extra configuration and return the linked assembly.
    pub(super) fn link_single_file_js_target_with<F>(
        &self,
        module_id: ModuleId,
        name: &str,
        configure: F,
    ) -> LinkedScriptTarget
    where
        F: FnOnce(&mut Target),
    {
        let package_id = self.program.modules.get(module_id).package_id;
        let target_id = TargetId::new(package_id, name);

        self.configure_single_file_js_target(module_id, name);
        self.configure_target(module_id, name, configure);
        self.run(ArtifactKey::package_output(package_id, target_id));

        // clean builds are easier to reason about in linker tests
        self.check_no_diagnostic(DiagnosticSeverity::Error);

        let output = self.package_output(package_id, name);
        self.linked_script_target(package_id, &output)
    }

    /// Assert one linked script target exactly.
    pub(super) fn assert_linked_script_target(
        &self,
        actual: &LinkedScriptTarget,
        expected: &LinkedScriptTarget,
    ) {
        self.assert_linked_script_assembly(actual, expected.assembly);
        self.assert_linked_script_output_groups(actual, &expected.output_groups);
        self.assert_linked_script_entry(actual, &expected.entry);
        self.assert_linked_script_manifest(actual, &expected.manifest);
        self.assert_linked_script_map(actual, &expected.map);
    }

    /// Assert the assembly mode for one linked script target.
    pub(super) fn assert_linked_script_assembly(
        &self,
        actual: &LinkedScriptTarget,
        expected: PackageAssembly,
    ) {
        assert_eq!(actual.assembly, expected, "linked script assembly mismatch");
    }

    /// Assert the output groups for one linked script target.
    pub(super) fn assert_linked_script_output_groups(
        &self,
        actual: &LinkedScriptTarget,
        expected: &IndexMap<TargetOutputName, Vec<String>>,
    ) {
        self.assert_output_groups(&actual.output_groups, expected, "linked output groups");
    }

    /// Assert the primary entry output for one linked script target.
    pub(super) fn assert_linked_script_entry(
        &self,
        actual: &LinkedScriptTarget,
        expected: &LinkedTextFile,
    ) {
        self.assert_linked_text_file(&actual.entry, expected, "linked entry output");
    }

    /// Assert the manifest output for one linked script target.
    pub(super) fn assert_linked_script_manifest(
        &self,
        actual: &LinkedScriptTarget,
        expected: &LinkedJsonFile<BuildManifest>,
    ) {
        self.assert_linked_json_file(&actual.manifest, expected, "linked manifest output");
    }

    /// Assert the source map output for one linked script target.
    pub(super) fn assert_linked_script_map(
        &self,
        actual: &LinkedScriptTarget,
        expected: &LinkedJsonFile<SourceMapArtifact>,
    ) {
        self.assert_linked_json_file(&actual.map, expected, "linked source map output");
    }

    /// Normalize one linked script target into exact assertion data.
    fn linked_script_target(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
    ) -> LinkedScriptTarget {
        let output_groups = output
            .outputs
            .iter()
            .map(|(name, files)| {
                let files = files
                    .iter()
                    .map(|file| self.normalize_uri_path(package_id, &file.uri))
                    .collect::<Vec<_>>();
                (*name, files)
            })
            .collect::<IndexMap<_, _>>();

        LinkedScriptTarget {
            assembly: output.assembly,
            output_groups,
            entry: self.single_text_output(package_id, output, TargetOutputName::Entry),
            manifest: self.single_json_output(package_id, output, TargetOutputName::Manifest),
            map: self.single_json_output(package_id, output, TargetOutputName::Maps),
        }
    }

    /// Return one normalized text output file from the given group.
    fn single_text_output(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
    ) -> LinkedTextFile {
        let file = self.single_output_file(output, output_name);

        match &file.content {
            OutputContent::Text { code, file_type } => LinkedTextFile {
                path: self.normalize_uri_path(package_id, &file.uri),
                file_type: *file_type,
                text: code.clone(),
            },
            _ => panic!("expected text output in '{}'", output_name.as_str()),
        }
    }

    /// Return one normalized JSON output file from the given group.
    fn single_json_output<T>(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
    ) -> LinkedJsonFile<T>
    where
        T: DeserializeOwned,
    {
        let file = self.single_output_file(output, output_name);

        match &file.content {
            OutputContent::Json {
                content,
                value: _,
                file_type,
            } => LinkedJsonFile {
                path: self.normalize_uri_path(package_id, &file.uri),
                file_type: *file_type,
                text: content.clone(),
                value: serde_json::from_str(content).unwrap_or_else(|error| {
                    panic!(
                        "failed to decode '{}' json output: {error}",
                        output_name.as_str()
                    )
                }),
            },
            _ => panic!("expected json output in '{}'", output_name.as_str()),
        }
    }

    /// Return one exact output file from the given group.
    fn single_output_file<'a>(
        &self,
        output: &'a PackageOutput,
        output_name: TargetOutputName,
    ) -> &'a OutputFile {
        let files = output
            .outputs
            .get(&output_name)
            .unwrap_or_else(|| panic!("missing output group '{}'", output_name.as_str()));

        match files.as_slice() {
            [file] => file,
            _ => panic!(
                "expected exactly one output file in '{}', found {}",
                output_name.as_str(),
                files.len()
            ),
        }
    }

    /// Normalize one URI to a package-relative path when possible.
    fn normalize_uri_path(&self, package_id: PackageId, uri: &Uri) -> String {
        let path = uri
            .to_path()
            .unwrap_or_else(|| panic!("uri '{}' is not a path", uri));

        self.normalize_package_path(package_id, path)
    }

    /// Normalize one path to a package-relative path when possible.
    fn normalize_package_path(&self, package_id: PackageId, path: &Path) -> String {
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let package_dir = package
            .path
            .clone()
            .unwrap_or_else(|| self.program.cwd.clone());
        let relative = path.strip_prefix(&package_dir).unwrap_or(path);

        relative.to_string_lossy().replace('\\', "/")
    }

    /// Assert one exact linked text file.
    fn assert_linked_text_file(
        &self,
        actual: &LinkedTextFile,
        expected: &LinkedTextFile,
        noun: &str,
    ) {
        assert_eq!(actual.path, expected.path, "{noun} path mismatch");
        assert_eq!(
            actual.file_type, expected.file_type,
            "{noun} file type mismatch"
        );

        if actual.text != expected.text {
            print_diff(&expected.text, &actual.text, &DiffOptions::new());
            panic!("{noun} text mismatch");
        }
    }

    /// Assert one exact linked JSON file.
    fn assert_linked_json_file<T>(
        &self,
        actual: &LinkedJsonFile<T>,
        expected: &LinkedJsonFile<T>,
        noun: &str,
    ) where
        T: PartialEq + Serialize + std::fmt::Debug,
    {
        assert_eq!(actual.path, expected.path, "{noun} path mismatch");
        assert_eq!(
            actual.file_type, expected.file_type,
            "{noun} file type mismatch"
        );

        if actual.text != expected.text {
            print_diff(&expected.text, &actual.text, &DiffOptions::new());
            panic!("{noun} text mismatch");
        }

        assert_eq!(actual.value, expected.value, "{noun} value mismatch");
    }

    /// Assert exact linked output groups.
    fn assert_output_groups(
        &self,
        actual: &IndexMap<TargetOutputName, Vec<String>>,
        expected: &IndexMap<TargetOutputName, Vec<String>>,
        noun: &str,
    ) {
        let actual = self.format_output_groups(actual);
        let expected = self.format_output_groups(expected);

        if actual != expected {
            print_diff(&expected, &actual, &DiffOptions::new());
            panic!("{noun} mismatch");
        }
    }
    /// Format linked output groups for stable diff output.
    fn format_output_groups(&self, groups: &IndexMap<TargetOutputName, Vec<String>>) -> String {
        groups
            .iter()
            .map(|(name, files)| format!("{}: {}", name.as_str(), files.join(", ")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
