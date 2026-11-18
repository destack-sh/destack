import type { Environment } from "@destack/environment";
import { CheckError } from "../error/index.ts";
import { Workspace } from "./workspace.ts";

/** List the member directories the working tree's changes since a base commit affect. */
export async function listAffected(
    directory: string,
    base: string,
    environment: Environment<"files" | "process">,
): Promise<string[] | undefined> {
    // read the workspace enclosing the directory
    const workspace = await Workspace.read(directory, environment.files);
    if (workspace === undefined) {
        throw CheckError.of(
            "CONFIGURATION_INVALID",
            `directory '${directory}' is invalid: it is not inside a workspace`,
        );
    }

    // list the files changed in the working tree since the merge base, and the untracked files
    const changed = await listGit(
        ["diff", "--name-only", "--no-renames", "--merge-base", base],
        workspace.root,
        environment,
    );
    const untracked = await listGit(
        ["ls-files", "--others", "--exclude-standard"],
        workspace.root,
        environment,
    );

    // select the affected members
    return workspace.affected([...changed, ...untracked]);
}

/** List the lines a git command prints in a directory. */
async function listGit(
    command: string[],
    directory: string,
    environment: Environment<"process">,
): Promise<string[]> {
    // run git
    const child = environment.process.spawn(["git", ...command], {
        workingDirectory: directory,
        stderr: "pipe",
    });
    const { code, stdout, stderr } = await child.output();
    const decoder = new TextDecoder();
    if (code !== 0) {
        const message = decoder.decode(stderr).trim();
        throw CheckError.of("TOOL_BROKEN", `git command '${command.join(" ")}' failed: ${message}`);
    }

    return decoder
        .decode(stdout)
        .split("\n")
        .filter((line) => line !== "");
}
