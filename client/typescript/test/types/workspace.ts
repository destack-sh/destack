import type { TextRange } from "../../src/_generated/source/edit/text.js";
import type { CheckInput } from "../../src/_generated/workspace/command/check.js";
import type { InfoInput } from "../../src/_generated/workspace/command/info.js";
import type { FileOperation } from "../../src/_generated/workspace/file/image.js";
import type { SourceUpdate } from "../../src/_generated/workspace/update.js";
import type { ArtifactRequest, ExportRequest } from "../../src/_generated/workspace/service/artifact.js";
import type * as Command from "../../src/_generated/workspace/service/command.js";
import type {
    ResolveQueryFileRequest,
    RunQueryRequest,
} from "../../src/_generated/workspace/service/query.js";
import type { ReadFilesRequest } from "../../src/_generated/workspace/service/source.js";
import type { Workspace } from "../../src/index.js";
import { openWorkspace } from "../../src/index.js";

declare const workspace: Workspace;
declare const checkInput: CheckInput;
declare const infoInput: InfoInput;
declare const fileOperation: FileOperation;
declare const sourceUpdate: SourceUpdate;
declare const range: TextRange;
declare const artifact: ArtifactRequest["artifact"];
declare const exportInput: ExportRequest["input"];
declare const benchInput: Command.BenchRequest["input"];
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

workspace.revision();
workspace.reload();
workspace.watch();
workspace.applyFileOperation(fileOperation);
workspace.applySourceUpdate(sourceUpdate);
workspace.check(checkInput);
workspace.format(formatInput);
workspace.query(queryInput);
workspace.rewrite(rewriteInput);
workspace.build(buildInput);
workspace.test(testInput);
workspace.doc(docInput);
workspace.bench(benchInput);
workspace.info(infoInput);
workspace.targets(targetsInput);
workspace.cache(cacheInput);
workspace.settings(settingsInput);
workspace.doctor(doctorInput);
workspace.task(taskInput);
workspace.clean(cleanInput);
workspace.formatFile({ path: "main.ds", range });
workspace.isFileOpen("main.ds");
workspace.readFiles(readFiles);
workspace.artifact(artifact);
workspace.export(exportInput);
workspace.diagnose();
workspace.resolveQueryFile(queryPath);
workspace.runQuery(queryRun);

void physical;
void memory;
void remote;

// @ts-expect-error the workspace supplies its own root
workspace.check({ root: "/wrong", input: checkInput });

// @ts-expect-error the workspace supplies its own root
workspace.formatFile({ root: "/wrong", path: "main.ds", range });
