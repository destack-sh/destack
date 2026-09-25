use std::sync::Arc;

use tspp_artifact::{ArtifactKey, ArtifactPayload, ArtifactReference};
use tspp_program::ProgramBuilder;
use tspp_source::{PackageId, TargetId};

use crate::tests::harness::TestWorkspace;

/// Publish the exact storage bytes of one Program artifact as a Blob.
#[test]
fn test_publish_program_artifact_as_blob() {
    let test = TestWorkspace::new("build-program-blob");
    let program = ProgramBuilder::new(Default::default())
        .bytecode(Default::default())
        .build()
        .expect("Program should build");

    // publish one exact Program artifact in the current revision
    let package = PackageId::new(1);
    let target = TargetId::new(package, "default");
    let key = ArtifactKey::program(package, target);
    let revision = test.workspace.revision().expect("revision should read");
    test.workspace
        .repository
        .complete_artifact(
            revision,
            key,
            ArtifactPayload::Program(Arc::new(program)),
            Vec::new(),
            Vec::new(),
            None,
        )
        .expect("Program artifact should publish");
    let version = test.workspace.repository.artifact_identity(key, &[]);
    let reference = ArtifactReference { key, version };

    // publish the exact encoded Program bytes
    let payload = test
        .workspace
        .artifact(reference)
        .expect("Program artifact should read");
    let ArtifactPayload::Program(payload) = payload else {
        panic!("Program reference should resolve to a Program payload");
    };
    let blob = test
        .workspace
        .blob(reference)
        .expect("Program Blob should publish");
    let memory = test
        .workspace
        .repository
        .blob_store()
        .open(blob)
        .expect("Program Blob should open");

    assert_eq!(memory.bytes(), payload.bytes());
}
