/// <reference types="bun" />
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The command starting the sandbox launcher outside the workload sandbox. */
export interface LauncherCommand {
    /** The executable to start. */
    readonly executable: string;
    /** The arguments to start it with. */
    readonly arguments: readonly string[];
}

/** Name the launcher command: the release's launcher beside a standalone executable, else Bun running the launcher entry. */
export function launcherCommand(): LauncherCommand {
    // start the release's launcher from a standalone executable
    if (Bun.isStandaloneExecutable) {
        return { executable: join(dirname(process.execPath), "destack-sandbox"), arguments: [] };
    }

    // run the entry with Bun otherwise
    const entry = fileURLToPath(new URL("../main.ts", import.meta.url));

    return { executable: process.execPath, arguments: ["run", "--no-env-file", entry] };
}
