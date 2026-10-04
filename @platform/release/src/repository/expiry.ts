import { Updater } from "tuf-js";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { RepositoryConfiguration } from "./configuration.ts";
import { schema } from "@destack/schema";
import { print } from "../output/index.ts";

/** The expiry a signed metadata document carries. */
const Expiring = schema
    .object({ signed: schema.object({ expires: schema.string() }).strip() })
    .strip();

/** The milliseconds in one day. */
const DAY_MS = 86_400_000;

/**
 * The days each role must stay valid before the check fails.
 *
 * Root and targets leave time for an offline ceremony or a stable release.
 * Snapshot and timestamp leave three days to repair the nightly that re-signs them.
 */
const MINIMUM_LIFETIME_DAYS = { root: 90, targets: 30, snapshot: 3, timestamp: 3 };

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
        for (const [role, days] of Object.entries(MINIMUM_LIFETIME_DAYS)) {
            const document = Expiring.parse(
                JSON.parse(await readFile(join(directory, `${role}.json`), "utf8")),
            );
            const expiration = Date.parse(document.signed.expires);
            if (!Number.isFinite(expiration) || expiration - now <= days * DAY_MS) {
                failures.push(`${role} expires at ${document.signed.expires}`);
            }
            print(`${role}: ${document.signed.expires}`);
        }
        if (failures.length) {
            throw new Error(`release metadata needs renewal: ${failures.join(", ")}`);
        }
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
}

await checkExpiry();
