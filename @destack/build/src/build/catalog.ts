import { join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import { Catalog, CatalogSource } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { CATALOG_DIRECTORY } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";

/** Compile a package's catalog sources by path, refusing other files, packages or misnamed ones. */
export async function compileCatalogs(
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
            throw BuildError.of(
                "CATALOG_INVALID",
                `catalog path '${path}' is invalid: it is not a catalog`,
            );
        }
        // compile its source to the catalog the build ships, each message's translation alone
        const text = await files.readTextFile(join(directory, path));
        let catalog: Catalog;
        try {
            const source = CatalogSource.parse(JSON.parse(text));
            catalog = Catalog.of(path, CatalogSource.compile(source), owner);
        } catch (cause) {
            throw BuildError.of(
                "CATALOG_INVALID",
                `catalog '${path}' is invalid: it does not parse`,
                { cause },
            );
        }
        catalogs.set(path, new TextEncoder().encode(JSON.stringify(catalog)));
    }

    return catalogs;
}

/** List a directory's entries, none for a missing directory. */
async function listDirectory(directory: string, files: FileSystem): Promise<string[]> {
    try {
        return (await files.readDirectory(directory)).map((entry) => entry.name);
    } catch (error) {
        if (!FileSystemError.is(error, "FILE_NOT_FOUND")) {
            throw error;
        }

        return [];
    }
}
