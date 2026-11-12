import { join } from "node:path";
import type { BuildReader } from "@destack/package/manifest";
import { fileURLToPath } from "node:url";
import { CompilerPool, outputsOf, type PackageBuildOptions } from "@destack/build";
import { readPackageJson } from "@destack/build/local";
import type { DependencyRelease } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { Digest, schema, Version } from "@destack/schema";
import { testHost } from "@destack/test/host";

/** The version the fixture sources declare. */
export const FIXTURE_VERSION = "2026.9.0";

/** The versions of the answer package the setup builds, in release order. */
export const ANSWER_VERSIONS = [FIXTURE_VERSION, "2026.9.1"];

/** The dev prerelease of the answer package after its releases. */
export const PRERELEASE_VERSION = "2026.9.2-dev.20260912100000.0123456789ab";

/** The workspace's lockfile, which fixture copies link to beside them. */
const WORKSPACE_LOCKFILE = fileURLToPath(new URL("../../../../bun.lock", import.meta.url));

/** The fixture packages: real package sources under tests/fixture called `@example/<name>`. */
export type FixturePackage = "answer" | "greeting";

/** A copy of a fixture package's source, built on the run's compilers into the persistent store. */
export class FixtureSource {
    /** The fixture package. */
    readonly name: FixturePackage;
    /** The directory with the copy, beside a link to the workspace's lockfile. */
    readonly directory: string;
    /** The compilers building the copy into the store. */
    readonly #compilers: CompilerPool;
    /** The outputs and dependencies the last build compiled. */
    #compiled: Pick<PackageBuildOptions, "outputs" | "dependencies"> | undefined;

    /** Take a copy and the compilers building it. */
    private constructor(name: FixturePackage, directory: string, compilers: CompilerPool) {
        this.name = name;
        this.directory = directory;
        this.#compilers = compilers;
    }

    /** Copy a fixture package's source below a directory, beside the workspace's lockfile. */
    static async open(
        name: FixturePackage,
        parent: string,
        compilers: CompilerPool,
    ): Promise<FixtureSource> {
        // copy the source, and link the workspace's lockfile beside it as the compilers locate workspaces by it
        const directory = join(parent, name, "source");
        const { files } = testHost();
        await files.copy(fileURLToPath(new URL(`./${name}/`, import.meta.url)), directory);
        await files.symlink(WORKSPACE_LOCKFILE, join(parent, name, "bun.lock"));

        return new FixtureSource(name, directory, compilers);
    }

    /** Build each version on the releases it depends on, which the store has. */
    async build(
        releases: readonly BuildReader[],
        versions: readonly string[],
    ): Promise<BuildReader[]> {
        // resolve each release it declares by its manifest, as a build outside the workspace does
        const declaration = schema
            .jsonObject()
            .parse(await readJson(join(this.directory, "package.json")));
        const dependencies = await resolveReleases(releases, declaration);

        // build one unbundled module per runtime the package ships, as a build worker does
        const outputs = Object.fromEntries(
            Object.entries(outputsOf(await readPackageJson(this.directory, testHost().files))).map(
                ([name, { kind, runtime }]) => [name, { kind, runtime }],
            ),
        );

        // build each version
        const builds: BuildReader[] = [];
        for (const version of versions) {
            await testHost().files.writeTextFile(
                join(this.directory, "package.json"),
                JSON.stringify({ ...declaration, version }),
            );
            builds.push(await this.#build({ outputs, dependencies }));
        }
        this.#compiled = { outputs, dependencies };

        return builds;
    }

    /** Build a prerelease of the next version. */
    async prerelease(prerelease: Version): Promise<BuildReader> {
        if (this.#compiled === undefined) {
            throw new TypeError(`fixture '${this.name}' has not been built`);
        }

        // author the next version, and build its prerelease
        const { outputs, dependencies } = this.#compiled;
        const path = join(this.directory, "package.json");
        const declaration = schema.jsonObject().parse(await readJson(path));
        const version = Version.next(Version.parse(declaration["version"]));
        await testHost().files.writeTextFile(path, JSON.stringify({ ...declaration, version }));
        const name = schema.string().parse(declaration["name"]);

        return await this.#build({
            outputs,
            dependencies,
            versions: { [name]: prerelease },
        });
    }

    /** Build the copy with a warm compiler of the pool. */
    #build(request: Omit<PackageBuildOptions, "directory">): Promise<BuildReader> {
        const options = { ...request, directory: this.directory };

        return this.#compilers.use(this.directory, (compiler) => compiler.build(options));
    }
}

/** Read a JSON file. */
export async function readJson(path: string): Promise<unknown> {
    return JSON.parse(await testHost().files.readTextFile(path));
}

/** Resolve each Destack dependency a declaration lists to the release built of it. */
async function resolveReleases(
    releases: readonly BuildReader[],
    declaration: Readonly<Record<string, unknown>>,
): Promise<Record<string, DependencyRelease>> {
    // find each declared dependency's release, and resolve it by its manifest's digest
    const declared = schema
        .record(schema.string(), schema.string())
        .parse(declaration["dependencies"] ?? {});
    const resolved: Record<string, DependencyRelease> = {};
    for (const name of Object.keys(declared)) {
        const release = releases.find((each) => each.manifest.package.name === name);
        if (release === undefined) {
            throw new TypeError(`fixture has no release of '${name}'`);
        }
        const bytes = PackageManifest.serialize(release.manifest);
        resolved[name] = {
            kind: "destack",
            package: PackageManifest.package(release.manifest),
            manifest: await Digest.of(bytes, testHost().crypto),
        };
    }

    return resolved;
}
