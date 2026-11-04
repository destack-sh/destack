import { CompilerPool, LocalBuildStore, outputsOf, Workspace } from "@destack/build";
import { DevelopmentServer } from "@destack/view/build";
import { type Environment, FileSystemError } from "@destack/environment";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

/** The site's static view. */
const VIEW = "site";

/** Absolute package directory for development and production builds. */
const directory = fileURLToPath(new URL("../", import.meta.url));

/** Build the site, or serve it with live updates. */
export async function run(
    command: string | undefined,
    environment: Environment<"files" | "process" | "host">,
): Promise<void> {
    // open the site's workspace on the person's development store
    await using store = await LocalBuildStore.user("development", environment);
    await using compilers = await CompilerPool.open(store);
    const workspace = await Workspace.open(directory, compilers);

    // serve the view, rendering each page on request, until the process receives a shutdown signal
    if (command === "dev") {
        await using development = await DevelopmentServer.start({
            workspace,
            view: VIEW,
            environment,
            server: { port: 3737, strictPort: true, watch: { ignored: ["**/.output/**"] } },
        });
        development.vite.printUrls();
        await new Promise<void>((resolve) => {
            environment.process.on("SIGINT", resolve);
            environment.process.on("SIGTERM", resolve);
        });
    }
    // build the site and write its build, which a machine serves, into the output directory
    else if (command === "build") {
        const build = await workspace.build({ outputs: outputsOf(workspace.root.packageJson) });
        const output = join(directory, ".output");
        await environment.files.remove(output, { recursive: true }).catch(FileSystemError.absent);
        await build.writeDirectory(output, environment.files);
    } else {
        throw new Error("expected build or dev");
    }
}
