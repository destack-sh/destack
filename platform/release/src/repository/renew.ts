import { Metadata, MetadataKind } from "@tufjs/models";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { writeFile } from "node:fs/promises";
import { readReleaseKeys, readRenewalKeys } from "../key/key.ts";
import { encode, renewMetadata, writeMetadata } from "./repository.ts";
import { RepositoryConfiguration } from "./configuration.ts";
import { RepositoryRenewal } from "./renewal.ts";

/** Source checkout used to publish refreshed metadata. */
const directory = fileURLToPath(new URL("../../../../", import.meta.url));
/** Selected release repository. */
const configuration = new RepositoryConfiguration();
/** Role renewal selected by the protected workflow. */
const operation = process.argv[2] ?? "freshness";
if (operation !== "freshness" && operation !== "targets") {
    throw new Error("select freshness or targets renewal");
}

/** Authenticated repository state, including potentially expired freshness documents. */
const renewal = await RepositoryRenewal.read(configuration.url, await configuration.root());
/** Directory containing renewed metadata for publication. */
const output = join(directory, "dist/refresh");

// require protected release credentials to authorize a new targets expiration
if (operation === "targets") {
    const keys = await readReleaseKeys();
    const targets = Metadata.fromJSON(MetadataKind.Targets, JSON.parse(renewal.targets.toString()));
    await writeMetadata(output, renewal.revision, renewal.root, keys, targets, configuration.url);
}
// retain target signatures and require the publication service to compare authoritative storage
else {
    const keys = await readRenewalKeys();
    await renewMetadata(output, renewal.revision, renewal.root, keys, renewal.targets);
}

// retain the complete rotation history for publication verification and older clients
for (const root of renewal.history) {
    await writeFile(join(output, "metadata", `${root.signed.version}.root.json`), encode(root));
}
