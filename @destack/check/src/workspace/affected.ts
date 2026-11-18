import type { Environment } from "@destack/environment";
import { CheckError } from "../error/index.ts";
import { Workspace } from "./workspace.ts";

/** List the member directories the working tree's changes since a base commit affect. */
export async function listAffected(
    directory: string,
    base: string,
    environment: Environment<"files" | "process">,
): Promise<string[] | undefined> {
    // select the members the changes touch
    const workspace = await readWorkspace(directory, environment);

    return workspace.affected(await listChanged(workspace.root, base, environment));
}

/** List a package's TypeScript modules changed since a base commit, from the package. */
export async function listChangedModules(
    directory: string,
    base: string,
    environment: Environment<"files" | "process">,
): Promise<string[]> {
    // select the package's existing modules among the changes
    const workspace = await readWorkspace(directory, environment);
    const changed = await listChanged(workspace.root, base, environment);

    return await workspace.modules(directory, changed, environment.files);
}

/** Read the workspace enclosing a directory, refusing a directory outside any. */
async function readWorkspace(
    directory: string,
    environment: Environment<"files">,
): Promise<Workspace> {
    const workspace = await Workspace.read(directory, environment.files);
    if (workspace === undefined) {
        throw CheckError.of(
            "CONFIGURATION_INVALID",
            `directory '${directory}' is invalid: it is not inside a workspace`,
        );
    }

    return workspace;
}

/** List the paths changed since a base's merge base, and the untracked ones, from a root. */
async function listChanged(
    root: string,
    base: string,
    environment: Environment<"process">,
): Promise<string[]> {
    const changed = await listGit(
        ["diff", "--name-only", "--no-renames", "--merge-base", base],
        root,
        environment,
    );
    const untracked = await listGit(
        ["ls-files", "--others", "--exclude-standard"],
        root,
        environment,
    );

    return [...changed, ...untracked];
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
