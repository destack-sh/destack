import { join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import { Catalog } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { BuildError } from "../error/index.ts";

/** The directory of a package that stores its catalogs. */
const CATALOG_DIRECTORY = "locale";

/** Read a package's catalogs by path, refusing other files, packages or misnamed catalogs. */
export async function readCatalogs(
    directory: string,
    owner: PackageId,
    files: FileSystem,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // list the catalog directory, which a package without catalogs lacks
    const names = await listDirectory(join(directory, CATALOG_DIRECTORY), files);
    const catalogs = new Map<string, Uint8Array<ArrayBuffer>>();
    for (const name of names.toSorted()) {
        // require a catalog file of the package's messages
        const path = `${CATALOG_DIRECTORY}/${name}`;
        if (!Catalog.isPath(path)) {
            throw BuildError.of("CATALOG_INVALID", `not a catalog: ${path}`);
        }
        const bytes = await files.readFile(join(directory, path));
        try {
            Catalog.of(path, JSON.parse(new TextDecoder().decode(bytes)), owner);
        } catch (cause) {
            throw BuildError.of("CATALOG_INVALID", `invalid catalog: ${path}`, { cause });
        }
        catalogs.set(path, bytes);
    }

    return catalogs;
}

/** List a directory's entries, none for a missing directory. */
async function listDirectory(directory: string, files: FileSystem): Promise<string[]> {
    try {
        return (await files.readDirectory(directory)).map((entry) => entry.name);
    } catch (error) {
        if (!FileSystemError.is(error, "NOT_FOUND")) {
            throw error;
        }

        return [];
    }
}
