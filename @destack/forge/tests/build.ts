import { cp, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { aligned, zip } from "@destack/schema";
import { FIXTURE_RELEASES, FixtureSource, PackageFixture } from "./fixture/package.ts";

/** Build each fixture release into a directory by name and version, publishing each so that its dependents install it from a forge. */
async function build(builds: string): Promise<void> {
    const directory = await mkdtemp(join(tmpdir(), "destack-forge-sources-"));
    try {
        // copy every source and start its builder at once, since booting a builder takes most of its first build
        await using stack = new AsyncDisposableStack();
        const [forge, ...sources] = await Promise.all([
            PackageFixture.open("sqlite"),
            ...FIXTURE_RELEASES.map(([name]) => FixtureSource.open(name, directory)),
        ]);
        stack.use(forge);
        sources.forEach((source) => stack.use(source));

        // keep and publish each version's build in dependency order
        for (const [source, [, versions]] of zip(sources, FIXTURE_RELEASES)) {
            for (const built of await source.build(forge.npmrc(), versions)) {
                stack.use(built);
                const version = built.manifest.package.version;
                await cp(built.directory, join(builds, source.name, version), { recursive: true });
                await forge.release(source.name, built);
            }
        }

        // keep the answer package's last version compiled without a commit
        const answer = aligned(sources, 0);
        await using uncommitted = await answer.uncommitted();
        await cp(uncommitted.directory, join(builds, answer.name, "uncommitted"), {
            recursive: true,
        });
    } finally {
        await rm(directory, { recursive: true });
    }
}

/** The directory the builds go into, as the first argument names it. */
const [, , destination] = process.argv;
if (destination === undefined) {
    throw new TypeError("missing builds directory argument");
}
await build(destination);

// end the process, which releases the schemas the test PostgreSQL server keeps claimed until then
process.exit(0);
