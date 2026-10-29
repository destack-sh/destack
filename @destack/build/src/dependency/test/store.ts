import { dirname, join } from "node:path";
import type { FileSystem } from "@destack/environment";
import type { NpmRelease } from "@destack/package";

/** A package of a fixture Bun store: its manifest fields and files. */
export interface StoredPackage {
    /** The package.json fields beside its name and version. */
    readonly manifest?: Readonly<Record<string, unknown>>;
    /** Its files by path. */
    readonly files: Readonly<Record<string, string>>;
    /** The installed packages linked beside it. */
    readonly links?: readonly string[];
}

/** Write a Bun store of packages, each at `name@1.0.0`, answering its root and locked releases. */
export async function writeStore(
    packages: Readonly<Record<string, StoredPackage>>,
    files: FileSystem,
): Promise<{ root: string; locked: Record<string, NpmRelease> }> {
    // write each package's copy, then link its dependencies beside it as Bun's linker does
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-npm-"));
    const locked: Record<string, NpmRelease> = {};
    for (const [name, stored] of Object.entries(packages)) {
        const directory = join(root, `${name}@1.0.0`, "node_modules", name);
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(
            join(directory, "package.json"),
            JSON.stringify({ name, version: "1.0.0", ...stored.manifest }),
        );
        for (const [path, text] of Object.entries(stored.files)) {
            await files.mkdir(dirname(join(directory, path)), { recursive: true });
            await files.writeTextFile(join(directory, path), text);
        }
        for (const link of stored.links ?? []) {
            await files.symlink(
                join(root, `${link}@1.0.0`, "node_modules", link),
                join(root, `${name}@1.0.0`, "node_modules", link),
            );
        }
        locked[`${name}@1.0.0`] = {
            package: { name, version: "1.0.0" },
            registry: "https://registry.npmjs.org/",
            integrity: `sha512-${name}`,
        };
    }

    return { root, locked };
}

/** Find a package's copy in a fixture store. */
export function copy(root: string, name: string): string {
    return join(root, `${name}@1.0.0`, "node_modules", name);
}

/** Read a module load's failure as its runtime's code and message, or "loaded". */
export function moduleFailure(
    pending: Promise<unknown>,
): Promise<"loaded" | readonly [unknown, string]> {
    return pending.then(
        () => "loaded" as const,
        (error: unknown) => {
            // rethrow a failure without a code, failing the test that awaits it
            if (!(error instanceof Error && "code" in error)) {
                throw error;
            }

            return [error.code, error.message] as const;
        },
    );
}
