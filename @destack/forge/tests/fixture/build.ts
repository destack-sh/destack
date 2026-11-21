import { join } from "node:path";
import { testSimulation } from "@destack/object/test";
import { inject } from "@destack/test";
import { PackageBuild } from "@destack/build";
import { FIXTURE_VERSION, type FixturePackage, PRERELEASE_VERSION } from "./package.ts";

/** The environment the global setup's builds are read in, over the host's files. */
const environment = testSimulation().environment("builds");

/** Read the global setup's build of a fixture package at a version. */
export function fixtureBuild(
    name: FixturePackage,
    version = FIXTURE_VERSION,
): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), name, version), environment);
}

/** Read one of the global setup's dev prereleases of the answer package, the later by default. */
export function prereleaseBuild(version = PRERELEASE_VERSION): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), "answer", version), environment);
}
