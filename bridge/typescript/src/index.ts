import type * as Wasm from "@destack/language-wasm";
import type {
    FileUpdate,
    Revision,
    SourceEdit,
    SourceFile,
    SourceSnapshot,
    SourceUpdate,
    SourceUpdateResult,
    TextEdit,
    TextRange,
} from "./language.generated.js";

/** Supported language backend kinds. */
export type LanguageBackend = "napi" | "wasm";

export type {
    FileUpdate,
    Revision,
    SourceEdit,
    SourceFile,
    SourceSnapshot,
    SourceUpdate,
    SourceUpdateResult,
    TextEdit,
    TextRange,
} from "./language.generated.js";

/** Open input for one language session. */
export type OpenSessionInput =
    | {
          /** Native filesystem path. */
          readonly path: string;
          /** Backend override. */
          readonly backend?: LanguageBackend;
      }
    | {
          /** Source root path used for logical file identity. */
          readonly root: string;
          /** Complete source snapshot. */
          readonly source: SourceSnapshot;
          /** Backend override. */
          readonly backend?: LanguageBackend;
      };

/** A live language session. */
export interface LanguageSession {
    /** Selected backend. */
    readonly backend: LanguageBackend;
    /** Return the current session revision. */
    revision(): Revision;
    /** Return editable repository file paths. */
    files(): readonly string[];
    /** Apply one source update. */
    update(update: SourceUpdate): SourceUpdateResult;
    /** Reload tracked files from the backend source. */
    reload(): readonly FileUpdate[];
    /** Load one module path into the current session. */
    loadModule(path: string): string;
}

type NapiModule = typeof import("@destack/language-napi");
type WasmModule = typeof import("@destack/language-wasm");

/** Detect the default backend for the current runtime. */
export function detectBackend(): LanguageBackend {
    const processValue = (globalThis as { process?: { versions?: { node?: string } } }).process;

    return processValue?.versions?.node == null ? "wasm" : "napi";
}

/** Open one language session. */
export async function openSession(input: OpenSessionInput): Promise<LanguageSession> {
    const backend = input.backend ?? detectBackend();

    // native path sessions require node api filesystem access
    if ("path" in input) {
        if (backend !== "napi") {
            throw new Error("openSession({ path }) requires the napi backend");
        }

        return openNapiPath(input.path);
    }

    // source sessions work in both backends
    if (backend === "napi") {
        return openNapiSource(input.root, input.source);
    }

    return openWasmSource(input.root, input.source);
}

/** Open one NAPI language session from a native filesystem path. */
export async function openNapiPath(path: string): Promise<LanguageSession> {
    const napi = await import("@destack/language-napi");
    const session = napi.LanguageSession.openPath(path);

    return new NapiLanguageSession(session);
}

/** Open one NAPI language session from an explicit source snapshot. */
export async function openNapiSource(root: string, source: SourceSnapshot): Promise<LanguageSession> {
    const napi = await import("@destack/language-napi");
    const session = napi.LanguageSession.openSource(root, source);

    return new NapiLanguageSession(session);
}

/** Open one WASM language session from an explicit source snapshot. */
export async function openWasmSource(root: string, source: SourceSnapshot): Promise<LanguageSession> {
    const wasm = await import("@destack/language-wasm");
    await wasm.default();
    const session = wasm.LanguageSession.openSource(root, lowerWasmSourceSnapshot(wasm, source));

    return new WasmLanguageSession(wasm, session);
}

class NapiLanguageSession implements LanguageSession {
    public readonly backend = "napi";

    public constructor(private readonly session: InstanceType<NapiModule["LanguageSession"]>) {}

    public revision(): Revision {
        return this.session.revision();
    }

    public files(): readonly string[] {
        return this.session.files();
    }

    public update(update: SourceUpdate): SourceUpdateResult {
        return this.session.update(update);
    }

    public reload(): readonly FileUpdate[] {
        return this.session.reload();
    }

    public loadModule(path: string): string {
        return this.session.loadModule(path);
    }
}

class WasmLanguageSession implements LanguageSession {
    public readonly backend = "wasm";

    public constructor(
        private readonly wasm: WasmModule,
        private readonly session: Wasm.LanguageSession,
    ) {}

    public revision(): Revision {
        return this.session.revision();
    }

    public files(): readonly string[] {
        return this.session.files().map(String);
    }

    public update(update: SourceUpdate): SourceUpdateResult {
        const result = this.session.update(lowerWasmSourceUpdate(this.wasm, update));

        return liftWasmSourceUpdateResult(result);
    }

    public reload(): readonly FileUpdate[] {
        return this.session.reload().map(liftWasmFileUpdate);
    }

    public loadModule(path: string): string {
        return this.session.loadModule(path);
    }
}

function lowerWasmSourceSnapshot(wasm: WasmModule, source: SourceSnapshot): Wasm.SourceSnapshot {
    const snapshot = new wasm.SourceSnapshot();

    // append source files in caller order
    for (const file of source.files) {
        if (file.text != null) {
            snapshot.addFile(wasm.SourceFile.text(file.path, file.text));
        } else if (file.bytes != null) {
            snapshot.addFile(wasm.SourceFile.bytes(file.path, Uint8Array.from(file.bytes)));
        } else {
            throw new Error(`source file is missing content: ${file.path}`);
        }
    }

    return snapshot;
}

function lowerWasmSourceUpdate(wasm: WasmModule, update: SourceUpdate): Wasm.SourceUpdate {
    const lowered = new wasm.SourceUpdate();

    // set compare and swap base when requested
    if (update.base != null) {
        lowered.setBase(new wasm.Revision(update.base.id));
    }

    // append source edits in caller order
    for (const edit of update.edits) {
        lowered.addEdit(lowerWasmSourceEdit(wasm, edit));
    }

    return lowered;
}

function lowerWasmSourceEdit(wasm: WasmModule, edit: SourceEdit): Wasm.SourceEdit {
    if (edit.kind === "setText" && edit.setText != null) {
        return wasm.SourceEdit.setText(edit.setText.path, edit.setText.text);
    }

    if (edit.kind === "editText" && edit.editText != null) {
        const lowered = wasm.SourceEdit.editText(edit.editText.path);
        for (const textEdit of edit.editText.edits) {
            const range = new wasm.TextRange(textEdit.range.start, textEdit.range.end);
            lowered.addTextEdit(new wasm.TextEdit(range, textEdit.text));
        }

        return lowered;
    }

    if (edit.kind === "setBytes" && edit.setBytes != null) {
        return wasm.SourceEdit.setBytes(edit.setBytes.path, Uint8Array.from(edit.setBytes.bytes));
    }

    if (edit.kind === "remove" && edit.remove != null) {
        return wasm.SourceEdit.remove(edit.remove.path);
    }

    if (edit.kind === "move" && edit.moveFile != null) {
        return wasm.SourceEdit.moveFile(edit.moveFile.from, edit.moveFile.to);
    }

    throw new Error(`source edit is missing payload: ${edit.kind}`);
}

function liftWasmSourceUpdateResult(result: Wasm.SourceUpdateResult): SourceUpdateResult {
    return {
        before: { id: result.before.id },
        after: { id: result.after.id },
        files: result.files.map(liftWasmFileUpdate),
    };
}

function liftWasmFileUpdate(update: Wasm.FileUpdate): FileUpdate {
    return {
        path: update.path,
        uri: update.uri,
        kind: update.kind,
        isRemoved: update.isRemoved,
        moduleId: update.moduleId,
    };
}
