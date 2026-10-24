import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Process } from "@destack/environment";

/** The command starting the sandbox launcher outside the workload sandbox. */
export interface LauncherCommand {
    /** The executable to start. */
    readonly executable: string;
    /** The arguments to start it with. */
    readonly arguments: readonly string[];
}

/** Find the launcher command. */
export function launcherCommand(process: Process): LauncherCommand {
    // start the release's launcher from a standalone executable
    if (process.runtime.isStandalone) {
        return { executable: join(dirname(process.executable), "destack-sandbox"), arguments: [] };
    }

    // run the entry with Bun otherwise, preloading the module metadata the build would inject
    const entry = fileURLToPath(new URL("../main.ts", import.meta.url));
    const preload = fileURLToPath(import.meta.resolve("@destack/package/bun/preload"));

    return {
        executable: process.executable,
        arguments: ["run", "--no-env-file", "--preload", preload, entry],
    };
}
