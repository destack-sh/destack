import { ids } from "@destack/account/test";
import { ForgeFixture } from "../src/test/index.ts";
import { testStore } from "@destack/build/test";
import { cp, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { ImageIndex } from "@destack/db/test";
import { TestRun } from "@destack/db/test/run";
import { nth, zip } from "@destack/schema";
import {
    FIXTURE_RELEASES,
    FixtureSource,
    PRERELEASE_VERSION,
    ACCOUNT_ID,
    openUniverse,
} from "./fixture/package.ts";

/** Build each fixture release into a directory, publishing it for its dependents. */
async function build(builds: string, images: ImageIndex): Promise<void> {
    const directory = await mkdtemp(join(tmpdir(), "destack-forge-sources-"));
    try {
        // replicate every source and start its builder at once, since booting a builder takes most of its first build
        await using stack = new AsyncDisposableStack();
        const [accounts, ...sources] = await Promise.all([
            openUniverse(images),
            ...FIXTURE_RELEASES.map(([name]) => FixtureSource.open(name, directory)),
        ]);
        stack.use(accounts);
        const forge = stack.use(
            await ForgeFixture.create(accounts, { account: ACCOUNT_ID, user: ids.owner }),
        );
        sources.forEach((source) => stack.use(source));

        // build each version into the local store, then publish it in dependency order
        const store = stack.use(await testStore());
        for (const [source, [, versions]] of zip(sources, FIXTURE_RELEASES)) {
            for (const built of await source.build(forge.npmrc("@example"), versions, store)) {
                stack.use(built);
                const version = built.manifest.package.version;
                await cp(built.directory, join(builds, source.name, version), { recursive: true });
                await forge.publish(built.reader);
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

/** The directory the builds go into and the run's template image index, the arguments. */
const [, , destination, index] = process.argv;
if (destination === undefined || index === undefined) {
    throw new TypeError("missing builds directory or template index argument");
}
await build(destination, await TestRun.read(index));
