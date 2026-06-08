import type * as Wasm from "@destack/language-wasm";
import type { ArtifactKey } from "../artifact/key.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import type { SessionFile } from "../session/file.generated.js";
import type { Module } from "../session/module.generated.js";
import type { FileUpdate } from "../session/source/file.generated.js";
import type { SourceSnapshot } from "../session/source/snapshot.generated.js";
import type { SourceUpdate, SourceUpdateResult } from "../session/source/update.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import type { Session } from "../session/session.js";
import {
    fromWasmFileUpdate,
    fromWasmArtifactSidecar,
    fromWasmDiagnostic,
    fromWasmModule,
    fromWasmSessionFile,
    fromWasmSourceUpdateResult,
    toWasmArtifactKey,
    toWasmRevision,
    toWasmSourceSnapshot,
    toWasmSourceUpdate,
} from "./generated.js";

type WasmModule = typeof import("@destack/language-wasm");

/** Open one WASM language session from an explicit source snapshot. */
export async function openWasmSource(root: string, source: SourceSnapshot): Promise<Session> {
    const wasm = await import("@destack/language-wasm");
    await wasm.default();
    const session = wasm.Session.openSource(root, toWasmSourceSnapshot(wasm, source));

    return new WasmSession(wasm, session);
}

class WasmSession implements Session {
    public constructor(
        private readonly wasm: WasmModule,
        private readonly session: Wasm.Session,
    ) {}

    public revision(): Revision {
        return this.session.revision();
    }

    public files(): readonly SessionFile[] {
        return this.session.files().map(fromWasmSessionFile);
    }

    public update(update: SourceUpdate): SourceUpdateResult {
        const result = this.session.update(toWasmSourceUpdate(this.wasm, update));

        return fromWasmSourceUpdateResult(result);
    }

    public reload(): readonly FileUpdate[] {
        return this.session.reload().map(fromWasmFileUpdate);
    }

    public loadModule(path: string): Module {
        return fromWasmModule(this.session.loadModule(path));
    }

    public provide(revision: Revision, keys: readonly ArtifactKey[]): void {
        const transportKeys = keys.map((key) => toWasmArtifactKey(this.wasm, key));

        this.session.provide(toWasmRevision(this.wasm, revision), transportKeys);
    }

    public require(revision: Revision, key: ArtifactKey): ArtifactVersion {
        const version = this.session.require(
            toWasmRevision(this.wasm, revision),
            toWasmArtifactKey(this.wasm, key),
        );

        return {
            key,
            fingerprint: version.fingerprint,
        };
    }

    public diagnostics(revision: Revision, key?: ArtifactKey): readonly Diagnostic[] {
        const diagnostics = this.session.diagnostics(
            toWasmRevision(this.wasm, revision),
            key == null ? undefined : toWasmArtifactKey(this.wasm, key),
        );

        return diagnostics.map(fromWasmDiagnostic);
    }

    public sidecars(revision: Revision, key: ArtifactKey): readonly ArtifactSidecar[] {
        const sidecars = this.session.sidecars(
            toWasmRevision(this.wasm, revision),
            toWasmArtifactKey(this.wasm, key),
        );

        return sidecars.map(fromWasmArtifactSidecar);
    }
}
