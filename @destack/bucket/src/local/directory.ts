import { open } from "node:fs/promises";

/** Flush a directory's entries before publishing references to its files. */
export async function syncDirectory(directory: string): Promise<void> {
    // request write access for Windows FlushFileBuffers
    const mode = process.platform === "win32" ? "r+" : "r";
    await using file = await open(directory, mode);
    await file.sync();
}
