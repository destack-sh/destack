import { join } from "node:path";
import { inject } from "@destack/test";
import { PackageBuild } from "@destack/build";
import { FIXTURE_VERSION, type FixturePackage, PRERELEASE_VERSION } from "./package.ts";

/** Read the global setup's build of a fixture package at a version. */
export function fixtureBuild(
    name: FixturePackage,
    version = FIXTURE_VERSION,
): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), name, version));
}

/** Read the global setup's build of the answer package's last version compiled without a commit. */
export function uncommittedBuild(): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), "answer", "uncommitted"));
}

/** Read the global setup's dev prerelease of the answer package, compiled without a commit. */
export function prereleaseBuild(): Promise<PackageBuild> {
    return PackageBuild.read(join(inject("builds"), "answer", PRERELEASE_VERSION));
}
