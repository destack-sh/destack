import { Updater } from "tuf-js";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { RepositoryConfiguration } from "./configuration.ts";

/** Minimum remaining lifetime for each authenticated metadata role, in milliseconds. */
const MINIMUM_LIFETIME_MS = {
    root: 90 * 86400000,
    targets: 30 * 86400000,
    snapshot: 3 * 86400000,
    timestamp: 86400000,
};

/** Verify public update trust and report approaching renewal deadlines without signing credentials. */
async function checkExpiry(): Promise<void> {
    // use a fresh verifier so an unavailable or invalid repository fails independently of renewal
    const configuration = new RepositoryConfiguration();
    const directory = await mkdtemp(join(tmpdir(), "destack-expiry-"));
    try {
        await writeFile(
            join(directory, "root.json"),
            JSON.stringify((await configuration.root()).toJSON()),
        );
        const updater = new Updater({
            metadataDir: directory,
            metadataBaseUrl: new URL("metadata/", configuration.url).href,
            targetDir: join(directory, "targets"),
            targetBaseUrl: new URL("targets/", configuration.url).href,
        });
        await updater.refresh();

        // report every approaching deadline after verifying the complete signed metadata chain
        const failures: string[] = [];
        const now = Date.now();
        for (const [role, minimum] of Object.entries(MINIMUM_LIFETIME_MS)) {
            const document = JSON.parse(await readFile(join(directory, `${role}.json`), "utf8"));
            const expiration = Date.parse(document.signed.expires);
            if (!Number.isFinite(expiration) || expiration - now <= minimum) {
                failures.push(`${role} expires at ${document.signed.expires}`);
            }
            console.log(`${role}: ${document.signed.expires}`);
        }
        if (failures.length) {
            throw new Error(`release metadata needs renewal: ${failures.join(", ")}`);
        }
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
}

await checkExpiry();
