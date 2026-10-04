import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Metadata, MetadataKind } from "@tufjs/models";
import { RepositoryConfiguration } from "./configuration.ts";
import { print } from "../output/index.ts";
import { parseDocument } from "./document.ts";

/** Source checkout containing the renewed metadata. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

/** Submit only signed freshness documents without a bucket credential. */
async function submit(): Promise<void> {
    // select the exact snapshot referenced by the newly signed timestamp
    const directory = resolve(ROOT, process.argv[2] ?? "dist/refresh");
    const timestamp = await readFile(join(directory, "metadata/timestamp.json"), "utf8");
    const metadata = Metadata.fromJSON(MetadataKind.Timestamp, parseDocument(timestamp));
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
        const reason = await response.text();
        throw new Error(`release renewal rejected: ${response.status} ${reason}`);
    }
    print(`Published freshness revision ${revision}.`);
}

await submit();
