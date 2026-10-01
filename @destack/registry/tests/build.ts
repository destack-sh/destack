import { cp, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { FIXTURE_RELEASES, FixtureSource, RegistryFixture } from "./registry.ts";

/** Build each fixture release into a directory by name and version, publishing each so that its dependents install it from a registry. */
async function build(builds: string): Promise<void> {
    const directory = await mkdtemp(join(tmpdir(), "destack-registry-sources-"));
    try {
        // copy every source and start its builder at once, since booting a builder takes most of its first build
        await using stack = new AsyncDisposableStack();
        const [registry, ...sources] = await Promise.all([
            RegistryFixture.open("sqlite"),
            ...FIXTURE_RELEASES.map(([name]) => FixtureSource.open(name, directory)),
        ]);
        stack.use(registry);
        sources.forEach((source) => stack.use(source));

        // keep and publish each version's build in dependency order
        for (const [index, source] of sources.entries()) {
            for (const build of await source.build(registry.npmrc(), FIXTURE_RELEASES[index]![1])) {
                stack.use(build);
                const version = build.manifest.package.version;
                await cp(build.directory, join(builds, source.name, version), { recursive: true });
                await registry.release(source.name, build);
            }
        }
    } finally {
        await rm(directory, { recursive: true });
    }
}

await build(process.argv[2]!);
