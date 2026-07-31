use std::collections::BTreeMap;

use destack_artifact::ArtifactKey;
use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::ArtifactReader;
use destack_source::TargetId;

use crate::tests::TestSession;

/// Check every builtin library module.
#[test]
fn test_check_library() {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .unwrap_or_else(|error| panic!("builtin library target profile should resolve:\n{error}"))
        .id();

    // check every builtin module through the normal artifact path
    let keys = package
        .module_ids()
        .map(|module| ArtifactKey::dir_checked(module, profile))
        .collect::<Vec<_>>();
    let result = session.require_all(keys.iter().copied());
    let diagnostics = session.render_terminal_diagnostics_for(&keys);
    if !diagnostics.is_empty() {
        panic!("\n{diagnostics}");
    }

    if let Err(error) = result {
        panic!("{error}");
    }
}

/// The core knot stays bounded: shrinking it is welcome, growing it is a defect.
#[test]
fn test_bound_library_check_components() {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    // partition the library modules by check component
    let modules: Vec<_> = package.module_ids().collect();
    let graph = session.component_graph(profile);
    let mut sizes = BTreeMap::new();
    for module in &modules {
        let Some(component) = graph.reference_component(*module) else {
            panic!("builtin module {module:?} is absent from the check component graph");
        };
        *sizes.entry(component).or_insert(0usize) += 1;
    }

    // ratchet the largest mutually referential kernel
    let largest = sizes
        .values()
        .copied()
        .max()
        .expect("builtin library should contain modules");
    assert!(
        largest <= 53,
        "the largest library check component grew to {largest} modules"
    );
}

/// Coupling condensation dissolves the kernel: only inference cycles
/// stay joint, and the residue stays bounded.
#[test]
fn test_bound_library_inference_components() {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();
    let modules: Vec<_> = package.module_ids().collect();

    // provide export tables for every module
    let keys = modules
        .iter()
        .map(|module| ArtifactKey::dir_exported(*module, profile));
    session.require_all(keys).expect("export artifacts");
    let reader = ArtifactReader::new(repository, session.revision());

    // count every module's inference-coupling exports
    let mut total_inferred = 0usize;
    for module in &modules {
        let exported = reader.dir_exported(*module, profile).expect("exported");
        total_inferred += exported
            .exports
            .exports()
            .filter(|(_, export)| match export {
                dir::NamedExport::Local(local) => local.form.requires_inference(),
                dir::NamedExport::Indirect(_) => false,
            })
            .count();
    }

    // partition the library modules by inference component
    let graph = session.component_graph(profile);
    let mut sizes = FxIndexMap::default();
    for module in &modules {
        let Some(component) = graph.inference_component(*module) else {
            panic!("builtin module {module:?} is absent from the inference component graph");
        };
        *sizes.entry(component).or_insert(0usize) += 1;
    }

    // ratchet the largest inference component and the inferred export count
    let largest = sizes
        .values()
        .copied()
        .max()
        .expect("builtin library should contain inference components");
    assert!(
        largest <= 6,
        "the largest library inference component grew to {largest} modules"
    );
    assert!(
        total_inferred <= 633,
        "the library grew to {total_inferred} inferred exports"
    );
}
