import type { Environment } from "@destack/environment";
import { join } from "node:path";
import { type ReleaseKeys, type RenewalKeys, SigningKey } from "@destack/update/publish";

/** Read separate online role keys from the selected private directory. */
export async function readReleaseKeys(
    environment: Environment<"files" | "process">,
): Promise<ReleaseKeys> {
    // load the additional authorization key only for publication
    const [targets, renewal] = await Promise.all([
        readKey("targets", environment),
        readRenewalKeys(environment),
    ]);

    return { targets, ...renewal };
}

/** Read freshness keys without requesting the targets signing credential. */
export async function readRenewalKeys(
    environment: Environment<"files" | "process">,
): Promise<RenewalKeys> {
    // load only the two roles permitted to maintain freshness
    const [snapshot, timestamp] = await Promise.all([
        readKey("snapshot", environment),
        readKey("timestamp", environment),
    ]);

    return { snapshot, timestamp };
}

/** Read one role's private key from the online key directory. */
async function readKey(
    role: keyof ReleaseKeys,
    environment: Environment<"files" | "process">,
): Promise<SigningKey> {
    const path = join(keyDirectory(environment), `${role}.pem`);

    return await SigningKey.import(await environment.files.readTextFile(path), environment.crypto);
}

/** Read the private online key directory from the environment. */
function keyDirectory(environment: Environment<"process">): string {
    // require the directory before reading any role key
    const directory = environment.process.variables["DESTACK_RELEASE_KEYS"];
    if (directory === undefined || directory === "") {
        throw new Error("set DESTACK_RELEASE_KEYS to the private online key directory");
    }

    return directory;
}
