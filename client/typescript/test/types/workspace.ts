import type { Change } from "../../src/_generated/repository/change.js";
import type { Commit } from "../../src/_generated/repository/commit.js";
import type { File } from "../../src/_generated/repository/file.js";
import type { Revision } from "../../src/_generated/repository/revision.js";
import type { TextRange } from "../../src/_generated/source/edit/text.js";
import type { CheckInput } from "../../src/_generated/workspace/command/check.js";
import type { InfoInput } from "../../src/_generated/workspace/command/info.js";
import type { FileSelection } from "../../src/_generated/workspace/branch.js";
import type { ArtifactRequest, ExportRequest } from "../../src/_generated/workspace/service/artifact.js";
import type * as Command from "../../src/_generated/workspace/service/command.js";
import type {
    ResolveQueryFileRequest,
    RunQueryRequest,
} from "../../src/_generated/workspace/service/query.js";
import type {
    EditBranchRequest,
    EditRequest,
    ReadFilesRequest,
} from "../../src/_generated/workspace/service/source.js";
import type { Blob, Workspace, WorkspaceBranch } from "../../src/index.js";
import { openWorkspace } from "../../src/index.js";

declare const workspace: Workspace;
declare const branch: WorkspaceBranch;
declare const revision: Revision;
declare const checkInput: CheckInput;
declare const infoInput: InfoInput;
declare const editBranchRequest: Omit<EditBranchRequest, "root" | "name">;
declare const editRequest: Omit<EditRequest, "root">;
declare const files: FileSelection;
declare const range: TextRange;
declare const artifact: ArtifactRequest["artifact"];
declare const exportInput: ExportRequest["input"];
declare const buildInput: Command.BuildRequest["input"];
declare const cacheInput: Command.CacheRequest["input"];
declare const cleanInput: Command.CleanRequest["input"];
declare const docInput: Command.DocRequest["input"];
declare const doctorInput: Command.DoctorRequest["input"];
declare const formatInput: Command.FormatRequest["input"];
declare const queryInput: Command.QueryRequest["input"];
declare const rewriteInput: Command.RewriteRequest["input"];
declare const settingsInput: Command.SettingsRequest["input"];
declare const targetsInput: Command.TargetsRequest["input"];
declare const taskInput: Command.TaskRequest["input"];
declare const testInput: Command.TestRequest["input"];
declare const readFiles: Omit<ReadFilesRequest, "root">;
declare const queryPath: ResolveQueryFileRequest["path"];
declare const queryRun: RunQueryRequest["input"];

const physical: Promise<Workspace> = openWorkspace({ root: "/project" });
const memory: Promise<Workspace> = openWorkspace({
    memory: { root: "/project", files: {} },
});
const remote: Promise<Workspace> = openWorkspace({
    url: "ws://localhost/rpc",
    root: "/project",
});

const physicalRevision: Promise<Revision> = workspace.revision();
const boundBranch: WorkspaceBranch = workspace.branch("studio");
const branches: Promise<WorkspaceBranch[]> = workspace.branches();
const exactBranch: Promise<WorkspaceBranch> = workspace.createBranch("studio", revision);
const currentBranch: Promise<WorkspaceBranch> = workspace.createBranch("studio");
workspace.reload();
const written: Promise<Commit> = workspace.edit(editRequest);
const changes: Promise<readonly Change[]> = workspace.diff({
    before: revision,
    after: revision,
});
const repositoryFiles: Promise<readonly File[]> = workspace.files(revision);
const branchRevision: Promise<Revision> = branch.revision();
const edited: Promise<Commit> = branch.edit(editBranchRequest);
const persisted: Promise<Commit> = branch.save(files);
branch.restore(files);
branch.watch();
branch.remove();
workspace.check(checkInput);
workspace.format(formatInput);
workspace.query(queryInput);
workspace.rewrite(rewriteInput);
workspace.build(buildInput);
workspace.test(testInput);
workspace.doc(docInput);
workspace.info(infoInput);
workspace.targets(targetsInput);
workspace.cache(cacheInput);
workspace.settings(settingsInput);
workspace.doctor(doctorInput);
workspace.task(taskInput);
workspace.clean(cleanInput);
workspace.formatFile({ revision, path: "main.ds", range });
workspace.readFiles(readFiles);
workspace.artifact(artifact);
const artifactBlob: Promise<Blob> = workspace.blob(artifact);
workspace.export(exportInput);
workspace.diagnose(revision);
workspace.resolveQueryFile({ revision, path: queryPath });
workspace.runQuery(queryRun);

void physical;
void memory;
void remote;
void artifactBlob;
void physicalRevision;
void boundBranch;
void branches;
void exactBranch;
void currentBranch;
void written;
void changes;
void repositoryFiles;
void branchRevision;
void edited;
void persisted;

// @ts-expect-error the workspace supplies its own root
workspace.check({ root: "/wrong", input: checkInput });

// @ts-expect-error the workspace supplies its own root
workspace.formatFile({ root: "/wrong", revision, path: "main.ds", range });
