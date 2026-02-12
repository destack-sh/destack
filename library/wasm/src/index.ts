export type DiagnosticSeverity = "note" | "warning" | "error";

export interface Span {
    fileId: number;
    start: number;
    end: number;
}

export interface LabeledSpan {
    span: Span;
    label: string;
}

export interface Suggestion {
    spans: LabeledSpan[];
    replacement?: string;
    message: string;
    style: "normal" | "short" | "hidden" | "verbose";
    applicability: "automatic" | "dangerous";
}

export interface Diagnostic {
    code: string;
    originalCode?: string;
    severity: DiagnosticSeverity;
    originalSeverity?: DiagnosticSeverity;
    message: string;
    fileId: number;
    primarySpan: LabeledSpan;
    primaryHighlightSpans?: LabeledSpan[];
    secondarySpans?: LabeledSpan[];
    suggestions?: Suggestion[];
}

export interface DiagnosticOptions {
    errorWarnings: string[];
    suppressErrors: string[];
    suppressWarnings: string[];
}

export type FileType =
    | "destack"
    | "destackDeclaration"
    | "destackText"
    | "destackBinary"
    | "javaScript"
    | "javaScriptXml"
    | "typeScript"
    | "typeScriptXml"
    | "typeScriptDeclaration"
    | "text"
    | "toml"
    | "yaml"
    | "json"
    | "env"
    | "html"
    | "markdown"
    | "css"
    | "svg"
    | "wasm"
    | "node"
    | "sourceMap"
    | "object"
    | "destackAst"
    | "destackDir"
    | "destackMir"
    | "image"
    | "font"
    | "audio"
    | "video"
    | "model"
    | "neural"
    | "document"
    | "binary"
    | "unknown";

export type ModuleType = "code" | "data" | "text" | "binary";

export type ModuleFormat =
    | "commonJs"
    | "amd"
    | "umd"
    | "system"
    | "es2015"
    | "es2020"
    | "es2022"
    | "esNext"
    | "node16"
    | "nodeNext"
    | "preserve"
    | "none";

export type CompilerResolveMode = "strict" | "lenient";

export interface TypeScriptOptions {
    discovery: "disabled" | "automatic";
    configFile?: string;
    references: "disabled" | "automatic" | "manual";
    referencePaths: string[];
}

export interface AliasValue {
    path?: string;
}

export interface AliasEntry {
    pattern: string;
    targets: AliasValue[];
}

export interface ExtensionAliasEntry {
    extension: string;
    aliases: string[];
}

export interface ResolveOptions {
    cwd?: string;
    tsconfig: TypeScriptOptions;
    alias: AliasEntry[];
    conditions: string[];
    enforceExtension: "enabled" | "disabled";
    extensions: string[];
    isFullySpecified: boolean;
    fallback: AliasEntry[];
    extensionAlias: ExtensionAliasEntry[];
    mainFiles: string[];
    modules: string[];
    resolveToContext: boolean;
    preferRelative: boolean;
    preferAbsolute: boolean;
    restrictions: string[];
    roots: string[];
    canonicalizeSymlinks: boolean;
}

export interface CompilerOptions {
    diagnostic: DiagnosticOptions;
    workers: number;
    followImports: boolean;
    importResolve: ResolveOptions;
    resolveMode: CompilerResolveMode;
    disallowAmbiguousTreeLiteral: boolean;
    defaultIntWidth: number;
    defaultFloatWidth: number;
    injectPrelude: boolean;
    loadLibs: boolean;
    sourceMap: boolean;
    elaborateWithTernary: boolean;
    elaborateSplitDeclarators: boolean;
    elaborateExplicitReturn: boolean;
    elaborateParenthesizeCasts: boolean;
    retainComptimeAsComment: boolean;
    retainComptimeCommentMaxLength: number;
    emitOverwrite: boolean;
    emitCreateDirs: boolean;
    emitDryRun: boolean;
    timings: boolean;
    validateBuiltinLibs: boolean;
    verifyMir: boolean;
}

export interface Resolution {
    path: string;
    query?: string;
    fragment?: string;
}

export interface FormatOptions {
    lineEnding: "lineFeed" | "carriageReturnLineFeed" | "carriageReturn";
    indentStyle: "tab" | "space";
    indentWidth: number;
    lineWidth: number;
}

export interface FormatResult {
    code: string;
}

export interface CheckOptions {
    cwd: string;
    roots: string[];
    compiler: CompilerOptions;
}

export interface CheckResult {
    diagnostics: Diagnostic[];
    diagnosticCount: number;
    hasErrors: boolean;
    hasWarnings: boolean;
}

export interface Program {
    path: string;
    astJson: string;
    rootCount: number;
    tokenCount: number;
    sideTokenCount: number;
}

export interface ParseOptions {
    cwd: string;
    roots: string[];
    compiler: CompilerOptions;
}

export interface ParseResult {
    program: Program;
    diagnostics: Diagnostic[];
    diagnosticCount: number;
    hasErrors: boolean;
}

export type TransformTarget = "typeScript" | "javaScript";

export interface TransformOptions {
    cwd: string;
    roots: string[];
    compiler: CompilerOptions;
    target: TransformTarget;
}

export interface TransformResult {
    code: string;
}

export interface WorkspaceServiceOptions {
    cwd: string;
    roots: string[];
    compiler?: CompilerOptions;
}

export interface WorkspaceVirtualUpdate {
    kind: "text" | "bytes" | "touch" | "removed";
    content?: string;
    bytes?: number[];
}

export interface WorkspaceWatchEvent {
    path: string;
    previousPath?: string;
    kind: "created" | "modified" | "deleted" | "renamed" | "overflow";
}

export type WorkspaceServiceRescanReason =
    | "startup"
    | "overflow"
    | "manual"
    | "update";

export interface WorkspaceModuleId {
    packageId: string;
    localId: number;
}

export interface WorkspaceInvalidation {
    fileId: number;
    fileVersion: string;
    kinds: Array<"moduleSource" | "dsConfig" | "tsConfig" | "unknown">;
    modules: WorkspaceModuleId[];
    packages: string[];
    profiles: number[];
    graphsDropped: number[];
}

export interface WorkspaceFileSnapshot {
    id: number;
    name: string;
    uri: string;
    path?: string;
    fileType: FileType;
    content?: string;
}

export interface WorkspaceUpdateRecord {
    moduleId?: WorkspaceModuleId;
    fileId: number;
    file: WorkspaceFileSnapshot;
    invalidation: WorkspaceInvalidation;
    diagnostics: Diagnostic[];
}

export interface WorkspaceMessage {
    kind: "info" | "warning" | "error";
    code: string;
    message: string;
}

export interface WorkspaceServiceResult {
    updates: WorkspaceUpdateRecord[];
    messages: WorkspaceMessage[];
}

export interface WorkspaceAnalyzeOutcome {
    semanticQueryReady: boolean;
    detail?: string;
}

export interface WasmCapabilities {
    profile: string;
    query: boolean;
    lint: boolean;
    optimize: boolean;
    formatApi: boolean;
    transformApi: boolean;
    parallel: boolean;
    nativeCodegen: boolean;
    deadlockDetection: boolean;
    builtinFull: boolean;
    builtinWasmCore: boolean;
}

type GeneratedModule = {
    default(input?: unknown): Promise<void>;
    capabilities(): WasmCapabilities;
    defaultCompilerOptions(): CompilerOptions;
    defaultResolveOptions(): ResolveOptions;
    resolveSync(specifier: string, from: string, options?: ResolveOptions): Resolution;
    defaultFormatOptions(): FormatOptions;
    formatSync(path: string, content: string, options?: FormatOptions): FormatResult;
    defaultCheckOptions(): CheckOptions;
    checkSync(path: string, content: string, options?: CheckOptions): CheckResult;
    defaultParseOptions(): ParseOptions;
    parseSync(path: string, content: string, options?: ParseOptions): ParseResult;
    defaultTransformOptions(): TransformOptions;
    transformSync(path: string, content: string, options?: TransformOptions): TransformResult;
    defaultWorkspaceServiceOptions(): WorkspaceServiceOptions;
    WorkspaceService: {
        new (options?: WorkspaceServiceOptions): GeneratedWorkspaceService;
    };
};

type GeneratedWorkspaceService = {
    readonly cwd: string;
    readonly programHandleCount: number;
    openWorkspaceRoot(root: string): void;
    closeWorkspaceRoot(root: string): void;
    hasWorkspaceRoot(root: string): boolean;
    removeWorkspaceRoot(root: string): boolean;
    clearCacheAll(): void;
    shutdown(): void;
    handleForRoot(root: string): string;
    updateVirtualFile(path: string, content: string): WorkspaceServiceResult;
    applyVirtualUpdate(path: string, update: WorkspaceVirtualUpdate): WorkspaceServiceResult;
    applyWatchEvents(events: WorkspaceWatchEvent[]): WorkspaceServiceResult;
    rescanAll(reason: WorkspaceServiceRescanReason): WorkspaceServiceResult;
    rescanRoots(roots: string[], analyze: boolean): WorkspaceServiceResult;
    analyzePath(path: string): WorkspaceAnalyzeOutcome;
    ensureAnalyzedForPath(path: string): void;
    queryForPath(path: string, request: unknown): unknown;
    queryForHandle(handle: string, request: unknown): unknown;
};

let generated: GeneratedModule | null = null;
const generatedModulePath = "./generated/index.js";

async function loadGenerated(): Promise<GeneratedModule> {
    if (generated != null) {
        return generated;
    }

    const imported = (await import(
        /* @vite-ignore */ generatedModulePath
    )) as unknown as GeneratedModule;
    generated = imported;
    return imported;
}

function requireGenerated(): GeneratedModule {
    if (generated == null) {
        throw new Error("@destack-sh/wasm is not initialized: call init() first");
    }

    return generated;
}

export async function init(input?: unknown): Promise<void> {
    const module = await loadGenerated();
    await module.default(input);
}

export function capabilities(): WasmCapabilities {
    return requireGenerated().capabilities();
}

export function defaultCompilerOptions(): CompilerOptions {
    return requireGenerated().defaultCompilerOptions();
}

export function defaultResolveOptions(): ResolveOptions {
    return requireGenerated().defaultResolveOptions();
}

export function resolveSync(
    specifier: string,
    from: string,
    options?: ResolveOptions,
): Resolution {
    return requireGenerated().resolveSync(specifier, from, options);
}

export function defaultFormatOptions(): FormatOptions {
    return requireGenerated().defaultFormatOptions();
}

export function formatSync(
    path: string,
    content: string,
    options?: FormatOptions,
): FormatResult {
    return requireGenerated().formatSync(path, content, options);
}

export function defaultCheckOptions(): CheckOptions {
    return requireGenerated().defaultCheckOptions();
}

export function checkSync(
    path: string,
    content: string,
    options?: CheckOptions,
): CheckResult {
    return requireGenerated().checkSync(path, content, options);
}

export function defaultParseOptions(): ParseOptions {
    return requireGenerated().defaultParseOptions();
}

export function parseSync(
    path: string,
    content: string,
    options?: ParseOptions,
): ParseResult {
    return requireGenerated().parseSync(path, content, options);
}

export function defaultTransformOptions(): TransformOptions {
    return requireGenerated().defaultTransformOptions();
}

export function transformSync(
    path: string,
    content: string,
    options?: TransformOptions,
): TransformResult {
    return requireGenerated().transformSync(path, content, options);
}

export function defaultWorkspaceServiceOptions(): WorkspaceServiceOptions {
    return requireGenerated().defaultWorkspaceServiceOptions();
}

export class WorkspaceService {
    private readonly inner: GeneratedWorkspaceService;

    public constructor(options?: WorkspaceServiceOptions) {
        this.inner = new (requireGenerated().WorkspaceService)(options);
    }

    public get cwd(): string {
        return this.inner.cwd;
    }

    public get programHandleCount(): number {
        return this.inner.programHandleCount;
    }

    public openWorkspaceRoot(root: string): void {
        this.inner.openWorkspaceRoot(root);
    }

    public closeWorkspaceRoot(root: string): void {
        this.inner.closeWorkspaceRoot(root);
    }

    public hasWorkspaceRoot(root: string): boolean {
        return this.inner.hasWorkspaceRoot(root);
    }

    public removeWorkspaceRoot(root: string): boolean {
        return this.inner.removeWorkspaceRoot(root);
    }

    public clearCacheAll(): void {
        this.inner.clearCacheAll();
    }

    public shutdown(): void {
        this.inner.shutdown();
    }

    public handleForRoot(root: string): string {
        return this.inner.handleForRoot(root);
    }

    public updateVirtualFile(path: string, content: string): WorkspaceServiceResult {
        return this.inner.updateVirtualFile(path, content);
    }

    public applyVirtualUpdate(path: string, update: WorkspaceVirtualUpdate): WorkspaceServiceResult {
        return this.inner.applyVirtualUpdate(path, update);
    }

    public applyWatchEvents(events: WorkspaceWatchEvent[]): WorkspaceServiceResult {
        return this.inner.applyWatchEvents(events);
    }

    public rescanAll(reason: WorkspaceServiceRescanReason): WorkspaceServiceResult {
        return this.inner.rescanAll(reason);
    }

    public rescanRoots(roots: string[], analyze: boolean): WorkspaceServiceResult {
        return this.inner.rescanRoots(roots, analyze);
    }

    public analyzePath(path: string): WorkspaceAnalyzeOutcome {
        return this.inner.analyzePath(path);
    }

    public ensureAnalyzedForPath(path: string): void {
        this.inner.ensureAnalyzedForPath(path);
    }

    public queryForPath(path: string, request: unknown): unknown {
        return this.inner.queryForPath(path, request);
    }

    public queryForHandle(handle: string, request: unknown): unknown {
        return this.inner.queryForHandle(handle, request);
    }
}
