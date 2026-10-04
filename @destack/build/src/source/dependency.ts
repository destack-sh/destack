import { readFile, stat, symlink } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { BuildError, isMissing } from "../error/index.ts";
import { DependencyName, DependencyResolution } from "@destack/package";
import { BunLockfile, type LockedPackage } from "@destack/check/bun";
import { schema } from "@destack/schema";

/** The dependencies a package.json declares. */
const DependencyManifest = schema.looseObject({
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    peerDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    optionalDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
});

/** The release an installed package.json names, which an unreleased workspace package lacks. */
const InstalledPackage = schema.looseObject({
    name: schema.string(),
    version: schema.string().exactOptional(),
});
/** The release an installed package.json names. */
type InstalledPackage = schema.Infer<typeof InstalledPackage>;

/** The release a package.json names, absent from a subpath manifest that only configures modules. */
const ReleasedPackage = schema.looseObject({ name: schema.string(), version: schema.string() });

/** The lockfile format version whose package records dependency resolution reads. */
const LOCKFILE_VERSION = 2;

/** Link the nearest installed dependency directory into an isolated compiler directory. */
export async function linkDependencies(source: string, destination: string): Promise<void> {
    // link the nearest installed dependency tree above the package
    for (const directory of ancestors(source)) {
        const path = join(directory, "node_modules");
        if (await isDirectory(path)) {
            await symlink(path, join(destination, "node_modules"), "junction");

            return;
        }
    }
}

/** Locate the named package containing a resolved module. */
export async function modulePackage(
    directory: string,
): Promise<{ directory: string; name: string; version: string }> {
    // find the nearest manifest naming a release, passing subpath manifests that configure module scopes
    for (const current of ancestors(directory)) {
        const metadata = ReleasedPackage.safeParse(await readJson(join(current, "package.json")));
        if (metadata.success) {
            return { directory: current, name: metadata.data.name, version: metadata.data.version };
        }
    }

    throw new BuildError("BUILD_FAILED", `no package declaration for module: ${directory}`);
}

/** Read installed npm releases from Bun's lockfile. */
export async function readDependencies(
    directory: string,
): Promise<Record<string, DependencyResolution>> {
    // read the authored dependency names, and resolve none for a package that declares none
    directory = resolve(directory);
    const declaration = DependencyManifest.parse(
        JSON.parse(await readFile(join(directory, "package.json"), "utf8")),
    );
    const names = new Set(
        Object.keys({
            ...declaration.dependencies,
            ...declaration.peerDependencies,
            ...declaration.optionalDependencies,
        }),
    );
    if (names.size === 0) {
        return {};
    }

    // locate the lockfile without changing the selected releases
    const lock = await readLockfile(directory);
    const dependencies: Record<string, DependencyResolution> = {};

    // combine registry locations with locked release integrity
    for (const [id, entry] of Object.entries(lock.packages)) {
        const resolution = readLockedRelease(id, entry);
        if (resolution === undefined) {
            continue;
        }
        const key = `${resolution.package.name}@${resolution.package.version}`;
        const existing = dependencies[key];
        if (existing && JSON.stringify(existing) !== JSON.stringify(resolution)) {
            throw new BuildError("BUILD_FAILED", `conflicting locked dependency: ${key}`);
        }
        dependencies[key] = resolution;
    }

    // associate authored import names, including npm aliases, with their installed releases
    for (const name of names) {
        DependencyName.parse(name);
        const installed = await readInstalledPackage(name, directory);
        if (installed?.version === undefined) {
            continue;
        }
        const release = dependencies[`${installed.name}@${installed.version}`];
        if (release) {
            dependencies[name] = release;
        }
    }

    return dependencies;
}

/** Read a locked registry release, absent for a workspace package. */
function readLockedRelease(id: string, entry: LockedPackage): DependencyResolution | undefined {
    // name the release, leaving workspace packages to their sources
    const [release, location, , integrity] = entry;
    const separator = release.indexOf("@", 1);
    const name = release.slice(0, separator);
    const version = release.slice(separator + 1);
    if (version.startsWith("workspace:")) {
        return undefined;
    }
    if (separator < 1 || integrity === undefined || integrity === "") {
        throw new BuildError("BUILD_FAILED", `unsupported locked dependency: ${id}`);
    }

    // read the registry from the full tarball URL that Bun stores for other registries
    const suffix = `/${name}/-/`;
    let registry = "https://registry.npmjs.org/";
    if (location !== undefined && location !== "") {
        const position = location.indexOf(suffix);
        if (position < 0) {
            throw new BuildError("BUILD_FAILED", `unsupported registry location: ${location}`);
        }
        registry = location.slice(0, position + 1);
    }

    return DependencyResolution.parse({
        kind: "npm",
        package: { name, version },
        registry,
        integrity,
    });
}

/** Locate an installed dependency using Node's ancestor-directory lookup. */
async function readInstalledPackage(
    name: string,
    directory: string,
): Promise<InstalledPackage | undefined> {
    // read the nearest installed manifest of the dependency
    for (const current of ancestors(directory)) {
        const manifest = await readJson(join(current, "node_modules", name, "package.json"));
        if (manifest !== undefined) {
            return InstalledPackage.parse(manifest);
        }
    }

    return undefined;
}

/** Name a file by its path below a directory, with forward slashes. */
export function relativePath(directory: string, file: string): string {
    return relative(directory, file).split(sep).join("/");
}

/** Report whether a compiler file is one of the package's modules. */
export function isAuthored(root: string, file: string): boolean {
    return contains(root, file) && !relative(root, file).split(sep).includes("node_modules");
}

/** Report whether a file lies inside a directory. */
function contains(directory: string, file: string): boolean {
    const path = relative(directory, file);

    return path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path);
}

/** Read the nearest Bun lockfile used by the source package, refusing other formats. */
async function readLockfile(directory: string): Promise<BunLockfile> {
    // require the nearest lockfile in the format the package records follow
    const lockfile = await BunLockfile.read(directory);
    if (lockfile === undefined) {
        throw new BuildError("BUILD_FAILED", "no Bun lockfile");
    }
    if (lockfile.version !== LOCKFILE_VERSION) {
        throw new BuildError(
            "BUILD_FAILED",
            `unsupported Bun lockfile version: ${lockfile.version}`,
        );
    }

    return lockfile;
}

/** List a directory and each directory above it, up to the filesystem root. */
function* ancestors(directory: string): Generator<string> {
    for (let current = resolve(directory); ; current = dirname(current)) {
        yield current;
        if (dirname(current) === current) {
            return;
        }
    }
}

/** Read a file's text, absent for a missing file. */
async function readText(path: string): Promise<string | undefined> {
    try {
        return await readFile(path, "utf8");
    } catch (error) {
        if (!isMissing(error)) {
            throw error;
        }

        return undefined;
    }
}

/** Read a JSON file, absent for a missing file. */
async function readJson(path: string): Promise<unknown> {
    const text = await readText(path);

    return text === undefined ? undefined : JSON.parse(text);
}

/** Report whether a path is an existing directory. */
async function isDirectory(path: string): Promise<boolean> {
    try {
        return (await stat(path)).isDirectory();
    } catch (error) {
        if (!isMissing(error)) {
            throw error;
        }

        return false;
    }
}
