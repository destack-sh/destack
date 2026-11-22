import { fileURLToPath } from "node:url";
import { CompilerPool, LocalBuildStore, Workspace } from "@destack/build";
import { Toolchain } from "@destack/check/toolchain";
import { BunEnvironment } from "@destack/environment/bun";
import { DevelopmentServer } from "@destack/view/build";

/** The port the examples page listens on, beside the site's 3737. */
const PORT = 3738;

/** The package whose examples the page shows. */
const directory = fileURLToPath(new URL("../", import.meta.url));

/** The process's own effects, which this entrypoint reads. */
const environment = BunEnvironment.system;

/** The person's development build store. */
await using store = await LocalBuildStore.user("development", environment);

/** The compilers the workspace builds with. */
await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());

/** The package's workspace, whose root declares the examples. */
const workspace = await (await Workspace.open(directory, compilers)).package();

/** The development server of the package's examples, listening until a shutdown signal. */
await using server = await DevelopmentServer.start({
    workspace,
    environment,
    server: { port: PORT, strictPort: true },
});
server.vite.printUrls();
await new Promise<void>((resolve) => {
    environment.process.on("SIGINT", resolve);
    environment.process.on("SIGTERM", resolve);
});
