import type * as Wasm from "@destack/language-wasm";
import type { ArtifactKey } from "../artifact/key.generated.js";
import type { ArtifactRecord } from "../artifact/record.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { DirChecked } from "../dir/checked.generated.js";
import type { DirParsed } from "../dir/parsed.generated.js";
import type { DirResolved } from "../dir/resolved.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import type { SessionFile } from "../session/file.generated.js";
import type { Module } from "../session/module.generated.js";
import type { ProfileId } from "../source/profile.generated.js";
import type { FileChange } from "../session/source/file.generated.js";
import type { Source } from "../session/source/source.generated.js";
import type { FileUpdate, FileUpdateResult } from "../session/source/update.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import type { Session } from "../session/session.js";
import {
    fromWasmFileChange,
    fromWasmArtifactRecord,
    fromWasmArtifactSidecar,
    fromWasmDiagnostic,
    fromWasmDirChecked,
    fromWasmDirParsed,
    fromWasmDirResolved,
    fromWasmModule,
    fromWasmSessionFile,
    fromWasmFileUpdateResult,
    toWasmArtifactKey,
    toWasmModule,
    toWasmProfileId,
    toWasmRevision,
    toWasmSource,
    toWasmFileUpdate,
} from "./generated.js";

type WasmModule = typeof import("@destack/language-wasm");

/** Open one WASM language session from one source input. */
export async function openWasmSession(source: Source): Promise<Session> {
    const wasm = await import("@destack/language-wasm");
    await wasm.default();
    const session = wasm.Session.open(toWasmSource(wasm, source));

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

    public update(update: FileUpdate): FileUpdateResult {
        const result = this.session.update(toWasmFileUpdate(this.wasm, update));

        return fromWasmFileUpdateResult(result);
    }

    public reload(): readonly FileChange[] {
        return this.session.reload().map(fromWasmFileChange);
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

    public artifactRecord(revision: Revision, key: ArtifactKey): ArtifactRecord {
        const record = this.session.artifactRecord(
            toWasmRevision(this.wasm, revision),
            toWasmArtifactKey(this.wasm, key),
        );

        return fromWasmArtifactRecord(record);
    }

    public parse(revision: Revision, module: Module): DirParsed {
        const parsed = this.session.parse(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
        );

        return fromWasmDirParsed(parsed);
    }

    public resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved {
        const resolved = this.session.resolve(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            toWasmProfileId(this.wasm, profile),
        );

        return fromWasmDirResolved(resolved);
    }

    public check(revision: Revision, module: Module, profile: ProfileId): DirChecked {
        const checked = this.session.check(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            toWasmProfileId(this.wasm, profile),
        );

        return fromWasmDirChecked(checked);
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
