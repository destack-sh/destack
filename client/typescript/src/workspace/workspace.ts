import {
    WorkspaceClient,
    workspaceService,
} from "../_generated/workspace/workspace.js";
import { daemonService } from "../_generated/daemon/daemon.js";
import type { Content, ContentId } from "../_generated/source/file/model/file.js";
import type { DiagnosticsRequest } from "../_generated/workspace/diagnostic/file.js";
import type { ArtifactRequest, ExportRequest } from "../_generated/workspace/service/artifact.js";
import type {
    BenchRequest,
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
import type { FileOperation } from "../_generated/workspace/file/image.js";
import type { SourceUpdate } from "../_generated/workspace/update.js";
import type {
    FormatFileRequest,
    ReadFilesRequest,
} from "../_generated/workspace/service/source.js";
import {
    Connection,
    type ConnectionOptions,
} from "../rpc/index.js";
import { Daemon } from "../daemon/daemon.js";
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
    /** Bind one workspace root to a negotiated connection. */
    constructor(connection: Connection, root: string) {
        this.connection = connection;
        this.client = new WorkspaceClient(connection);
        this.root = root;
    }

    /** Return this root's current revision. */
    revision() {
        return this.client.readRevision({ root: this.root });
    }

    /** Reload this root from its host. */
    reload() {
        return this.client.reload({ root: this.root });
    }

    /** Watch this root for revision changes. */
    watch() {
        return this.client.watch({ root: this.root });
    }

    /** Apply one physical file operation to this root. */
    applyFileOperation(operation: FileOperation) {
        return this.client.applyFileOperation({ root: this.root, operation });
    }

    /** Apply one source update to this root. */
    applySourceUpdate(update: SourceUpdate) {
        return this.client.applySourceUpdate({ root: this.root, update });
    }

    /** Format one file in this root. */
    formatFile(request: Omit<FormatFileRequest, "root">) {
        return this.client.formatFile({ root: this.root, ...request });
    }

    /** Return whether one file is open in this root. */
    isFileOpen(path: string) {
        return this.client.isFileOpen({ root: this.root, path });
    }

    /** Read source files from one exact revision of this root. */
    readFiles(request: Omit<ReadFilesRequest, "root">) {
        return this.client.readFiles({ root: this.root, ...request });
    }

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

    /** Run benchmarks in this root. */
    bench(input: BenchRequest["input"]) {
        return this.client.bench({ root: this.root, input });
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

    /** Read one exact artifact from this root. */
    artifact(artifact: ArtifactRequest["artifact"]) {
        return this.client.artifact({ root: this.root, artifact });
    }

    /** Store one content value through this workspace. */
    store(content: Content) {
        return this.client.store({ root: this.root, content });
    }

    /** Load one content value through this workspace. */
    load(content: ContentId) {
        return this.client.load({ root: this.root, content });
    }

    /** Materialize one artifact through this root's host. */
    export(input: ExportRequest["input"]) {
        return this.client.export({ root: this.root, input });
    }

    /** Read diagnostics for this root. */
    diagnose(request: DiagnosticsRequest = { kind: "all" }) {
        return this.client.diagnose({ root: this.root, request });
    }

    /** Resolve one source file for semantic queries. */
    resolveQueryFile(path: ResolveQueryFileRequest["path"]) {
        return this.client.resolveQueryFile({ root: this.root, path });
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
        const daemon = new Daemon(connection);

        return await daemon.openWorkspace(options.root);
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

export { WorkspaceClient, workspaceService };
export type {
    MemoryContent,
    MemoryFile,
    MemoryWorkspace,
    MemoryWorkspaceOptions,
};
