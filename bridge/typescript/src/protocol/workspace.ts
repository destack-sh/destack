import type { DiagnosticBatch } from "../_generated/protocol/notification.js";
import type { ProtocolError } from "../_generated/protocol/error.js";
import type { ArtifactReference } from "../_generated/artifact/reference.js";
import type {
    FileOperation,
    FileImage,
} from "../_generated/protocol/workspace/file/image.js";
import {
    Content,
    type ContentId,
} from "../_generated/source/file/model/file.js";
import type { FileType } from "../_generated/source/file/model/type.js";
import type { SourceUpdate } from "../_generated/protocol/workspace/file/update.js";
import type { RootId, SourceUpdateResponse } from "../_generated/protocol/root.js";
import type { ArtifactBlob } from "../_generated/protocol/response.js";
import type { Revision } from "../_generated/repository/revision.js";
import type {
    DiagnosticSnapshot,
    FileImagesRequest,
    FileSnapshot,
    FileSnapshotRequest,
    RootSnapshot,
    WorkspaceQuery,
    WorkspaceQueryResponse,
} from "../_generated/protocol/query.js";
import { WorkspaceRequest } from "../_generated/protocol/request.js";
import type { WorkspaceResponse } from "../_generated/protocol/response.js";
import type { UpdateBatch } from "../_generated/protocol/workspace/message.js";
import { WorkspaceClient } from "../_generated/protocol/workspace/client.js";
import type { ReloadReason } from "../_generated/protocol/workspace/root.js";
import type {
    WatchBatchResponse,
    WatchStartOptions,
    WatchStartedResponse,
    WatchStoppedResponse,
} from "../_generated/protocol/watch.js";
import type { BenchInput } from "../_generated/protocol/workspace/command/bench.js";
import type { BuildInput, BuildOutputs } from "../_generated/protocol/workspace/command/build.js";
import type { CacheInput } from "../_generated/protocol/workspace/command/cache.js";
import type { CheckInput, LintInput } from "../_generated/protocol/workspace/command/check.js";
import type { CleanInput } from "../_generated/protocol/workspace/command/clean.js";
import type { DocInput } from "../_generated/protocol/workspace/command/doc.js";
import type { DoctorInput } from "../_generated/protocol/workspace/command/doctor.js";
import {
    FormatSource,
    type FormatInput,
} from "../_generated/protocol/workspace/command/format.js";
import type { InfoInput } from "../_generated/protocol/workspace/command/info.js";
import type {
    BenchOutput,
    BuildOutput,
    CacheOutput,
    CheckOutput,
    CleanOutput,
    DocOutput,
    DoctorOutput,
    FormatOutput,
    InfoOutput,
    LintOutput,
    RunOutput,
    SettingsOutput,
    TargetsOutput,
    TaskOutput,
    TestOutput,
} from "../_generated/protocol/workspace/command/output.js";
import { RunMode, type RunInput } from "../_generated/protocol/workspace/command/run.js";
import type { SettingsInput } from "../_generated/protocol/workspace/command/settings.js";
import type { TargetsInput } from "../_generated/protocol/workspace/command/targets.js";
import { TaskAction, type TaskInput } from "../_generated/protocol/workspace/command/task.js";
import type { TestInput } from "../_generated/protocol/workspace/command/test.js";
import {
    CommandRevision,
    type CommandEnvVar,
    type CommandInput,
    type CommandTargetOverrides,
    type ManifestOverride,
} from "../_generated/protocol/workspace/command/common.js";
import type {
    ExportRequest,
    ExportResult,
} from "../_generated/protocol/workspace/artifact/export.js";
import { Connection, connectEndpoint } from "./connection/index.js";

const DEFAULT_WATCH_COALESCE_WINDOW_MS = 50n;
const DEFAULT_WATCH_BATCH_SIZE = 1024;

/** Options for opening a remote workspace root. */
export type RemoteWorkspaceOptions = {
    /** Connected workspace protocol client. */
    readonly connection?: Connection;
    /** WebSocket workspace endpoint URL. */
    readonly url?: string | URL;
    /** Workspace root used by the server to identify the workspace. */
    readonly workspace: string;
    /** Root path to open inside the workspace. */
    readonly root?: string;
    /** Whether opening should preload root diagnostics. */
    readonly loadIndex?: boolean;
};

/** Shared command input fields accepted by remote workspace commands. */
type CommandInputFields = {
    /** Revision selected for command execution. */
    readonly revision: CommandRevision;
    /** Input sources for the command. */
    readonly inputs: ReadonlyArray<CommandInput>;
    /** Whether destack.json should resolve inputs when none are provided. */
    readonly configInputs: boolean;
    /** Optional working directory for this command. */
    readonly cwd?: string;
    /** Optional Destack manifest path override. */
    readonly manifest?: string;
    /** Optional target name override. */
    readonly target?: string;
    /** Optional target overrides. */
    readonly targetOverrides?: CommandTargetOverrides;
    /** Optional profile name override. */
    readonly profile?: string;
    /** Optional environment overrides. */
    readonly env: ReadonlyArray<CommandEnvVar>;
    /** Optional manifest overrides. */
    readonly overrides: ReadonlyArray<ManifestOverride>;
    /** Whether the command should watch for changes. */
    readonly watch: boolean;
    /** Whether the command should skip writes. */
    readonly dryRun: boolean;
};

/** Sparse shared command input accepted by remote workspace commands. */
export type CommandInputInit = Partial<CommandInputFields>;

/** Sparse check input accepted by `RemoteWorkspace.check`. */
export type CheckInputInit = CommandInputInit & Partial<Omit<CheckInput, keyof CommandInputFields>>;

/** Sparse lint input accepted by `RemoteWorkspace.lint`. */
export type LintInputInit = CommandInputInit & Partial<Omit<LintInput, keyof CommandInputFields>>;

/** Sparse format input accepted by `RemoteWorkspace.format`. */
export type FormatInputInit = CommandInputInit &
    Partial<Omit<FormatInput, keyof CommandInputFields | "source">> & {
        /** Source selected for formatting. */
        readonly source?: FormatSourceInit;
    };

/** Formatting source accepted by `RemoteWorkspace.format`. */
export type FormatSourceInit =
    | FormatSource
    | string
    | ReadonlyArray<string>
    | {
          /** Files or directories to format. */
          readonly files: ReadonlyArray<string>;
      }
    | {
          /** Open file to format. */
          readonly openFile: string;
      }
    | {
          /** Stored content to format. */
          readonly content: ContentId;
          /** Stored content file type. */
          readonly fileType?: FileType;
          /** Input label. */
          readonly name?: string;
      };

/** Sparse build input accepted by `RemoteWorkspace.build`. */
export type BuildInputInit = CommandInputInit & Partial<Omit<BuildInput, keyof CommandInputFields>>;

/** Sparse run input accepted by `RemoteWorkspace.run`. */
export type RunInputInit = CommandInputInit & Partial<Omit<RunInput, keyof CommandInputFields>>;

/** Sparse test input accepted by `RemoteWorkspace.test`. */
export type TestInputInit = CommandInputInit & Partial<Omit<TestInput, keyof CommandInputFields>>;

/** Sparse doc input accepted by `RemoteWorkspace.doc`. */
export type DocInputInit = CommandInputInit & Partial<Omit<DocInput, keyof CommandInputFields>>;

/** Sparse bench input accepted by `RemoteWorkspace.bench`. */
export type BenchInputInit = CommandInputInit & Partial<Omit<BenchInput, keyof CommandInputFields>>;

/** Sparse info input accepted by `RemoteWorkspace.info`. */
export type InfoInputInit = CommandInputInit & Partial<Omit<InfoInput, keyof CommandInputFields>>;

/** Sparse targets input accepted by `RemoteWorkspace.targets`. */
export type TargetsInputInit = CommandInputInit & Partial<Omit<TargetsInput, keyof CommandInputFields>>;

/** Sparse cache input accepted by `RemoteWorkspace.cache`. */
export type CacheInputInit = CommandInputInit & Partial<Omit<CacheInput, keyof CommandInputFields>>;

/** Sparse settings input accepted by `RemoteWorkspace.settings`. */
export type SettingsInputInit = CommandInputInit & Partial<Omit<SettingsInput, keyof CommandInputFields>>;

/** Sparse doctor input accepted by `RemoteWorkspace.doctor`. */
export type DoctorInputInit = CommandInputInit & Partial<Omit<DoctorInput, keyof CommandInputFields>>;

/** Sparse task input accepted by `RemoteWorkspace.task`. */
export type TaskInputInit = CommandInputInit & Partial<Omit<TaskInput, keyof CommandInputFields>>;

/** Sparse clean input accepted by `RemoteWorkspace.clean`. */
export type CleanInputInit = CommandInputInit & Partial<Omit<CleanInput, keyof CommandInputFields>>;

/** Sparse watch options accepted by `RemoteWorkspace.watch`. */
export type WatchOptionsInit = Partial<WatchStartOptions>;

/** Workspace command and query interface. */
export interface Workspace {
    /** Return the protocol connection backing this workspace. */
    connection(): Connection;
    /** Return the workspace path registered with the server. */
    workspace(): string;
    /** Return the opened root path. */
    root(): string;
    /** Return the opened protocol root handle. */
    handle(): RootId;
    /** Return the current root revision. */
    revision(): Promise<Revision>;
    /** Return plain diagnostic batches for this root. */
    diagnostics(): Promise<readonly DiagnosticBatch[]>;
    /** Return diagnostic snapshots with source file images for this root. */
    diagnosticSnapshots(): Promise<readonly DiagnosticSnapshot[]>;
    /** Return query context for this root. */
    snapshot(target?: string): Promise<RootSnapshot>;
    /** Return one source file snapshot. */
    fileSnapshot(request: FileSnapshotRequest): Promise<FileSnapshot | undefined>;
    /** Return source file images for one revision. */
    fileImages(request: FileImagesRequest): Promise<readonly FileImage[]>;
    /** Apply one file operation through the workspace protocol. */
    applyFileOperation(operation: FileOperation): Promise<UpdateBatch>;
    /** Apply a source update through the workspace protocol. */
    applySourceUpdate(update: SourceUpdate): Promise<SourceUpdateResponse>;
    /** Reload this root from the server filesystem. */
    reload(reason?: ReloadReason): Promise<UpdateBatch>;
    /** Check source state. */
    check(request?: CheckInputInit): Promise<CheckOutput>;
    /** Lint source state. */
    lint(request?: LintInputInit): Promise<LintOutput>;
    /** Format source files or content. */
    format(request?: FormatInputInit): Promise<FormatOutput>;
    /** Build target artifacts. */
    build(request?: BuildInputInit): Promise<BuildOutput>;
    /** Run one workspace target. */
    run(request?: RunInputInit): Promise<RunOutput>;
    /** Run workspace tests. */
    test(request?: TestInputInit): Promise<TestOutput>;
    /** Generate documentation. */
    doc(request?: DocInputInit): Promise<DocOutput>;
    /** Run benchmarks. */
    bench(request?: BenchInputInit): Promise<BenchOutput>;
    /** Return workspace information. */
    info(request?: InfoInputInit): Promise<InfoOutput>;
    /** Return configured targets. */
    targets(request?: TargetsInputInit): Promise<TargetsOutput>;
    /** Return cache locations. */
    cache(request?: CacheInputInit): Promise<CacheOutput>;
    /** Return resolved settings. */
    settings(request?: SettingsInputInit): Promise<SettingsOutput>;
    /** Return workspace health information. */
    doctor(request?: DoctorInputInit): Promise<DoctorOutput>;
    /** Run workspace tasks. */
    task(request?: TaskInputInit): Promise<TaskOutput>;
    /** Clean generated state. */
    clean(request?: CleanInputInit): Promise<CleanOutput>;
    /** Return one artifact payload. */
    artifact(artifact: ArtifactReference): Promise<ArtifactBlob>;
    /** Store one content payload. */
    store(content: Content | string | Uint8Array | readonly number[]): Promise<ContentId>;
    /** Load one content payload. */
    load(content: ContentId): Promise<Content>;
    /** Materialize derived outputs on the workspace host. */
    export(request: ExportRequest): Promise<ExportResult>;
    /** Run one workspace query against this root. */
    query(query: WorkspaceQuery): Promise<WorkspaceQueryResponse>;
    /** Watch roots through this workspace handle. */
    watch(roots?: readonly string[], options?: WatchOptionsInit): Promise<WatchStartedResponse>;
    /** Receive and apply the next watch batch. */
    nextWatch(): Promise<WatchBatchResponse>;
    /** Stop watching roots through this workspace handle. */
    unwatch(): Promise<WatchStoppedResponse>;
    /** Close this root handle on the workspace server. */
    close(): Promise<void>;
}

/** Workspace backed by the workspace protocol. */
export class RemoteWorkspace implements Workspace {
    readonly #client: WorkspaceClient;
    readonly #workspace: string;
    readonly #root: string;

    private constructor(
        connection: Connection,
        workspace: string,
        root: string,
        handle: RootId,
    ) {
        this.#client = new WorkspaceClient(connection, handle);
        this.#workspace = workspace;
        this.#root = root;
    }

    /** Open one remote workspace root. */
    static async open(options: RemoteWorkspaceOptions): Promise<RemoteWorkspace> {
        const connection = await remoteConnection(options);
        await connection.handshake();

        const root = options.root ?? options.workspace;
        const response = await connection.request(WorkspaceRequest.openRoot({
            root,
            options: {
                loadIndex: options.loadIndex ?? false,
            },
        }));
        const opened = expectResponse(response, "rootOpened").root_opened;

        return new RemoteWorkspace(connection, options.workspace, opened.root, opened.handle);
    }

    /** Return the protocol connection backing this workspace. */
    connection(): Connection {
        return this.#client.connection();
    }

    /** Return the workspace path registered with the server. */
    workspace(): string {
        return this.#workspace;
    }

    /** Return the opened root path. */
    root(): string {
        return this.#root;
    }

    /** Return the opened protocol root handle. */
    handle(): RootId {
        return this.#client.handle();
    }

    /** Return the current root revision. */
    async revision(): Promise<Revision> {
        return this.#client.currentRevision();
    }

    /** Return plain diagnostic batches for this root. */
    async diagnostics(): Promise<readonly DiagnosticBatch[]> {
        return this.#client.diagnostics();
    }

    /** Return diagnostic snapshots with source file images for this root. */
    async diagnosticSnapshots(): Promise<readonly DiagnosticSnapshot[]> {
        return this.#client.diagnosticSnapshots();
    }

    /** Return query context for this root. */
    async snapshot(target?: string): Promise<RootSnapshot> {
        return this.#client.rootSnapshot(target);
    }

    /** Return one source file snapshot. */
    async fileSnapshot(request: FileSnapshotRequest): Promise<FileSnapshot | undefined> {
        return this.#client.fileSnapshot(request);
    }

    /** Return source file images for one revision. */
    async fileImages(request: FileImagesRequest): Promise<readonly FileImage[]> {
        return this.#client.fileImages(request);
    }

    /** Apply one file operation through the workspace protocol. */
    async applyFileOperation(operation: FileOperation): Promise<UpdateBatch> {
        const response = await this.#client.applyFileOperation(operation);

        return response.updates;
    }

    /** Apply a source update through the workspace protocol. */
    async applySourceUpdate(update: SourceUpdate): Promise<SourceUpdateResponse> {
        return this.#client.applySourceUpdate(update);
    }

    /** Reload this root from the server filesystem. */
    async reload(reason: ReloadReason = "manual"): Promise<UpdateBatch> {
        const response = await this.#client.reloadRoot(reason);

        return response.updates;
    }

    /** Check source state. */
    async check(request: CheckInputInit = {}): Promise<CheckOutput> {
        return this.#client.check(checkInput(request));
    }

    /** Lint source state. */
    async lint(request: LintInputInit = {}): Promise<LintOutput> {
        return this.#client.lint(lintInput(request));
    }

    /** Format source files or content. */
    async format(request: FormatInputInit = {}): Promise<FormatOutput> {
        return this.#client.format(formatInput(request));
    }

    /** Build target artifacts. */
    async build(request: BuildInputInit = {}): Promise<BuildOutput> {
        return this.#client.build(buildInput(request));
    }

    /** Run one workspace target. */
    async run(request: RunInputInit = {}): Promise<RunOutput> {
        return this.#client.run(runInput(request));
    }

    /** Run workspace tests. */
    async test(request: TestInputInit = {}): Promise<TestOutput> {
        return this.#client.test(testInput(request));
    }

    /** Generate documentation. */
    async doc(request: DocInputInit = {}): Promise<DocOutput> {
        return this.#client.doc(docInput(request));
    }

    /** Run benchmarks. */
    async bench(request: BenchInputInit = {}): Promise<BenchOutput> {
        return this.#client.bench(benchInput(request));
    }

    /** Return workspace information. */
    async info(request: InfoInputInit = {}): Promise<InfoOutput> {
        return this.#client.info(infoInput(request));
    }

    /** Return configured targets. */
    async targets(request: TargetsInputInit = {}): Promise<TargetsOutput> {
        return this.#client.targets(targetsInput(request));
    }

    /** Return cache locations. */
    async cache(request: CacheInputInit = {}): Promise<CacheOutput> {
        return this.#client.cache(cacheInput(request));
    }

    /** Return resolved settings. */
    async settings(request: SettingsInputInit = {}): Promise<SettingsOutput> {
        return this.#client.settings(settingsInput(request));
    }

    /** Return workspace health information. */
    async doctor(request: DoctorInputInit = {}): Promise<DoctorOutput> {
        return this.#client.doctor(doctorInput(request));
    }

    /** Run workspace tasks. */
    async task(request: TaskInputInit = {}): Promise<TaskOutput> {
        return this.#client.task(taskInput(request));
    }

    /** Clean generated state. */
    async clean(request: CleanInputInit = {}): Promise<CleanOutput> {
        return this.#client.clean(cleanInput(request));
    }

    /** Return one artifact payload. */
    async artifact(artifact: ArtifactReference): Promise<ArtifactBlob> {
        return this.#client.artifact(artifact);
    }

    /** Store one content payload. */
    async store(content: Content | string | Uint8Array | readonly number[]): Promise<ContentId> {
        return this.#client.store(contentPayload(content));
    }

    /** Load one content payload. */
    async load(content: ContentId): Promise<Content> {
        return this.#client.load(content);
    }

    /** Materialize derived outputs on the workspace host. */
    async export(request: ExportRequest): Promise<ExportResult> {
        return this.#client.export(request);
    }

    /** Run one workspace query against this root. */
    async query(query: WorkspaceQuery): Promise<WorkspaceQueryResponse> {
        return this.#client.query(query);
    }

    /** Watch roots through this workspace handle. */
    async watch(
        roots: readonly string[] = [this.#root],
        options: WatchOptionsInit = {},
    ): Promise<WatchStartedResponse> {
        return this.#client.startWatch(roots, watchOptions(options));
    }

    /** Receive and apply the next watch batch. */
    async nextWatch(): Promise<WatchBatchResponse> {
        return this.#client.nextWatchBatch();
    }

    /** Stop watching roots through this workspace handle. */
    async unwatch(): Promise<WatchStoppedResponse> {
        return this.#client.stopWatch();
    }

    /** Close this root handle on the workspace server. */
    async close(): Promise<void> {
        await this.#client.closeRoot();
    }
}

/** Open one remote workspace through a workspace protocol endpoint. */
export function openRemoteWorkspace(options: RemoteWorkspaceOptions): Promise<RemoteWorkspace> {
    return RemoteWorkspace.open(options);
}

/** Connect to the provided endpoint or reuse an existing connection. */
async function remoteConnection(options: RemoteWorkspaceOptions): Promise<Connection> {
    if (options.connection !== undefined) {
        return options.connection;
    }

    if (options.url !== undefined) {
        return connectEndpoint(options.url);
    }

    throw new Error("remote workspace requires either connection or url");
}

/** Return exact shared command fields from a sparse command input. */
function commandInput(input: CommandInputInit): CommandInputFields {
    return {
        revision: input.revision ?? CommandRevision.current(),
        inputs: input.inputs ?? [],
        configInputs: input.configInputs ?? true,
        cwd: input.cwd,
        manifest: input.manifest,
        target: input.target,
        targetOverrides: input.targetOverrides,
        profile: input.profile,
        env: input.env ?? [],
        overrides: input.overrides ?? [],
        watch: input.watch ?? false,
        dryRun: input.dryRun ?? false,
    };
}

/** Return an exact check input from a sparse check input. */
function checkInput(input: CheckInputInit): CheckInput {
    return {
        ...commandInput(input),
        lint: input.lint ?? true,
        fix: input.fix ?? false,
        unsafeFixes: input.unsafeFixes ?? false,
        diff: input.diff ?? false,
        trace: input.trace ?? "summary",
    };
}

/** Return an exact lint input from a sparse lint input. */
function lintInput(input: LintInputInit): LintInput {
    return {
        ...commandInput(input),
        fix: input.fix ?? false,
        unsafeFixes: input.unsafeFixes ?? false,
        diff: input.diff ?? false,
    };
}

/** Return an exact format input from a sparse format input. */
function formatInput(input: FormatInputInit): FormatInput {
    return {
        ...commandInput(input),
        source: formatSource(input.source),
        mode: input.mode ?? "preview",
    };
}

/** Return an exact format source from an idiomatic format source input. */
function formatSource(source: FormatSourceInit | undefined): FormatSource {
    if (source === undefined) {
        return FormatSource.files([]);
    }

    if (typeof source === "string") {
        return FormatSource.files([source]);
    }

    if (Array.isArray(source)) {
        return FormatSource.files(source);
    }

    if ("kind" in source) {
        return source;
    }

    if ("files" in source) {
        return FormatSource.files(source.files);
    }

    if ("openFile" in source) {
        return FormatSource.openFile(source.openFile);
    }

    if ("content" in source) {
        return FormatSource.content(source.name ?? "<content>", source.fileType ?? "destack", source.content);
    }

    throw new TypeError("unsupported format source");
}

/** Return an exact build input from a sparse build input. */
function buildInput(input: BuildInputInit): BuildInput {
    return {
        ...commandInput(input),
        trace: input.trace ?? "summary",
        product: input.product,
        outputs: input.outputs ?? buildOutputs(),
    };
}

/** Return the default build output families. */
function buildOutputs(): BuildOutputs {
    return {
        products: true,
        bundles: true,
        programs: true,
        assets: false,
    };
}

/** Return an exact run input from a sparse run input. */
function runInput(input: RunInputInit): RunInput {
    return {
        ...commandInput(input),
        entry: input.entry,
        args: input.args ?? [],
        runMode: input.runMode ?? RunMode.program(),
    };
}

/** Return an exact test input from a sparse test input. */
function testInput(input: TestInputInit): TestInput {
    return commandInput(input);
}

/** Return an exact doc input from a sparse doc input. */
function docInput(input: DocInputInit): DocInput {
    return commandInput(input);
}

/** Return an exact bench input from a sparse bench input. */
function benchInput(input: BenchInputInit): BenchInput {
    return commandInput(input);
}

/** Return an exact info input from a sparse info input. */
function infoInput(input: InfoInputInit): InfoInput {
    return {
        ...commandInput(input),
        all: input.all ?? false,
    };
}

/** Return an exact targets input from a sparse targets input. */
function targetsInput(input: TargetsInputInit): TargetsInput {
    return {
        ...commandInput(input),
        all: input.all ?? false,
    };
}

/** Return an exact cache input from a sparse cache input. */
function cacheInput(input: CacheInputInit): CacheInput {
    return commandInput(input);
}

/** Return an exact settings input from a sparse settings input. */
function settingsInput(input: SettingsInputInit): SettingsInput {
    return commandInput(input);
}

/** Return an exact doctor input from a sparse doctor input. */
function doctorInput(input: DoctorInputInit): DoctorInput {
    return {
        ...commandInput(input),
        full: input.full ?? false,
    };
}

/** Return an exact task input from a sparse task input. */
function taskInput(input: TaskInputInit): TaskInput {
    return {
        ...commandInput(input),
        action: input.action ?? TaskAction.list(),
        projects: input.projects ?? [],
        groups: input.groups ?? [],
    };
}

/** Return an exact clean input from a sparse clean input. */
function cleanInput(input: CleanInputInit): CleanInput {
    return {
        ...commandInput(input),
        dir: input.dir,
        dist: input.dist ?? false,
        cache: input.cache ?? false,
        all: input.all ?? false,
        allPackages: input.allPackages ?? false,
    };
}

/** Return exact content payload from ergonomic content input. */
function contentPayload(content: Content | string | Uint8Array | readonly number[]): Content {
    if (isContent(content)) {
        return content;
    }

    if (typeof content === "string") {
        return Content.text(content);
    }

    return Content.binary(content);
}

/** Return whether one value is already an exact content payload. */
function isContent(content: unknown): content is Content {
    if (typeof content !== "object" || content === null) {
        return false;
    }

    if (content instanceof Uint8Array || Array.isArray(content)) {
        return false;
    }

    return "kind" in content;
}

/** Return exact watch options from sparse watch options. */
function watchOptions(options: WatchOptionsInit): WatchStartOptions {
    return {
        coalesceWindowMs: options.coalesceWindowMs ?? DEFAULT_WATCH_COALESCE_WINDOW_MS,
        maxBatchSize: options.maxBatchSize ?? DEFAULT_WATCH_BATCH_SIZE,
    };
}

/** Return one response variant or throw its protocol error. */
function expectResponse<K extends WorkspaceResponse["kind"]>(
    response: WorkspaceResponse,
    kind: K,
): Extract<WorkspaceResponse, { readonly kind: K }> {
    if (response.kind === kind) {
        return response as Extract<WorkspaceResponse, { readonly kind: K }>;
    }

    if (response.kind === "error") {
        throw protocolError(response.error);
    }

    throw new Error(`expected ${kind} response, got ${response.kind}`);
}

/** Convert one protocol error into a JavaScript error. */
function protocolError(error: ProtocolError): Error {
    return new Error(`${error.code}: ${error.message}`);
}
