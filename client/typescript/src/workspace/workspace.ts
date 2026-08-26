import type { Blob } from "../_generated/core/blob.js";
import {
    DaemonClient,
    daemonService,
} from "../_generated/daemon/daemon.js";
import type { Revision } from "../_generated/repository/revision.js";
import {
    WorkspaceClient,
    workspaceService,
} from "../_generated/workspace/workspace.js";
import type { DiagnosticsRequest } from "../_generated/workspace/diagnostic/file.js";
import type { ArtifactRequest, ExportRequest } from "../_generated/workspace/service/artifact.js";
import type {
    BuildRequest,
    CacheRequest,
    CheckRequest,
    CleanRequest,
    DocRequest,
    DoctorRequest,
    FormatRequest,
    InfoRequest,
    QueryRequest,
    RewriteRequest,
    SettingsRequest,
    TargetsRequest,
    TaskRequest,
    TestRequest,
} from "../_generated/workspace/service/command.js";
import type {
    ResolveQueryFileRequest,
    RunQueryRequest,
} from "../_generated/workspace/service/query.js";
import type {
    DiffRequest,
    EditRequest,
    FormatFileRequest,
    ReadFilesRequest,
} from "../_generated/workspace/service/source.js";
import {
    Connection,
    type ConnectionOptions,
} from "../rpc/index.js";
import { hasNodeProcess } from "../runtime.js";
import type { NapiWorkspaceOptions } from "../napi.js";
import type { WasmWorkspaceOptions } from "../wasm.js";
import type {
    MemoryContent,
    MemoryFile,
    MemoryWorkspace,
    MemoryWorkspaceOptions,
} from "./memory.js";
import { isMemoryWorkspaceOptions } from "./memory.js";
import { Branch } from "./branch.js";

/** Location of one workspace served by a remote daemon. */
export type RemoteWorkspaceOptions = {
    /** RPC WebSocket endpoint. */
    readonly url: string | URL;
    /** Root to open through the daemon. */
    readonly root: string;
    /** RPC negotiation overrides. */
    readonly connection?: ConnectionOptions;
};

/** Options for opening one local workspace. */
export type LocalWorkspaceOptions = NapiWorkspaceOptions | WasmWorkspaceOptions;

/** Options for opening one local or remote workspace. */
export type WorkspaceOptions = LocalWorkspaceOptions | RemoteWorkspaceOptions;

/** One root-bound workspace over a composable RPC connection. */
export class Workspace {
    /** Shared RPC connection. */
    readonly connection: Connection;
    /** Complete generated workspace service client. */
    readonly client: WorkspaceClient;
    /** Canonical opened root. */
    readonly root: string;

    /** Bind one opened workspace to a negotiated connection. */
    constructor(connection: Connection, root: string) {
        this.connection = connection;
        this.client = new WorkspaceClient(connection);
        this.root = root;
    }

    // =============================================================================
    // Physical
    // =============================================================================

    /** Read the current physical revision. */
    async revision(): Promise<Revision> {
        const response = await this.client.revision({ root: this.root });

        return response.value;
    }

    /** Reload physical workspace state from its host. */
    async reload() {
        const response = await this.client.reload({ root: this.root });

        return response.value;
    }

    /** Commit source edits to disk and physical state. */
    async edit(request: Omit<EditRequest, "root">) {
        const response = await this.client.edit({ root: this.root, ...request });

        return response.value;
    }

    /** Watch physical state for exact committed transitions. */
    watch() {
        return this.client.watch({ root: this.root });
    }

    // =============================================================================
    // Branch
    // =============================================================================

    /** Bind one known workspace branch. */
    branch(name: string): Branch {
        return new Branch(this, name);
    }

    /** List every branch in this workspace. */
    async branches(): Promise<Branch[]> {
        const response = await this.client.listBranches({ root: this.root });

        return response.value.map(({ name }) => this.branch(name));
    }

    /** Create one branch at an exact revision or current physical state. */
    async createBranch(name: string, revision?: Revision): Promise<Branch> {
        revision ??= await this.revision();
        await this.client.createBranch({ root: this.root, name, revision });

        return this.branch(name);
    }

    // =============================================================================
    // Source
    // =============================================================================

    /** Compare two exact revisions. */
    async diff(request: Omit<DiffRequest, "root">) {
        const response = await this.client.diff({ root: this.root, ...request });

        return response.value;
    }

    /** List files at one exact revision. */
    async files(revision: Revision) {
        const response = await this.client.listFiles({ root: this.root, revision });

        return response.value;
    }

    /** Format one file in this root. */
    async formatFile(request: Omit<FormatFileRequest, "root">) {
        const response = await this.client.formatFile({ root: this.root, ...request });

        return response.value;
    }

    /** Read source files from one exact revision of this root. */
    async readFiles(request: Omit<ReadFilesRequest, "root">) {
        const response = await this.client.readFiles({ root: this.root, ...request });

        return response.value;
    }

    // =============================================================================
    // Command
    // =============================================================================

    /** Check source in this root. */
    check(input: CheckRequest["input"]) {
        return this.client.check({ root: this.root, input });
    }

    /** Format source in this root. */
    format(input: FormatRequest["input"]) {
        return this.client.format({ root: this.root, input });
    }

    /** Query source in this root. */
    query(input: QueryRequest["input"]) {
        return this.client.query({ root: this.root, input });
    }

    /** Rewrite source in this root. */
    rewrite(input: RewriteRequest["input"]) {
        return this.client.rewrite({ root: this.root, input });
    }

    /** Build targets in this root. */
    build(input: BuildRequest["input"]) {
        return this.client.build({ root: this.root, input });
    }

    /** Run tests in this root. */
    test(input: TestRequest["input"]) {
        return this.client.test({ root: this.root, input });
    }

    /** Generate documentation for this root. */
    doc(input: DocRequest["input"]) {
        return this.client.doc({ root: this.root, input });
    }

    /** Return information about this root. */
    info(input: InfoRequest["input"]) {
        return this.client.info({ root: this.root, input });
    }

    /** Return configured targets in this root. */
    targets(input: TargetsRequest["input"]) {
        return this.client.targets({ root: this.root, input });
    }

    /** Return cache locations for this root. */
    cache(input: CacheRequest["input"]) {
        return this.client.cache({ root: this.root, input });
    }

    /** Return resolved settings for this root. */
    settings(input: SettingsRequest["input"]) {
        return this.client.settings({ root: this.root, input });
    }

    /** Diagnose configuration and state in this root. */
    doctor(input: DoctorRequest["input"]) {
        return this.client.doctor({ root: this.root, input });
    }

    /** Execute configured tasks in this root. */
    task(input: TaskRequest["input"]) {
        return this.client.task({ root: this.root, input });
    }

    /** Clean generated state in this root. */
    clean(input: CleanRequest["input"]) {
        return this.client.clean({ root: this.root, input });
    }

    // =============================================================================
    // Artifact
    // =============================================================================

    /** Read one exact artifact from this root. */
    artifact(artifact: ArtifactRequest["artifact"]) {
        return this.client.artifact({ root: this.root, artifact });
    }

    /** Publish one artifact's storage bytes as a Blob. */
    async blob(artifact: ArtifactRequest["artifact"]): Promise<Blob> {
        const response = await this.client.blob({ root: this.root, artifact });

        return response.value;
    }

    /** Materialize one artifact through this root's host. */
    export(input: ExportRequest["input"]) {
        return this.client.export({ root: this.root, input });
    }

    // =============================================================================
    // Diagnostic
    // =============================================================================

    /** Read diagnostics from one exact revision. */
    async diagnose(revision: Revision, request: DiagnosticsRequest = { kind: "all" }) {
        const response = await this.client.diagnose({ root: this.root, revision, request });

        return response.value;
    }

    // =============================================================================
    // Query
    // =============================================================================

    /** Resolve one source file for semantic queries. */
    async resolveQueryFile(request: Omit<ResolveQueryFileRequest, "root">) {
        const response = await this.client.resolveQueryFile({ root: this.root, ...request });

        return response.value;
    }

    /** Execute one semantic query in this root. */
    runQuery(input: RunQueryRequest["input"]) {
        return this.client.runQuery({ root: this.root, input });
    }
}

/** Connect to a remote daemon and open one workspace root. */
export async function openRemoteWorkspace(
    options: RemoteWorkspaceOptions,
): Promise<Workspace> {
    const connection = await Connection.connectWebSocket(
        options.url,
        [daemonService, workspaceService],
        options.connection,
    );
    try {
        const daemon = new DaemonClient(connection);
        const response = await daemon.openWorkspace({ root: options.root });

        return new Workspace(connection, response.value.root);
    } catch (error) {
        connection.close();

        throw error;
    }
}

/** Open a workspace through its selected remote or local host. */
export async function openWorkspace(options: WorkspaceOptions): Promise<Workspace> {
    if (isRemoteWorkspaceOptions(options)) {
        return openRemoteWorkspace(options);
    }
    if (hasNodeProcess()) {
        const { openNapiWorkspace } = await import("../napi.js");

        return openNapiWorkspace(options);
    }
    if (!isMemoryWorkspaceOptions(options)) {
        throw new Error("physical workspaces require a Node.js host");
    }

    const { openWasmWorkspace } = await import("../wasm.js");

    return openWasmWorkspace(options);
}

/** Open one local workspace through the best available host. */
export function openLocalWorkspace(options: LocalWorkspaceOptions): Promise<Workspace> {
    return openWorkspace(options);
}

/** Return whether workspace options select a remote endpoint. */
function isRemoteWorkspaceOptions(options: WorkspaceOptions): options is RemoteWorkspaceOptions {
    return "url" in options;
}

export { Branch, WorkspaceClient, workspaceService };
export type {
    MemoryContent,
    MemoryFile,
    MemoryWorkspace,
    MemoryWorkspaceOptions,
};
