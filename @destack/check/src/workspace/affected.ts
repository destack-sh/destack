import type { Environment } from "@destack/environment";
import { CheckError } from "../error/index.ts";
import { Workspace } from "./workspace.ts";

/** List the member directories the changes since a base commit affect. */
export async function listAffected(
    directory: string,
    base: string,
    environment: Environment<"files" | "process">,
): Promise<string[] | undefined> {
    // read the workspace enclosing the directory
    const workspace = await Workspace.read(directory, environment.files);
    if (workspace === undefined) {
        throw CheckError.of("CONFIGURATION_INVALID", `${directory} is not inside a workspace`);
    }

    // list the files changed since the merge base with both sides of renames
    const child = environment.process.spawn(
        ["git", "diff", "--name-only", "--no-renames", `${base}...HEAD`],
        { workingDirectory: workspace.root, stderr: "pipe" },
    );
    const { code, stdout, stderr } = await child.output();
    const decoder = new TextDecoder();
    if (code !== 0) {
        const message = decoder.decode(stderr).trim();
        throw CheckError.of("TOOL_FAILED", `git diff against ${base} failed: ${message}`);
    }
    const files = decoder
        .decode(stdout)
        .split("\n")
        .filter((line) => line !== "");

    // select the affected members
    return workspace.affected(files);
}
