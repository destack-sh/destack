import { CHECKOUT } from "../distribution/distribution.ts";
import { resolve } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { RepositoryConfiguration } from "./configuration.ts";
import { publish } from "./publish.ts";

/** Publish the signed repository directory the command line selects, below the checkout. */
export async function publishUpdate(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const [selected = "dist/update"] = arguments_;
    await publish(
        resolve(CHECKOUT, selected),
        new RepositoryConfiguration(environment),
        environment,
    );
}
