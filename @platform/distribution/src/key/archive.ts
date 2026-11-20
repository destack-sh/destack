import { basename, isAbsolute } from "node:path";
import { type Environment, FileSystemError, type HostCapability } from "@destack/environment";
import { Bitwarden } from "./bitwarden.ts";
import { BackupKey, readBackup } from "./backup.ts";
import { print } from "../distribution/output.ts";

/** Save a verified encrypted signing backup with its recovery key in Bitwarden. */
export async function archiveBackup(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // require explicit absolute source and destination paths
    const [directory, output, ...extra] = arguments_;
    if (
        directory === undefined ||
        output === undefined ||
        extra.length > 0 ||
        !isAbsolute(directory) ||
        !isAbsolute(output)
    ) {
        throw new Error(
            "usage: key backup <absolute-signing-directory> <absolute-new-archive.age>",
        );
    }

    // detect destination collisions before creating a vault record
    if (await isPresent(output, environment)) {
        throw new Error("backup destination already exists; choose a new archive name");
    }

    // persist the recovery identity before writing any encrypted material
    const bitwarden = new Bitwarden(environment);
    await bitwarden.sync();
    const archive = await readBackup(directory, environment);
    const name = `Destack release backup / ${basename(output)}`;
    const identity = await bitwarden.readRecovery(name);
    const key =
        identity === undefined
            ? await BackupKey.generate(environment)
            : await BackupKey.of(identity, environment);
    if (identity === undefined) {
        await bitwarden.saveRecovery(name, key.identity);
    }
    print(`recovery identity verified in Bitwarden: ${name}`, environment);

    // verify the encrypted destination before reporting successful completion
    await key.write(archive, output);
    print(`encrypted backup verified byte for byte: ${output}`, environment);
}

/** Report whether any file system entry exists at a path, including a dangling link. */
async function isPresent(path: string, environment: Environment<"files">): Promise<boolean> {
    const status = await environment.files
        .lstat(path)
        .catch((error: unknown) => FileSystemError.absent(error));

    return status !== undefined;
}
