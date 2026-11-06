import { CHECKOUT } from "../distribution/distribution.ts";
import { join, resolve } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { Metadata, MetadataKind } from "@tufjs/models";
import { RepositoryConfiguration } from "./configuration.ts";
import { print } from "../distribution/output.ts";
import { MetadataDocument } from "./document.ts";

/** How long the publication service may take to answer, in milliseconds. */
const SUBMIT_TIMEOUT_MILLISECONDS = 15_000;

/** Submit only signed freshness documents without a bucket credential. */
export async function submitUpdate(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the exact snapshot referenced by the newly signed timestamp
    const { files } = environment;
    const [selected = "dist/refresh"] = arguments_;
    const directory = resolve(CHECKOUT, selected);
    const timestamp = await files.readTextFile(join(directory, "metadata/timestamp.json"));
    const metadata = Metadata.fromJSON(MetadataKind.Timestamp, MetadataDocument.parse(timestamp));
    const revision = metadata.signed.snapshotMeta.version;
    const snapshot = await files.readTextFile(
        join(directory, `metadata/${revision}.snapshot.json`),
    );
    const configuration = new RepositoryConfiguration(environment);

    // let the publication service verify role signatures and atomically advance freshness
    const response = await environment.network.fetch(
        new Request(new URL("renew", configuration.url), {
            method: "POST",
            redirect: "error",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ snapshot, timestamp }),
            signal: environment.clock.timeout(SUBMIT_TIMEOUT_MILLISECONDS),
        }),
    );
    if (response.status !== 204) {
        const reason = await response.text();
        throw new Error(`release renewal rejected: ${response.status} ${reason}`);
    }
    print(`Published freshness revision ${revision}.`, environment);
}
