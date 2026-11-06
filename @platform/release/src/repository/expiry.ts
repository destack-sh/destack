import { Updater } from "tuf-js";
import { join } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { RepositoryConfiguration } from "./configuration.ts";
import { schema } from "@destack/schema";
import { print } from "../distribution/output.ts";

/** The expiry of a signed metadata document. */
const Expiring = schema
    .object({ signed: schema.object({ expires: schema.string() }).strip() })
    .strip();

/** The milliseconds in one day. */
const DAY_MS = 86_400_000;

/** The days each role must stay valid before the check fails. */
const MINIMUM_LIFETIME_DAYS = { root: 90, targets: 30, snapshot: 3, timestamp: 3 };

/** Verify public update trust and report near renewal deadlines without signing credentials. */
export async function checkExpiry(environment: Environment<HostCapability>): Promise<void> {
    // use a fresh verifier so an unavailable or invalid repository fails independently of renewal
    const { files } = environment;
    const configuration = new RepositoryConfiguration(environment);
    const directory = await files.makeTemporaryDirectory("destack-expiry-");
    try {
        await files.writeTextFile(
            join(directory, "root.json"),
            JSON.stringify(configuration.root().toJSON()),
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
        const now = environment.clock.now();
        for (const [role, days] of Object.entries(MINIMUM_LIFETIME_DAYS)) {
            const text = await files.readTextFile(join(directory, `${role}.json`));
            const document = Expiring.parse(JSON.parse(text));
            const expiration = Date.parse(document.signed.expires);
            if (!Number.isFinite(expiration) || expiration - now <= days * DAY_MS) {
                failures.push(`${role} expires at ${document.signed.expires}`);
            }
            print(`${role}: ${document.signed.expires}`, environment);
        }
        if (failures.length) {
            throw new Error(`release metadata needs renewal: ${failures.join(", ")}`);
        }
    } finally {
        await files.remove(directory, { recursive: true, force: true });
    }
}
