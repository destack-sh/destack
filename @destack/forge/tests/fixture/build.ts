import { join } from "node:path";
import { testSimulation } from "@destack/object/test";
import { inject } from "@destack/test";
import { PackageBuild } from "@destack/build";
import { FIXTURE_VERSION, type FixturePackage } from "./package.ts";

/** The environment the global setup's builds are read in, over the host's files. */
const environment = testSimulation().environment("builds");

/** Read the global setup's build of a fixture package at a version. */
export function fixtureBuild(
    name: FixturePackage,
    version = FIXTURE_VERSION,
): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), name, version), environment);
}
