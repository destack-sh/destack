import { readFile, stat, symlink } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { BuildError } from "../error/index.ts";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { DependencyName, DependencyResolution } from "@destack/package/package";
import { schema } from "@destack/schema";

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

/** Run the installed Deno toolchain. */
const execute = promisify(execFile);

/** Deno's installed npm package inventory. */
const DenoInventory = schema
    .object({
        version: schema.literal(1),
        npmPackages: schema.record(
            schema.string(),
            schema
                .object({
                    name: schema.string(),
                    version: schema.string(),
                    registryUrl: schema.string(),
                })
                .strip(),
        ),
    })
    .strip();

/** Read installed npm releases from Deno's inventory and lockfile. */
export async function readDependencies(
    directory: string,
): Promise<Record<string, DependencyResolution>> {
    // locate the lockfile without changing the selected releases
    directory = resolve(directory);
    const lock = await readLockfile(directory);
    const result = await execute(
        "deno",
        ["info", "--frozen-lockfile", "--node-modules-dir=none", "--json", "package.json"],
        {
            cwd: lock.directory,
            timeout: 30_000,
            maxBuffer: 32 * 1024 * 1024,
        },
    );
    const inventory = DenoInventory.parse(JSON.parse(result.stdout));
    const dependencies: Record<string, DependencyResolution> = {};

    // combine registry locations with locked release integrity
    for (const [id, entry] of Object.entries(inventory.npmPackages)) {
        const key = `${entry.name}@${entry.version}`;
        if (dependencies[key]) {
            continue;
        }
        const locked = lock.npm[id];
        if (!locked) {
            throw new BuildError("BUILD_FAILED", `Unlocked dependency: ${id}`);
        }
        dependencies[key] = DependencyResolution.parse({
            kind: "npm",
            package: { name: entry.name, version: entry.version },
            registry: entry.registryUrl,
            integrity: locked.integrity,
        });
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

/** Read the nearest Deno lockfile used by the local project. */
async function readLockfile(directory: string): Promise<{
    directory: string;
    npm: Record<string, { integrity: string }>;
}> {
    while (true) {
        let text: string;
        try {
            text = await readFile(join(directory, "deno.lock"), "utf8");
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
            const parent = dirname(directory);
            if (parent === directory) {
                throw new BuildError("BUILD_FAILED", "No Deno lockfile.");
            }
            directory = parent;
            continue;
        }
        const lock = JSON.parse(text);
        if (lock.version !== "5") {
            throw new BuildError(
                "BUILD_FAILED",
                `Unsupported Deno lockfile version: ${lock.version}`,
            );
        }

        return { directory, npm: lock.npm ?? {} };
    }
}
