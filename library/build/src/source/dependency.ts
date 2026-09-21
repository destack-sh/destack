import { readFile, stat, symlink } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { BuildError } from "../error/index.ts";
import { DependencyName, DependencyResolution } from "@destack/package/package";
import { JSONC } from "bun";
import { schema } from "@destack/schema";

/** Bun's versioned package resolution records. */
const BunLockfile = schema
    .object({
        lockfileVersion: schema.literal(2),
        packages: schema.record(
            schema.string(),
            schema.tuple([
                schema.string(),
                schema.string().optional(),
                schema.json().optional(),
                schema.string().optional(),
            ]),
        ),
    })
    .strip();

/** Link the nearest installed dependency directory into an isolated compiler directory. */
export async function linkDependencies(source: string, destination: string): Promise<void> {
    let directory = source;
    while (true) {
        // preserve the package manager's installed dependency tree
        const path = join(directory, "node_modules");
        let exists = false;
        try {
            exists = (await stat(path)).isDirectory();
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
        }
        if (exists) {
            await symlink(path, join(destination, "node_modules"), "junction");

            return;
        }

        // search the enclosing workspace when the package has no local installation
        const parent = dirname(directory);
        if (parent === directory) {
            return;
        }
        directory = parent;
    }
}

/** Locate the named package containing a resolved module. */
export async function modulePackage(
    directory: string,
): Promise<{ directory: string; name: string; version: string }> {
    while (true) {
        try {
            const metadata = JSON.parse(await readFile(join(directory, "package.json"), "utf8"));
            if (typeof metadata.name === "string" && typeof metadata.version === "string") {
                return { directory, name: metadata.name, version: metadata.version };
            }
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
        }
        const parent = dirname(directory);
        if (parent === directory) {
            throw new BuildError("BUILD_FAILED", `No package declaration for module: ${directory}`);
        }
        directory = parent;
    }
}

/** Read installed npm releases from Bun's lockfile. */
export async function readDependencies(
    directory: string,
): Promise<Record<string, DependencyResolution>> {
    // locate the lockfile without changing the selected releases
    directory = resolve(directory);
    const lock = await readLockfile(directory);
    const dependencies: Record<string, DependencyResolution> = {};

    // combine registry locations with locked release integrity
    for (const [id, entry] of Object.entries(lock.packages)) {
        const [release, location, , integrity] = entry;
        const separator = release.lastIndexOf("@");
        const name = release.slice(0, separator);
        const version = release.slice(separator + 1);
        if (version.startsWith("workspace:")) {
            continue;
        }
        if (separator < 1 || !integrity) {
            throw new BuildError("BUILD_FAILED", `unsupported locked dependency: ${id}`);
        }

        // Bun stores the full tarball URL for non-default registries
        const suffix = `/${name}/-/`;
        const position = location?.indexOf(suffix) ?? -1;
        if (location && position < 0) {
            throw new BuildError("BUILD_FAILED", `unsupported registry location: ${location}`);
        }
        const registry = location ? location.slice(0, position + 1) : "https://registry.npmjs.org/";
        const key = `${name}@${version}`;
        const resolution = DependencyResolution.parse({
            kind: "npm",
            package: { name, version },
            registry,
            integrity,
        });
        const existing = dependencies[key];
        if (existing && JSON.stringify(existing) !== JSON.stringify(resolution)) {
            throw new BuildError("BUILD_FAILED", `conflicting locked dependency: ${key}`);
        }
        dependencies[key] = resolution;
    }

    // associate authored import names, including npm aliases, with their installed releases
    const declaration = JSON.parse(await readFile(join(directory, "package.json"), "utf8"));
    const names = new Set(
        Object.keys({
            ...declaration.dependencies,
            ...declaration.peerDependencies,
            ...declaration.optionalDependencies,
        }),
    );
    for (const name of names) {
        DependencyName.parse(name);
        const installed = await readInstalledPackage(name, directory);
        if (!installed) {
            continue;
        }
        const release = dependencies[`${installed.name}@${installed.version}`];
        if (release) {
            dependencies[name] = release;
        }
    }

    return dependencies;
}

/** Locate an installed dependency using Node's ancestor-directory lookup. */
async function readInstalledPackage(
    name: string,
    directory: string,
): Promise<
    | {
          name: string;
          version: string;
      }
    | undefined
> {
    while (true) {
        try {
            return JSON.parse(
                await readFile(join(directory, "node_modules", name, "package.json"), "utf8"),
            );
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
            const parent = dirname(directory);
            if (parent === directory) {
                return undefined;
            }
            directory = parent;
        }
    }
}

/** Read the nearest Bun lockfile used by the source package. */
async function readLockfile(directory: string): Promise<{
    packages: Record<string, [string, string?, unknown?, string?]>;
}> {
    while (true) {
        let text: string;
        try {
            text = await readFile(join(directory, "bun.lock"), "utf8");
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
            const parent = dirname(directory);
            if (parent === directory) {
                throw new BuildError("BUILD_FAILED", "no Bun lockfile");
            }
            directory = parent;
            continue;
        }
        return BunLockfile.parse(JSONC.parse(text));
    }
}
