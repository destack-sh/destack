import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import type { NpmRelease } from "@destack/package";
import { onTestFinished } from "@destack/test";
import { NpmConversion } from "../../compile/conversion.ts";
import { type NpmConverter, NpmUnit } from "../npm.ts";

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
): Promise<{ root: string; locked: Record<string, NpmRelease> }> {
    // write each package's copy, then link its dependencies beside it as Bun's linker does
    const root = await realpath(await mkdtemp(join(tmpdir(), "destack-npm-")));
    onTestFinished(() => rm(root, { recursive: true }));
    const locked: Record<string, NpmRelease> = {};
    for (const [name, stored] of Object.entries(packages)) {
        const directory = join(root, `${name}@1.0.0`, "node_modules", name);
        await mkdir(directory, { recursive: true });
        await writeFile(
            join(directory, "package.json"),
            JSON.stringify({ name, version: "1.0.0", ...stored.manifest }),
        );
        for (const [path, text] of Object.entries(stored.files)) {
            await mkdir(dirname(join(directory, path)), { recursive: true });
            await writeFile(join(directory, path), text);
        }
        for (const link of stored.links ?? []) {
            await symlink(
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

/** Convert an npm package in this process, as the compiler does. */
export const convertHere: NpmConverter = async (request) => {
    // convert into a separate directory of the test, then read the converted copy
    const destination = await mkdtemp(join(tmpdir(), "destack-npm-convert-"));
    onTestFinished(() => rm(destination, { recursive: true }));
    const conversion = new NpmConversion(request.name, request.dependencies, request.peers);
    const imported = await conversion.convert(request.source, destination);

    return { files: await NpmUnit.files(destination), imported: new Set(imported) };
};

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
