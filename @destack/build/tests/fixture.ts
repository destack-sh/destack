import { cp, mkdtemp, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { Buffer } from "node:buffer";
import { expect } from "@destack/test";
import { buildPackage, type BuildOptions, type PackageBuild } from "../src/index.ts";
import { readDependencies } from "../src/local/index.ts";
import { linkDependencies } from "../src/source/index.ts";
import { tmpdir } from "node:os";

/** Compare every distributed path and byte with the fixture's expected directory. */
export async function expectBuild(build: PackageBuild, expected: URL): Promise<void> {
    // include the manifest alongside the exact distributed bytes
    const files = new Map(build.files);
    files.set(
        "manifest.json",
        new TextEncoder().encode(`${JSON.stringify(build.manifest, null, 4)}\n`),
    );
    await expectDirectory(files, expected);
}

/** Compare a complete file collection with its expected directory. */
export async function expectDirectory(
    files: ReadonlyMap<string, Uint8Array>,
    expected: URL,
): Promise<void> {
    const directory = fileURLToPath(expected);
    const paths = [...files.keys()].sort();
    const update = process.env.UPDATE_BUILD_FIXTURES === "1";

    // refresh only the explicitly selected fixture output
    if (update) {
        await mkdir(directory, { recursive: true });
        const entries = await readdir(directory, { recursive: true, withFileTypes: true });
        for (const entry of entries) {
            if (!entry.isFile()) {
                continue;
            }
            const path = join(entry.parentPath, entry.name);
            if (!files.has(relative(directory, path).replaceAll("\\", "/"))) {
                await rm(path);
            }
        }
        for (const [path, bytes] of files) {
            const destination = join(directory, path);
            await mkdir(dirname(destination), { recursive: true });
            await writeFile(destination, bytes);
        }
    }

    // reject missing and unexpected files before comparing their contents
    const entries = await readdir(directory, { recursive: true, withFileTypes: true });
    const actual = entries
        .filter((entry) => entry.isFile())
        .map((entry) =>
            relative(directory, join(entry.parentPath, entry.name)).replaceAll("\\", "/"),
        )
        .sort();
    expect(actual).toEqual(paths);
    for (const path of paths) {
        const bytes = await readFile(join(directory, path));
        const expected = files.get(path)!;
        if (Buffer.compare(bytes, expected) !== 0) {
            expect(new Uint8Array(bytes), path).toEqual(expected);
        }
    }
}

/** Compare complete file sets without traversing each byte as an object property. */
export function expectFiles(
    actual: ReadonlyMap<string, Uint8Array>,
    expected: ReadonlyMap<string, Uint8Array>,
): void {
    expect([...actual.keys()]).toEqual([...expected.keys()]);
    for (const [path, bytes] of actual) {
        expect(Buffer.compare(bytes, expected.get(path)!), path).toBe(0);
    }
}

/** An isolated source checkout used by complete build fixtures. */
export class Fixture implements AsyncDisposable {
    /** Reuse immutable dependency records across cases for the same fixture. */
    static readonly #dependencies = new Map<string, ReturnType<typeof readDependencies>>();

    /** Retain copied source and its immutable dependencies. */
    private constructor(
        readonly fixture: URL,
        readonly directory: string,
        readonly source: string,
        readonly dependencies: BuildOptions["dependencies"],
    ) {}

    /** Copy a fixture and resolve its installed dependencies. */
    static async open(name: string): Promise<Fixture> {
        const fixture = new URL(`./fixture/${name}/`, import.meta.url);

        // share dependency preparation across concurrent cases without caching edited source
        let dependencies = Fixture.#dependencies.get(name);
        if (!dependencies) {
            dependencies = readDependencies(fileURLToPath(new URL("source/", fixture)));
            Fixture.#dependencies.set(name, dependencies);
        }
        const resolved = await dependencies;

        // copy each case into its own editable checkout
        const directory = await mkdtemp(join(tmpdir(), "destack-build-fixture-"));
        const source = join(directory, "source");
        try {
            await cp(new URL("source/", fixture), source, {
                recursive: true,
                filter: (path) =>
                    !path.endsWith("/node_modules") && !path.endsWith("\\node_modules"),
            });
            await linkDependencies(fileURLToPath(new URL("source/", fixture)), source);

            return new Fixture(fixture, directory, source, resolved);
        } catch (error) {
            await rm(directory, { recursive: true });
            throw error;
        }
    }

    /** Build the copied package with the fixture's output selection. */
    async build(request: Omit<BuildOptions, "directory" | "dependencies">): Promise<PackageBuild> {
        return await buildPackage({
            directory: this.source,
            dependencies: this.dependencies,
            ...request,
        });
    }

    /** Remove only this fixture's temporary checkout and distribution. */
    async [Symbol.asyncDispose](): Promise<void> {
        await rm(this.directory, { recursive: true });
    }
}
