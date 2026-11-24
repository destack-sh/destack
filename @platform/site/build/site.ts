import { CompilerPool, LocalBuildStore, outputsOf, Workspace } from "@destack/build";
import { Toolchain } from "@destack/check/toolchain";
import { type Environment, FileSystemError } from "@destack/environment";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

/** Absolute package directory for development and production builds. */
const directory = fileURLToPath(new URL("../", import.meta.url));

/** Build the site into its output directory. */
export async function run(
    command: string | undefined,
    environment: Environment<"files" | "process" | "host">,
): Promise<void> {
    // open the site's workspace on the person's development store
    await using store = await LocalBuildStore.user("development", environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspacePackage = await (await Workspace.open(directory, compilers)).package();

    // build the site and write its build, which a machine serves, into the output directory
    if (command === "build") {
        const build = await workspacePackage.build({
            outputs: outputsOf(workspacePackage.root.packageJson),
        });
        const output = join(directory, ".output");
        await environment.files.remove(output, { recursive: true }).catch(FileSystemError.absent);
        await build.writeDirectory(output, environment.files);
    } else {
        throw new Error("expected build");
    }
}
