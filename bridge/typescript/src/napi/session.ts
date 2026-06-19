import type * as Napi from "@destack/language-napi";
import type { ArtifactKey } from "../artifact/key.generated.js";
import type { BuildOutput, BuildRequest } from "../artifact/output.generated.js";
import type { ArtifactRecord } from "../artifact/record.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { DirResolved } from "../dir/resolved.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import type { CheckOutput } from "../session/command/check.generated.js";
import type { SessionFile } from "../session/file.generated.js";
import type { FormatOutput, FormatRequest } from "../session/command/format.generated.js";
import type { LintOutput, LintRequest } from "../session/command/lint.generated.js";
import type { Module } from "../session/module.generated.js";
import type { ParseOutput } from "../session/command/parse.generated.js";
import type { Content, ContentId } from "../source/file.generated.js";
import type { PackageId } from "../source/package.generated.js";
import type { ProfileId } from "../source/profile.generated.js";
import type { TargetId } from "../source/target.generated.js";
import type { Change } from "../session/source/file.generated.js";
import type { Source } from "../session/source/source.generated.js";
import type { Commit, Edit } from "../session/source/update.generated.js";
import type { TraceReport } from "../repository/trace.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import type { Repository } from "../repository/repository.js";
import type { Session } from "../session/session.js";
import type { Workspace } from "../workspace/workspace.js";
import {
    fromNapiChange,
    fromNapiArtifactRecord,
    fromNapiArtifactSidecar,
    fromNapiArtifactVersion,
    fromNapiBuildOutput,
    fromNapiCheckOutput,
    fromNapiDiagnostic,
    fromNapiDirResolved,
    fromNapiParseOutput,
    fromNapiContent,
    fromNapiFormatOutput,
    fromNapiLintOutput,
    fromNapiModule,
    fromNapiProfileId,
    fromNapiSessionFile,
    fromNapiTargetId,
    fromNapiTraceReport,
    fromNapiCommit,
    toNapiArtifactKey,
    toNapiBuildRequest,
    toNapiContentId,
    toNapiFormatRequest,
    toNapiLintRequest,
    toNapiModule,
    toNapiPackageId,
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

/** Open one NAPI language repository from one source input. */
export async function openNapiRepository(source: Source): Promise<Repository> {
    const napi = await import("@destack/language-napi");
    const repository = napi.Repository.open(toNapiSource(source));

    return new NativeRepository(repository);
}

/** Open one NAPI language workspace from one source input. */
export async function openNapiWorkspace(source: Source): Promise<Workspace> {
    const napi = await import("@destack/language-napi");
    const workspace = napi.Workspace.open(toNapiSource(source));

    return new NativeWorkspace(workspace);
}

class NativeRepository implements Repository {
    public constructor(private readonly repository: InstanceType<NapiModule["Repository"]>) {}

    public root(): string {
        return this.repository.root();
    }

    public workspace(): Workspace {
        return new NativeWorkspace(this.repository.workspace());
    }
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

    public editIfCurrent(revision: Revision, edits: readonly Edit[]): Commit {
        const result = this.session.editIfCurrent(revision, edits.map(toNapiEdit));

        return fromNapiCommit(result);
    }

    public reload(): readonly Change[] {
        return this.session.reload().map(fromNapiChange);
    }

    public module(path: string): Module {
        return fromNapiModule(this.session.module(path));
    }

    public target(revision: Revision, packageValue: PackageId, name: string): TargetId {
        const target = this.session.target(revision, toNapiPackageId(packageValue), name);

        return fromNapiTargetId(target);
    }

    public profile(revision: Revision, module: Module, name: string): ProfileId {
        const profile = this.session.profile(revision, toNapiModule(module), name);

        return fromNapiProfileId(profile);
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

    public trace(revision: Revision, detailed: boolean): TraceReport | undefined {
        const trace = this.session.trace(revision, detailed);

        return trace == null ? undefined : fromNapiTraceReport(trace);
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

    public parse(revision: Revision, module: Module): ParseOutput {
        const output = this.session.parse(revision, toNapiModule(module));

        return fromNapiParseOutput(output);
    }

    public resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved {
        const resolved = this.session.resolve(
            revision,
            toNapiModule(module),
            toNapiProfileId(profile),
        );

        return fromNapiDirResolved(resolved);
    }

    public check(revision: Revision, module: Module, profile: ProfileId): CheckOutput {
        const output = this.session.check(
            revision,
            toNapiModule(module),
            toNapiProfileId(profile),
        );

        return fromNapiCheckOutput(output);
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

class NativeWorkspace implements Workspace {
    public constructor(private readonly workspace: InstanceType<NapiModule["Workspace"]>) {}

    public root(): string {
        return this.workspace.root();
    }

    public revision(): Revision {
        return this.workspace.revision();
    }

    public files(): readonly SessionFile[] {
        return this.workspace.files().map(fromNapiSessionFile);
    }

    public edit(edits: readonly Edit[]): Commit {
        const result = this.workspace.edit(edits.map(toNapiEdit));

        return fromNapiCommit(result);
    }

    public editIfCurrent(revision: Revision, edits: readonly Edit[]): Commit {
        const result = this.workspace.editIfCurrent(revision, edits.map(toNapiEdit));

        return fromNapiCommit(result);
    }

    public reload(): readonly Change[] {
        return this.workspace.reload().map(fromNapiChange);
    }

    public module(path: string): Module {
        return fromNapiModule(this.workspace.module(path));
    }

    public target(revision: Revision, packageValue: PackageId, name: string): TargetId {
        const target = this.workspace.target(revision, toNapiPackageId(packageValue), name);

        return fromNapiTargetId(target);
    }

    public profile(revision: Revision, module: Module, name: string): ProfileId {
        const profile = this.workspace.profile(revision, toNapiModule(module), name);

        return fromNapiProfileId(profile);
    }

    public provide(revision: Revision, keys: readonly ArtifactKey[]): void {
        this.workspace.provide(revision, keys.map(toNapiArtifactKey));
    }

    public require(revision: Revision, key: ArtifactKey): ArtifactVersion {
        const version = this.workspace.require(revision, toNapiArtifactKey(key));

        return fromNapiArtifactVersion(version);
    }

    public artifactRecord(revision: Revision, key: ArtifactKey): ArtifactRecord {
        const record = this.workspace.artifactRecord(revision, toNapiArtifactKey(key));

        return fromNapiArtifactRecord(record);
    }

    public trace(revision: Revision, detailed: boolean): TraceReport | undefined {
        const trace = this.workspace.trace(revision, detailed);

        return trace == null ? undefined : fromNapiTraceReport(trace);
    }

    public build(revision: Revision, request: BuildRequest): BuildOutput {
        const output = this.workspace.build(revision, toNapiBuildRequest(request));

        return fromNapiBuildOutput(output);
    }

    public content(id: ContentId): Content {
        const content = this.workspace.content(toNapiContentId(id));

        return fromNapiContent(content);
    }

    public text(id: ContentId): string {
        return this.workspace.text(toNapiContentId(id));
    }

    public bytes(id: ContentId): Uint8Array {
        return Uint8Array.from(this.workspace.bytes(toNapiContentId(id)));
    }

    public parse(revision: Revision, module: Module): ParseOutput {
        const output = this.workspace.parse(revision, toNapiModule(module));

        return fromNapiParseOutput(output);
    }

    public resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved {
        const resolved = this.workspace.resolve(
            revision,
            toNapiModule(module),
            toNapiProfileId(profile),
        );

        return fromNapiDirResolved(resolved);
    }

    public check(revision: Revision, module: Module, profile: ProfileId): CheckOutput {
        const output = this.workspace.check(
            revision,
            toNapiModule(module),
            toNapiProfileId(profile),
        );

        return fromNapiCheckOutput(output);
    }

    public format(revision: Revision, request: FormatRequest): FormatOutput {
        const output = this.workspace.format(revision, toNapiFormatRequest(request));

        return fromNapiFormatOutput(output);
    }

    public lint(revision: Revision, request: LintRequest): LintOutput {
        const output = this.workspace.lint(revision, toNapiLintRequest(request));

        return fromNapiLintOutput(output);
    }

    public diagnostics(revision: Revision, key?: ArtifactKey): readonly Diagnostic[] {
        const diagnostics = this.workspace.diagnostics(
            revision,
            key == null ? undefined : toNapiArtifactKey(key),
        );

        return diagnostics.map(fromNapiDiagnostic);
    }

    public sidecars(revision: Revision, key: ArtifactKey): readonly ArtifactSidecar[] {
        const sidecars = this.workspace.sidecars(revision, toNapiArtifactKey(key));

        return sidecars.map(fromNapiArtifactSidecar);
    }
}
