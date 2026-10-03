import { readFile } from "node:fs/promises";
import { FileSystemError } from "../error/index.ts";

/** Read a text file, or nothing when it does not exist. */
export async function readOptional(path: string): Promise<string | undefined> {
    try {
        return await readFile(path, "utf8");
    } catch (cause) {
        // pass on a failure without an operating-system code
        if (!(cause instanceof Error) || !("code" in cause) || typeof cause.code !== "string") {
            throw cause;
        }
        // treat a missing file as absent
        else if (cause.code === "ENOENT") {
            return undefined;
        }
        // report any other failure with its operation and path
        else {
            throw new FileSystemError("read", path, cause.code, { cause });
        }
    }
}
