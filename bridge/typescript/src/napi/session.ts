import type * as Napi from "@destack/language-napi";
import type { ArtifactKey } from "../artifact/key.generated.js";
import type { BuildOutput, BuildRequest } from "../artifact/output.generated.js";
import type { ArtifactRecord } from "../artifact/record.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { DirChecked } from "../dir/checked.generated.js";
import type { DirParsed } from "../dir/parsed.generated.js";
import type { DirResolved } from "../dir/resolved.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import type { SessionFile } from "../session/file.generated.js";
import type { FormatOutput, FormatRequest } from "../session/format.generated.js";
import type { LintOutput, LintRequest } from "../session/lint.generated.js";
import type { Module } from "../session/module.generated.js";
import type { Content, ContentId } from "../source/file.generated.js";
import type { ProfileId } from "../source/profile.generated.js";
import type { Change } from "../session/source/file.generated.js";
import type { Source } from "../session/source/source.generated.js";
import type { Commit, Edit } from "../session/source/update.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import type { Session } from "../session/session.js";
import {
    fromNapiChange,
    fromNapiArtifactRecord,
    fromNapiArtifactSidecar,
    fromNapiArtifactVersion,
    fromNapiBuildOutput,
    fromNapiDiagnostic,
    fromNapiDirChecked,
    fromNapiDirParsed,
    fromNapiDirResolved,
    fromNapiContent,
    fromNapiFormatOutput,
    fromNapiLintOutput,
    fromNapiModule,
    fromNapiSessionFile,
    fromNapiCommit,
    toNapiArtifactKey,
    toNapiBuildRequest,
    toNapiContentId,
    toNapiFormatRequest,
    toNapiLintRequest,
    toNapiModule,
    toNapiProfileId,
    toNapiSource,
    toNapiEdit,
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

    public edit(edits: readonly Edit[]): Commit {
        const result = this.session.edit(edits.map(toNapiEdit));

        return fromNapiCommit(result);
    }

    public editAt(revision: Revision, edits: readonly Edit[]): Commit {
        const result = this.session.editAt(revision, edits.map(toNapiEdit));

        return fromNapiCommit(result);
    }

    public reload(): readonly Change[] {
        return this.session.reload().map(fromNapiChange);
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

    public build(revision: Revision, request: BuildRequest): BuildOutput {
        const output = this.session.build(revision, toNapiBuildRequest(request));

        return fromNapiBuildOutput(output);
    }

    public content(id: ContentId): Content {
        const content = this.session.content(toNapiContentId(id));

        return fromNapiContent(content);
    }

    public text(id: ContentId): string {
        return this.session.text(toNapiContentId(id));
    }

    public bytes(id: ContentId): Uint8Array {
        return Uint8Array.from(this.session.bytes(toNapiContentId(id)));
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

    public format(revision: Revision, request: FormatRequest): FormatOutput {
        const output = this.session.format(revision, toNapiFormatRequest(request));

        return fromNapiFormatOutput(output);
    }

    public lint(revision: Revision, request: LintRequest): LintOutput {
        const output = this.session.lint(revision, toNapiLintRequest(request));

        return fromNapiLintOutput(output);
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
