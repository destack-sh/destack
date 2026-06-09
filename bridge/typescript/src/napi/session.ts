import type * as Napi from "@destack/language-napi";
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
    fromNapiFileChange,
    fromNapiArtifactRecord,
    fromNapiArtifactSidecar,
    fromNapiArtifactVersion,
    fromNapiDiagnostic,
    fromNapiDirChecked,
    fromNapiDirParsed,
    fromNapiDirResolved,
    fromNapiModule,
    fromNapiSessionFile,
    fromNapiFileUpdateResult,
    toNapiArtifactKey,
    toNapiModule,
    toNapiProfileId,
    toNapiSource,
    toNapiFileUpdate,
} from "./generated.js";

type NapiModule = typeof import("@destack/language-napi");

/** Open one NAPI language session from one source input. */
export async function openNapiSession(source: Source): Promise<Session> {
    const napi = await import("@destack/language-napi");
    const session = napi.Session.open(toNapiSource(source));

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

    public update(update: FileUpdate): FileUpdateResult {
        const result = this.session.update(toNapiFileUpdate(update));

        return fromNapiFileUpdateResult(result);
    }

    public reload(): readonly FileChange[] {
        return this.session.reload().map(fromNapiFileChange);
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

    public artifactRecord(revision: Revision, key: ArtifactKey): ArtifactRecord {
        const record = this.session.artifactRecord(revision, toNapiArtifactKey(key));

        return fromNapiArtifactRecord(record);
    }

    public parse(revision: Revision, module: Module): DirParsed {
        const parsed = this.session.parse(revision, toNapiModule(module));

        return fromNapiDirParsed(parsed);
    }

    public resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved {
        const resolved = this.session.resolve(
            revision,
            toNapiModule(module),
            toNapiProfileId(profile),
        );

        return fromNapiDirResolved(resolved);
    }

    public check(revision: Revision, module: Module, profile: ProfileId): DirChecked {
        const checked = this.session.check(
            revision,
            toNapiModule(module),
            toNapiProfileId(profile),
        );

        return fromNapiDirChecked(checked);
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
