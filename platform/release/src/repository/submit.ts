import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { Metadata, MetadataKind } from "@tufjs/models";
import { RepositoryConfiguration } from "./configuration.ts";

/** Submit only signed freshness documents without a bucket credential. */
async function submit(): Promise<void> {
    // select the exact snapshot referenced by the newly signed timestamp
    const directory = resolve(process.argv[2] ?? "dist/refresh");
    const timestamp = await readFile(join(directory, "metadata/timestamp.json"), "utf8");
    const metadata = Metadata.fromJSON(MetadataKind.Timestamp, JSON.parse(timestamp));
    const revision = metadata.signed.snapshotMeta.version;
    const snapshot = await readFile(join(directory, `metadata/${revision}.snapshot.json`), "utf8");
    const configuration = new RepositoryConfiguration();

    // let the publication service verify role signatures and atomically advance freshness
    const response = await fetch(new URL("renew", configuration.url), {
        method: "POST",
        redirect: "error",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ snapshot, timestamp }),
        signal: AbortSignal.timeout(15000),
    });
    if (response.status !== 204) {
        throw new Error(`release renewal rejected: ${response.status}`);
    }
    console.log(`Published freshness revision ${revision}.`);
}

await submit();
