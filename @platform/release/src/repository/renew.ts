import { CHECKOUT } from "../distribution/distribution.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { Metadata, MetadataKind } from "@tufjs/models";
import { join } from "node:path";
import { readReleaseKeys, readRenewalKeys } from "../key/key.ts";
import { RepositoryConfiguration } from "./configuration.ts";
import { RepositoryRenewal } from "./renewal.ts";
import { writeBootstrap } from "./bootstrap.ts";
import { SignedRepository } from "@destack/update/publish";
import { MetadataDocument } from "./document.ts";
import { present } from "@destack/schema";

/** Renew the published metadata's freshness or targets roles into `dist/refresh`. */
export async function renewUpdate(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the role renewal the protected workflow requests
    const configuration = new RepositoryConfiguration(environment);
    const [operation = "freshness"] = arguments_;
    if (operation !== "freshness" && operation !== "targets") {
        throw new Error("select freshness or targets renewal");
    }

    // read the authenticated repository state, including expired freshness documents
    const renewal = present(
        await RepositoryRenewal.read(configuration.url, configuration.root(), environment),
        "a published release",
    );
    const output = join(CHECKOUT, "dist/refresh");

    // require protected release credentials to authorize a new targets expiration
    if (operation === "targets") {
        const keys = await readReleaseKeys(environment);
        const targets = Metadata.fromJSON(
            MetadataKind.Targets,
            MetadataDocument.parse(renewal.targets.toString()),
        );
        const signed = await SignedRepository.write(
            output,
            renewal.revision,
            renewal.root,
            keys,
            targets,
            environment,
        );
        await writeBootstrap(output, signed, configuration.url, environment.files);
    }
    // retain target signatures and require the publication service to compare authoritative storage
    else {
        const keys = await readRenewalKeys(environment);
        await SignedRepository.renew(
            output,
            renewal.revision,
            renewal.root,
            keys,
            renewal.targets,
            environment,
        );
    }

    // retain the complete rotation history for publication verification and older clients
    for (const root of renewal.history) {
        await environment.files.writeFile(
            join(output, "metadata", `${root.signed.version}.root.json`),
            SignedRepository.encode(root),
        );
    }
}
