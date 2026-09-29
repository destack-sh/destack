import { lstat } from "node:fs/promises";
import { basename, isAbsolute } from "node:path";
import { Bitwarden } from "./bitwarden.ts";
import { BackupKey, readBackup } from "./backup.ts";

/** Save a verified encrypted signing backup with its recovery key in Bitwarden. */
async function main(): Promise<void> {
    // require explicit absolute source and destination paths
    const [directory, output, ...extra] = process.argv.slice(2);
    if (!directory || !output || extra.length || !isAbsolute(directory) || !isAbsolute(output)) {
        throw new Error("usage: backup.ts <absolute-signing-directory> <absolute-new-archive.age>");
    }

    // detect destination collisions before creating a vault record
    const existing = await lstat(output).catch((error: NodeJS.ErrnoException) => {
        if (error.code !== "ENOENT") {
            throw error;
        }
        return undefined;
    });
    if (existing) {
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
    console.log(`recovery identity verified in Bitwarden: ${name}`);

    // verify the encrypted destination before reporting successful completion
    await key.write(archive, output);
    console.log(`encrypted backup verified byte for byte: ${output}`);
}

await main();
