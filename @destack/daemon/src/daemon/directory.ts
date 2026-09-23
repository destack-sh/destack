import { mkdir, chmod } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

/** Locate the current OS user's Destack state, including daemon and distribution files. */
export function localDirectory(): string {
    return process.env["DESTACK_DIRECTORY"] ?? join(homedir(), ".destack");
}

/** Create private local state before process startup. */
export async function createDirectory(directory: string): Promise<void> {
    await mkdir(directory, { recursive: true, mode: 0o700 });
    if (process.platform !== "win32") {
        await chmod(directory, 0o700);
    }
}
