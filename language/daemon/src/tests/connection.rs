use destack_artifact::{ConditionSet, Host, Platform, Runtime};
use destack_core::Blob;
use destack_program::ProgramBuilder;
use destack_repository::Change;
use destack_rpc::{CallError, Code};
use destack_runtime::service::{
    RemoveRuntimeRequest, RunRequest, SpawnRuntimeRequest, WorldRequest,
};
use destack_runtime::world::{Run, RunOutcome};
use destack_source::{Edit, FileId};
use destack_workspace::{
    ApplySourceUpdateRequest, ReadRevisionRequest, SourceUpdate, WatchEvent, WatchRequest,
};

use super::harness::TestDaemon;
use crate::{
    BLOB_CHUNK_BYTE_LEN, CloseWorldRequest, CreateWorldRequest, OpenWorkspaceRequest,
    ReadBlobRequest,
};

/// Round trip workspace operations and daemon shutdown over IPC RPC.
#[test]
fn test_serve_workspace_connection() {
    let daemon = TestDaemon::start("daemon_rpc_connection");
    let connection = daemon.connect();
    let root = daemon.root().to_path_buf();
    let opened = connection
        .daemon()
        .open_workspace(OpenWorkspaceRequest { root: root.clone() })
        .expect("root should open")
        .value;
    assert_eq!(opened.root, root);

    let path = root.join("main.ds");
    let commit = connection
        .workspace()
        .apply_source_update(ApplySourceUpdateRequest {
            root: root.clone(),
            update: SourceUpdate {
                base: Some(opened.revision),
                edits: vec![Edit::SetText {
                    path,
                    text: "export const answer = 42;\n".to_string(),
                }],
            },
        })
        .expect("source update should apply")
        .value;
    let revision = connection
        .workspace()
        .read_revision(ReadRevisionRequest { root })
        .expect("revision should read")
        .value;

    assert_eq!(commit.after, revision);
    assert_ne!(commit.before, commit.after);

    daemon.shutdown(connection);
}

/// Stream exact Blob bytes through one daemon connection.
#[test]
fn test_put_read_blob() {
    let daemon = TestDaemon::start("daemon_blob_connection");
    let connection = daemon.connect();
    let bytes = (0..BLOB_CHUNK_BYTE_LEN * 2 + 17)
        .map(|index| index as u8)
        .collect::<Vec<_>>();

    // stream and publish one Blob larger than one protocol chunk
    let mut put = connection.blob().put(()).expect("Blob put should start");
    for chunk in bytes.chunks(BLOB_CHUNK_BYTE_LEN) {
        put.send(&chunk.to_vec()).expect("Blob chunk should send");
    }
    put.close_input().expect("Blob input should close");
    let blob = put.response().expect("Blob put should complete").value;

    // require the exact published identity through the same daemon store
    let is_present = connection
        .blob()
        .contains(blob)
        .expect("Blob presence should read")
        .value;
    assert_eq!(blob, Blob::for_bytes(&bytes));
    assert!(is_present);

    // stream one exact range back through multiple response items
    let offset = 11_u64;
    let byte_len = BLOB_CHUNK_BYTE_LEN as u64 + 23;
    let mut read = connection
        .blob()
        .read(ReadBlobRequest {
            blob,
            offset,
            byte_len: Some(byte_len),
        })
        .expect("Blob read should start");
    let mut actual = Vec::new();
    while let Some(chunk) = read.receive().expect("Blob chunk should receive") {
        actual.extend_from_slice(&chunk);
    }
    read.response().expect("Blob read should complete");

    let range = offset as usize..(offset + byte_len) as usize;
    assert_eq!(actual, bytes[range]);

    daemon.shutdown(connection);
}

/// Create, run, and close one World over IPC RPC.
#[test]
fn test_serve_world_connection() {
    let daemon = TestDaemon::start("daemon_world_connection");
    let connection = daemon.connect();
    let program = ProgramBuilder::new(Default::default())
        .bytecode(Default::default())
        .build()
        .expect("empty Program should build");

    // publish the complete Program through shared Blob storage
    let mut put = connection.blob().put(()).expect("Program put should start");
    put.send(&program.bytes().to_vec())
        .expect("Program bytes should send");
    put.close_input().expect("Program input should close");
    let program = put.response().expect("Program put should complete").value;

    // create one World and spawn one Runtime from the Program Blob
    let world_id = connection
        .daemon()
        .create_world(CreateWorldRequest {
            options: None,
            environment: None,
        })
        .expect("World should create")
        .value;
    let runtime_id = connection
        .world()
        .spawn_runtime(SpawnRuntimeRequest {
            world_id,
            program,
            options: None,
            environment: None,
            conditions: ConditionSet {
                modes: Default::default(),
                roles: Default::default(),
                features: Default::default(),
                tags: Default::default(),
                target: None,
                product: None,
                role: None,
                labels: Default::default(),
                stage: None,
                platform: Platform::Unknown,
                host: Host::Native,
                runtime: Runtime::Destack,
            },
        })
        .expect("Runtime should spawn")
        .value;

    // observe the exact hosted state and idle run outcome
    let runtime_ids = connection
        .world()
        .list_runtimes(WorldRequest { world_id })
        .expect("Runtimes should list")
        .value;
    let moment = connection
        .world()
        .read_moment(WorldRequest { world_id })
        .expect("Moment should read")
        .value;
    let outcome = connection
        .world()
        .run(RunRequest {
            world_id,
            run: Run::Task,
        })
        .expect("World should run")
        .value;

    assert_eq!(runtime_ids, vec![runtime_id]);
    assert_eq!(moment.sequence.get(), 1);
    assert_eq!(outcome, RunOutcome::Idle);

    // remove the Runtime and close the World through their owning services
    connection
        .world()
        .remove_runtime(RemoveRuntimeRequest {
            world_id,
            runtime_id,
        })
        .expect("Runtime should remove");
    connection
        .daemon()
        .close_world(CloseWorldRequest { world_id })
        .expect("World should close");
    let error = connection
        .world()
        .read_moment(WorldRequest { world_id })
        .expect_err("closed World should not resolve");
    let CallError::Status(status) = error else {
        panic!("closed World should return RPC status, got {error}");
    };

    assert_eq!(status.code, Code::NotFound);
    assert_eq!(status.message, "World 1 is not open");

    daemon.shutdown(connection);
}

/// Keep one workspace root alive when another client disconnects.
#[test]
fn test_share_workspace_root_across_connections() {
    let daemon = TestDaemon::start("daemon_shared_workspace_root");
    let first = daemon.connect();
    let second = daemon.connect();
    let root = daemon.root().to_path_buf();
    let first_opened = first
        .daemon()
        .open_workspace(OpenWorkspaceRequest { root: root.clone() })
        .expect("first root should open")
        .value;
    let second_opened = second
        .daemon()
        .open_workspace(OpenWorkspaceRequest { root: root.clone() })
        .expect("second root should open")
        .value;

    assert_eq!(first_opened, second_opened);

    first.close().expect("first connection should close");
    let revision = second
        .workspace()
        .read_revision(ReadRevisionRequest { root })
        .expect("shared root should remain open")
        .value;

    assert_eq!(revision, second_opened.revision);

    daemon.shutdown(second);
}

/// Publish one physical change as the same semantic commit to every client.
#[test]
fn test_share_physical_workspace_commit_across_connections() {
    let daemon = TestDaemon::start("daemon_shared_physical_commit");
    let first = daemon.connect();
    let second = daemon.connect();
    let root = daemon.root().to_path_buf();
    let mut first_watch = first
        .workspace()
        .watch(WatchRequest { root: root.clone() })
        .expect("first watch should start");
    let mut second_watch = second
        .workspace()
        .watch(WatchRequest { root: root.clone() })
        .expect("second watch should start");

    // establish both subscriptions at one exact revision
    let first_ready = first_watch
        .receive()
        .expect("first ready should receive")
        .expect("first ready should exist");
    let second_ready = second_watch
        .receive()
        .expect("second ready should receive")
        .expect("second ready should exist");
    assert_eq!(first_ready, second_ready);
    assert!(matches!(first_ready, WatchEvent::Ready { .. }));

    // write disk truth through the daemon owned host watch
    daemon.write_text("src/main.ds", "export const answer = 42;\n");
    let first_commit = first_watch
        .receive()
        .expect("first commit should receive")
        .expect("first commit should exist");
    let second_commit = second_watch
        .receive()
        .expect("second commit should receive")
        .expect("second commit should exist");

    assert_eq!(first_commit, second_commit);
    let WatchEvent::Commit(commit) = first_commit else {
        panic!("physical change should produce one semantic commit");
    };
    assert_eq!(
        commit.changes,
        vec![Change {
            file: FileId::from_logical_str("src/main.ds"),
            path: "src/main.ds".to_string(),
            before: None,
            after: Some(Blob::for_bytes(b"export const answer = 42;\n")),
        }]
    );
    let revision = second
        .workspace()
        .read_revision(ReadRevisionRequest { root })
        .expect("shared revision should read")
        .value;
    assert_eq!(commit.after, revision);

    first_watch.cancel().expect("first watch should cancel");
    second_watch.cancel().expect("second watch should cancel");
    assert_eq!(
        first_watch.receive().expect("first watch should finish"),
        None
    );
    assert_eq!(
        second_watch.receive().expect("second watch should finish"),
        None
    );
    let first_error = first_watch
        .response()
        .expect_err("first cancellation should fail the call");
    let second_error = second_watch
        .response()
        .expect_err("second cancellation should fail the call");
    let CallError::Status(first_status) = first_error else {
        panic!("expected first cancellation status, got {first_error}");
    };
    let CallError::Status(second_status) = second_error else {
        panic!("expected second cancellation status, got {second_error}");
    };

    assert_eq!(first_status.code, Code::Canceled);
    assert_eq!(first_status.message, "call did not complete");
    assert_eq!(second_status, first_status);

    first.close().expect("first connection should close");
    daemon.shutdown(second);
}
