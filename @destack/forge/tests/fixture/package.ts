import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { CompilerPool, outputsOf, PackageBuild, type PackageBuildOptions } from "@destack/build";
import { readPackageJson } from "@destack/build/local";
import type { DependencyResolution } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { Commit, Digest, schema, Version } from "@destack/schema";
import { PackageArchive } from "../../src/pack/index.ts";
import { testHost } from "@destack/test/host";

/** How long one fixture command may run, in milliseconds: an install or a pack. */
const RUN_MILLISECONDS = 60_000;

/** The version the fixture sources declare. */
export const FIXTURE_VERSION = "2026.9.0";

/** The versions of the answer package the setup builds, in release order. */
export const ANSWER_VERSIONS = [FIXTURE_VERSION, "2026.9.1"];

/** The dev prerelease of the answer package after its releases, built without a commit. */
export const PRERELEASE_VERSION = "2026.9.2-dev.1";

/** The workspace's lockfile, which fixture copies link to beside them. */
const WORKSPACE_LOCKFILE = fileURLToPath(new URL("../../../../bun.lock", import.meta.url));

/** The commit every fixture release is built from. */
export const COMMIT = "a".repeat(40);

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

    /** Install the releases it depends on from their archives, and build each version on them. */
    async build(
        releases: readonly PackageBuild[],
        versions: readonly string[],
    ): Promise<PackageBuild[]> {
        // install each release from its npm archive, as an install from the registry unpacks it
        for (const release of releases) {
            await install(release, join(this.directory, "node_modules"));
        }
        const declaration = schema
            .jsonObject()
            .parse(await readJson(join(this.directory, "package.json")));
        const dependencies = await readReleases(this.directory, declaration);

        // build one unbundled module per runtime the package ships, as a build worker does
        const outputs = Object.fromEntries(
            Object.entries(outputsOf(await readPackageJson(this.directory, testHost().files))).map(
                ([name, { kind, runtime }]) => [name, { kind, runtime }],
            ),
        );

        // build each version
        const builds: PackageBuild[] = [];
        for (const version of versions) {
            await testHost().files.writeTextFile(
                join(this.directory, "package.json"),
                JSON.stringify({ ...declaration, version }),
            );
            builds.push(await this.#build({ outputs, dependencies, commit: Commit.parse(COMMIT) }));
        }
        this.#compiled = { outputs, dependencies };

        return builds;
    }

    /** Build a working tree without a commit: the last version, or a prerelease of the next. */
    async uncommitted(prerelease?: Version): Promise<PackageBuild> {
        if (this.#compiled === undefined) {
            throw new TypeError(`fixture ${this.name} has not been built`);
        }

        // build the last version again
        const { outputs, dependencies } = this.#compiled;
        if (prerelease === undefined) {
            return await this.#build({ outputs, dependencies });
        }

        // author the next version, and build its prerelease
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
    #build(request: Omit<PackageBuildOptions, "directory">): Promise<PackageBuild> {
        const options = { ...request, directory: this.directory };

        return this.#compilers.use(this.directory, (compiler) => compiler.build(options));
    }
}

/** Unpack a release's npm archive into a node_modules directory, as npm installs a tarball. */
async function install(release: PackageBuild, modules: string): Promise<void> {
    // pack the release as the registry serves it, and write the tarball beside the tree
    const host = testHost();
    const archive = await PackageArchive.pack(release.reader, COMMIT, host);
    const destination = join(modules, archive.metadata.name);
    await host.files.mkdir(destination, { recursive: true });
    const tarball = join(modules, `${archive.metadata.name.replace("/", "-")}.tgz`);
    await host.files.writeFile(tarball, await new Response(archive.open()).bytes());

    // extract its package directory, as every npm tarball roots its files in it
    await run(["tar", "-xzf", tarball, "-C", destination, "--strip-components=1"], modules);
    await host.files.remove(tarball);
}

/** Run a command in a directory, failing with its error output unless it succeeds. */
export async function run(command: readonly string[], directory: string): Promise<string> {
    // run the command, collecting both outputs
    const child = testHost().process.spawn(command, {
        workingDirectory: directory,
        stdout: "pipe",
        stderr: "pipe",
        signal: AbortSignal.timeout(RUN_MILLISECONDS),
    });
    const [code, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
        throw new Error(`${command.join(" ")} failed with ${code}: ${stderr}`);
    }

    return stdout;
}

/** Read a JSON file. */
export async function readJson(path: string): Promise<unknown> {
    return JSON.parse(await testHost().files.readTextFile(path));
}

/** Resolve the Destack releases a source installed from the forge by their build manifests. */
async function readReleases(
    directory: string,
    declaration: Readonly<Record<string, unknown>>,
): Promise<Record<string, DependencyResolution>> {
    // read each installed dependency's build manifest, which the forge's archive carries
    const declared = schema
        .record(schema.string(), schema.string())
        .parse(declaration["dependencies"] ?? {});
    const releases: Record<string, DependencyResolution> = {};
    for (const name of Object.keys(declared)) {
        const path = join(directory, "node_modules", name, "build", "manifest.json");
        const bytes = await testHost().files.readFile(path);
        const manifest = PackageManifest.parse(JSON.parse(new TextDecoder().decode(bytes)));
        releases[name] = {
            kind: "destack",
            package: PackageManifest.package(manifest),
            manifest: await Digest.of(bytes, testHost().crypto),
        };
    }

    return releases;
}
