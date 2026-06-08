import type * as Napi from "@destack/language-napi";
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
    fromNapiFileUpdate,
    fromNapiArtifactSidecar,
    fromNapiArtifactVersion,
    fromNapiDiagnostic,
    fromNapiModule,
    fromNapiSessionFile,
    fromNapiSourceUpdateResult,
    toNapiArtifactKey,
    toNapiSourceSnapshot,
    toNapiSourceUpdate,
} from "./generated.js";

type NapiModule = typeof import("@destack/language-napi");

/** Open one NAPI language session from a native filesystem path. */
export async function openNapiPath(path: string): Promise<Session> {
    const napi = await import("@destack/language-napi");
    const session = napi.Session.openPath(path);

    return new NativeSession(session);
}

/** Open one NAPI language session from an explicit source snapshot. */
export async function openNapiSource(root: string, source: SourceSnapshot): Promise<Session> {
    const napi = await import("@destack/language-napi");
    const session = napi.Session.openSource(root, toNapiSourceSnapshot(source));

    return new NativeSession(session);
}

class NativeSession implements Session {
    public constructor(private readonly session: InstanceType<NapiModule["Session"]>) {}

    public revision(): Revision {
        return this.session.revision();
    }

    public files(): readonly SessionFile[] {
        return this.session.files().map(fromNapiSessionFile);
    }

    public update(update: SourceUpdate): SourceUpdateResult {
        const result = this.session.update(toNapiSourceUpdate(update));

        return fromNapiSourceUpdateResult(result);
    }

    public reload(): readonly FileUpdate[] {
        return this.session.reload().map(fromNapiFileUpdate);
    }

    public loadModule(path: string): Module {
        return fromNapiModule(this.session.loadModule(path));
    }

    public provide(revision: Revision, keys: readonly ArtifactKey[]): void {
        this.session.provide(revision, keys.map(toNapiArtifactKey));
    }

    public require(revision: Revision, key: ArtifactKey): ArtifactVersion {
        const version = this.session.require(revision, toNapiArtifactKey(key));

        return fromNapiArtifactVersion(version);
    }

    public diagnostics(revision: Revision, key?: ArtifactKey): readonly Diagnostic[] {
        const diagnostics = this.session.diagnostics(
            revision,
            key == null ? undefined : toNapiArtifactKey(key),
        );

        return diagnostics.map(fromNapiDiagnostic);
    }

    public sidecars(revision: Revision, key: ArtifactKey): readonly ArtifactSidecar[] {
        const sidecars = this.session.sidecars(revision, toNapiArtifactKey(key));

        return sidecars.map(fromNapiArtifactSidecar);
    }
}
