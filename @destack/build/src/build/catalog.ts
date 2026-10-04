import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { Catalog } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { BuildError, isMissing } from "../error/index.ts";

/** The directory of a package keeping its catalogs. */
const CATALOG_DIRECTORY = "locale";

/** Read a package's catalogs by package path, refusing another file, another package's catalog or a misnamed one. */
export async function readCatalogs(
    directory: string,
    owner: PackageId,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // list the catalog directory, which a package without catalogs lacks
    const names = await listDirectory(join(directory, CATALOG_DIRECTORY));
    const catalogs = new Map<string, Uint8Array<ArrayBuffer>>();
    for (const name of names.toSorted()) {
        // require a catalog file of the package's own messages
        const path = `${CATALOG_DIRECTORY}/${name}`;
        if (!Catalog.isPath(path)) {
            throw new BuildError("BUILD_FAILED", `not a catalog: ${path}`);
        }
        const bytes = new Uint8Array(await readFile(join(directory, path)));
        try {
            Catalog.of(path, JSON.parse(new TextDecoder().decode(bytes)), owner);
        } catch (cause) {
            throw new BuildError("BUILD_FAILED", `invalid catalog: ${path}`, { cause });
        }
        catalogs.set(path, bytes);
    }

    return catalogs;
}

/** List a directory's entries, none for a missing directory. */
async function listDirectory(directory: string): Promise<string[]> {
    try {
        return await readdir(directory);
    } catch (error) {
        if (!isMissing(error)) {
            throw error;
        }

        return [];
    }
}
