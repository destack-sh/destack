import { cp, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { nth, zip } from "@destack/schema";
import {
    FIXTURE_RELEASES,
    FixtureSource,
    PackageFixture,
    PRERELEASE_VERSION,
} from "./fixture/package.ts";

/** Build each fixture release into a directory, publishing it for its dependents. */
async function build(builds: string): Promise<void> {
    const directory = await mkdtemp(join(tmpdir(), "destack-forge-sources-"));
    try {
        // replicate every source and start its builder at once, since booting a builder takes most of its first build
        await using stack = new AsyncDisposableStack();
        const [forge, ...sources] = await Promise.all([
            PackageFixture.open(),
            ...FIXTURE_RELEASES.map(([name]) => FixtureSource.open(name, directory)),
        ]);
        stack.use(forge);
        sources.forEach((source) => stack.use(source));

        // store and publish each version's build in dependency order
        for (const [source, [, versions]] of zip(sources, FIXTURE_RELEASES)) {
            for (const built of await source.build(forge.npmrc(), versions)) {
                stack.use(built);
                const version = built.manifest.package.version;
                await cp(built.directory, join(builds, source.name, version), { recursive: true });
                await forge.publish(built);
            }
        }

        // store the answer package's last version and a dev prerelease compiled without a commit
        const answer = nth(sources, 0);
        await using uncommitted = await answer.uncommitted();
        await cp(uncommitted.directory, join(builds, answer.name, "uncommitted"), {
            recursive: true,
        });
        await using prerelease = await answer.uncommitted(PRERELEASE_VERSION);
        await cp(prerelease.directory, join(builds, answer.name, PRERELEASE_VERSION), {
            recursive: true,
        });
    } finally {
        await rm(directory, { recursive: true });
    }
}

/** The directory the builds go into, the first argument. */
const [, , destination] = process.argv;
if (destination === undefined) {
    throw new TypeError("missing builds directory argument");
}
await build(destination);

// end the process, which releases the schemas claimed on the test PostgreSQL server until then
process.exit(0);
