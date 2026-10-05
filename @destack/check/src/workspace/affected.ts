import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { CheckError } from "../error/index.ts";
import { Workspace } from "./workspace.ts";

/** Run a command and read its standard output. */
const run = promisify(execFile);

/** List the member directories the changes since a base commit affect, absent when a change affects every member. */
export async function listAffected(directory: string, base: string): Promise<string[] | undefined> {
    // read the workspace enclosing the directory
    const workspace = await Workspace.read(directory);
    if (workspace === undefined) {
        throw new CheckError("CONFIGURATION", `no workspace encloses ${directory}`);
    }

    // list the files changed since the merge base with both sides of renames
    const { stdout } = await run("git", ["diff", "--name-only", "--no-renames", `${base}...HEAD`], {
        cwd: workspace.root,
    }).catch((error: unknown) => {
        throw new CheckError("TOOL", `git diff against ${base} failed`, { cause: error });
    });
    const files = stdout.split("\n").filter((line) => line !== "");

    // select the affected members
    return workspace.affected(files);
}
