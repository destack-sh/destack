import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { type ReleaseKeys, type RenewalKeys, SigningKey } from "@destack/update/publish";

/** Read separate online role keys from the selected private directory. */
export async function readReleaseKeys(): Promise<ReleaseKeys> {
    // load the additional authorization key only for publication
    const [targets, renewal] = await Promise.all([readKey("targets"), readRenewalKeys()]);

    return { targets, ...renewal };
}

/** Read freshness keys without requesting the targets signing credential. */
export async function readRenewalKeys(): Promise<RenewalKeys> {
    // load only the two roles permitted to maintain freshness
    const [snapshot, timestamp] = await Promise.all([readKey("snapshot"), readKey("timestamp")]);

    return { snapshot, timestamp };
}

/** Read one role's private key from the online key directory. */
async function readKey(role: keyof ReleaseKeys): Promise<SigningKey> {
    return new SigningKey(await readFile(join(keyDirectory(), `${role}.pem`), "utf8"));
}

/** Read the private online key directory from the environment. */
function keyDirectory(): string {
    // require the directory before reading any role key
    const directory = process.env["DESTACK_RELEASE_KEYS"];
    if (directory === undefined || directory === "") {
        throw new Error("set DESTACK_RELEASE_KEYS to the private online key directory");
    }

    return directory;
}
