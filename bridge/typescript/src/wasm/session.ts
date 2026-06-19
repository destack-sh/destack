import type * as Wasm from "@destack/language-wasm";
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
    fromWasmChange,
    fromWasmArtifactRecord,
    fromWasmArtifactSidecar,
    fromWasmBuildOutput,
    fromWasmCheckOutput,
    fromWasmContent,
    fromWasmDiagnostic,
    fromWasmDirResolved,
    fromWasmParseOutput,
    fromWasmFormatOutput,
    fromWasmLintOutput,
    fromWasmModule,
    fromWasmProfileId,
    fromWasmSessionFile,
    fromWasmTargetId,
    fromWasmTraceReport,
    fromWasmCommit,
    toWasmArtifactKey,
    toWasmBuildRequest,
    toWasmContentId,
    toWasmFormatRequest,
    toWasmLintRequest,
    toWasmModule,
    toWasmPackageId,
    toWasmProfileId,
    toWasmRevision,
    toWasmSource,
    toWasmEdit,
} from "./generated.js";

type WasmModule = typeof import("@destack/language-wasm");

/** Open one WASM language session from one source input. */
export async function openWasmSession(source: Source): Promise<Session> {
    const wasm = await import("@destack/language-wasm");
    await wasm.default();
    const session = wasm.Session.open(toWasmSource(wasm, source));

    return new WasmSession(wasm, session);
}

/** Open one WASM language repository from one source input. */
export async function openWasmRepository(source: Source): Promise<Repository> {
    const wasm = await import("@destack/language-wasm");
    await wasm.default();
    const repository = wasm.Repository.open(toWasmSource(wasm, source));

    return new WasmRepository(wasm, repository);
}

/** Open one WASM language workspace from one source input. */
export async function openWasmWorkspace(source: Source): Promise<Workspace> {
    const wasm = await import("@destack/language-wasm");
    await wasm.default();
    const workspace = wasm.Workspace.open(toWasmSource(wasm, source));

    return new WasmWorkspace(wasm, workspace);
}

class WasmRepository implements Repository {
    public constructor(
        private readonly wasm: WasmModule,
        private readonly repository: Wasm.Repository,
    ) {}

    public root(): string {
        return this.repository.root();
    }

    public workspace(): Workspace {
        return new WasmWorkspace(this.wasm, this.repository.workspace());
    }
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

    public edit(edits: readonly Edit[]): Commit {
        const result = this.session.edit(edits.map((edit) => toWasmEdit(this.wasm, edit)));

        return fromWasmCommit(result);
    }

    public editIfCurrent(revision: Revision, edits: readonly Edit[]): Commit {
        const result = this.session.editIfCurrent(
            toWasmRevision(this.wasm, revision),
            edits.map((edit) => toWasmEdit(this.wasm, edit)),
        );

        return fromWasmCommit(result);
    }

    public reload(): readonly Change[] {
        return this.session.reload().map(fromWasmChange);
    }

    public module(path: string): Module {
        return fromWasmModule(this.session.module(path));
    }

    public target(revision: Revision, packageValue: PackageId, name: string): TargetId {
        const target = this.session.target(
            toWasmRevision(this.wasm, revision),
            toWasmPackageId(this.wasm, packageValue),
            name,
        );

        return fromWasmTargetId(target);
    }

    public profile(revision: Revision, module: Module, name: string): ProfileId {
        const profile = this.session.profile(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            name,
        );

        return fromWasmProfileId(profile);
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

    public trace(revision: Revision, detailed: boolean): TraceReport | undefined {
        const trace = this.session.trace(toWasmRevision(this.wasm, revision), detailed);

        return trace == null ? undefined : fromWasmTraceReport(trace);
    }

    public build(revision: Revision, request: BuildRequest): BuildOutput {
        const output = this.session.build(
            toWasmRevision(this.wasm, revision),
            toWasmBuildRequest(this.wasm, request),
        );

        return fromWasmBuildOutput(output);
    }

    public content(id: ContentId): Content {
        const content = this.session.content(toWasmContentId(this.wasm, id));

        return fromWasmContent(content);
    }

    public text(id: ContentId): string {
        return this.session.text(toWasmContentId(this.wasm, id));
    }

    public bytes(id: ContentId): Uint8Array {
        return this.session.bytes(toWasmContentId(this.wasm, id));
    }

    public parse(revision: Revision, module: Module): ParseOutput {
        const output = this.session.parse(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
        );

        return fromWasmParseOutput(output);
    }

    public resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved {
        const resolved = this.session.resolve(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            toWasmProfileId(this.wasm, profile),
        );

        return fromWasmDirResolved(resolved);
    }

    public check(revision: Revision, module: Module, profile: ProfileId): CheckOutput {
        const output = this.session.check(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            toWasmProfileId(this.wasm, profile),
        );

        return fromWasmCheckOutput(output);
    }

    public format(revision: Revision, request: FormatRequest): FormatOutput {
        const output = this.session.format(
            toWasmRevision(this.wasm, revision),
            toWasmFormatRequest(this.wasm, request),
        );

        return fromWasmFormatOutput(output);
    }

    public lint(revision: Revision, request: LintRequest): LintOutput {
        const output = this.session.lint(
            toWasmRevision(this.wasm, revision),
            toWasmLintRequest(this.wasm, request),
        );

        return fromWasmLintOutput(output);
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

class WasmWorkspace implements Workspace {
    public constructor(
        private readonly wasm: WasmModule,
        private readonly workspace: Wasm.Workspace,
    ) {}

    public root(): string {
        return this.workspace.root();
    }

    public revision(): Revision {
        return this.workspace.revision();
    }

    public files(): readonly SessionFile[] {
        return this.workspace.files().map(fromWasmSessionFile);
    }

    public edit(edits: readonly Edit[]): Commit {
        const result = this.workspace.edit(edits.map((edit) => toWasmEdit(this.wasm, edit)));

        return fromWasmCommit(result);
    }

    public editIfCurrent(revision: Revision, edits: readonly Edit[]): Commit {
        const result = this.workspace.editIfCurrent(
            toWasmRevision(this.wasm, revision),
            edits.map((edit) => toWasmEdit(this.wasm, edit)),
        );

        return fromWasmCommit(result);
    }

    public reload(): readonly Change[] {
        return this.workspace.reload().map(fromWasmChange);
    }

    public module(path: string): Module {
        return fromWasmModule(this.workspace.module(path));
    }

    public target(revision: Revision, packageValue: PackageId, name: string): TargetId {
        const target = this.workspace.target(
            toWasmRevision(this.wasm, revision),
            toWasmPackageId(this.wasm, packageValue),
            name,
        );

        return fromWasmTargetId(target);
    }

    public profile(revision: Revision, module: Module, name: string): ProfileId {
        const profile = this.workspace.profile(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            name,
        );

        return fromWasmProfileId(profile);
    }

    public provide(revision: Revision, keys: readonly ArtifactKey[]): void {
        const transportKeys = keys.map((key) => toWasmArtifactKey(this.wasm, key));

        this.workspace.provide(toWasmRevision(this.wasm, revision), transportKeys);
    }

    public require(revision: Revision, key: ArtifactKey): ArtifactVersion {
        const version = this.workspace.require(
            toWasmRevision(this.wasm, revision),
            toWasmArtifactKey(this.wasm, key),
        );

        return {
            key,
            fingerprint: version.fingerprint,
        };
    }

    public artifactRecord(revision: Revision, key: ArtifactKey): ArtifactRecord {
        const record = this.workspace.artifactRecord(
            toWasmRevision(this.wasm, revision),
            toWasmArtifactKey(this.wasm, key),
        );

        return fromWasmArtifactRecord(record);
    }

    public trace(revision: Revision, detailed: boolean): TraceReport | undefined {
        const trace = this.workspace.trace(toWasmRevision(this.wasm, revision), detailed);

        return trace == null ? undefined : fromWasmTraceReport(trace);
    }

    public build(revision: Revision, request: BuildRequest): BuildOutput {
        const output = this.workspace.build(
            toWasmRevision(this.wasm, revision),
            toWasmBuildRequest(this.wasm, request),
        );

        return fromWasmBuildOutput(output);
    }

    public content(id: ContentId): Content {
        const content = this.workspace.content(toWasmContentId(this.wasm, id));

        return fromWasmContent(content);
    }

    public text(id: ContentId): string {
        return this.workspace.text(toWasmContentId(this.wasm, id));
    }

    public bytes(id: ContentId): Uint8Array {
        return this.workspace.bytes(toWasmContentId(this.wasm, id));
    }

    public parse(revision: Revision, module: Module): ParseOutput {
        const output = this.workspace.parse(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
        );

        return fromWasmParseOutput(output);
    }

    public resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved {
        const resolved = this.workspace.resolve(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            toWasmProfileId(this.wasm, profile),
        );

        return fromWasmDirResolved(resolved);
    }

    public check(revision: Revision, module: Module, profile: ProfileId): CheckOutput {
        const output = this.workspace.check(
            toWasmRevision(this.wasm, revision),
            toWasmModule(this.wasm, module),
            toWasmProfileId(this.wasm, profile),
        );

        return fromWasmCheckOutput(output);
    }

    public format(revision: Revision, request: FormatRequest): FormatOutput {
        const output = this.workspace.format(
            toWasmRevision(this.wasm, revision),
            toWasmFormatRequest(this.wasm, request),
        );

        return fromWasmFormatOutput(output);
    }

    public lint(revision: Revision, request: LintRequest): LintOutput {
        const output = this.workspace.lint(
            toWasmRevision(this.wasm, revision),
            toWasmLintRequest(this.wasm, request),
        );

        return fromWasmLintOutput(output);
    }

    public diagnostics(revision: Revision, key?: ArtifactKey): readonly Diagnostic[] {
        const diagnostics = this.workspace.diagnostics(
            toWasmRevision(this.wasm, revision),
            key == null ? undefined : toWasmArtifactKey(this.wasm, key),
        );

        return diagnostics.map(fromWasmDiagnostic);
    }

    public sidecars(revision: Revision, key: ArtifactKey): readonly ArtifactSidecar[] {
        const sidecars = this.workspace.sidecars(
            toWasmRevision(this.wasm, revision),
            toWasmArtifactKey(this.wasm, key),
        );

        return sidecars.map(fromWasmArtifactSidecar);
    }
}
