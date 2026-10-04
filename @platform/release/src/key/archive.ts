import { lstat } from "node:fs/promises";
import { basename, isAbsolute } from "node:path";
import { Bitwarden } from "./bitwarden.ts";
import { BackupKey, readBackup } from "./backup.ts";
import { print } from "../output/index.ts";

/** Save a verified encrypted signing backup with its recovery key in Bitwarden. */
async function main(): Promise<void> {
    // require explicit absolute source and destination paths
    const [directory, output, ...extra] = process.argv.slice(2);
    if (
        directory === undefined ||
        output === undefined ||
        extra.length > 0 ||
        !isAbsolute(directory) ||
        !isAbsolute(output)
    ) {
        throw new Error(
            "usage: archive.ts <absolute-signing-directory> <absolute-new-archive.age>",
        );
    }

    // detect destination collisions before creating a vault record
    if (await isPresent(output)) {
        throw new Error("backup destination already exists; choose a new archive name");
    }

    // persist the recovery identity before writing any encrypted material
    Bitwarden.sync();
    const archive = readBackup(directory);
    const name = `Destack release backup / ${basename(output)}`;
    const identity = Bitwarden.readRecovery(name);
    const key = new BackupKey(identity);
    if (identity === undefined) {
        Bitwarden.saveRecovery(name, key.identity);
    }
    print(`recovery identity verified in Bitwarden: ${name}`);

    // verify the encrypted destination before reporting successful completion
    await key.write(archive, output);
    print(`encrypted backup verified byte for byte: ${output}`);
}

/** Report whether a path names any file system entry, including a dangling link. */
async function isPresent(path: string): Promise<boolean> {
    try {
        await lstat(path);
    } catch (error) {
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
            return false;
        }
        throw error;
    }

    return true;
}

await main();
