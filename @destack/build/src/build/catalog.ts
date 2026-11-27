import { join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import { Catalog, CatalogSource } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { CATALOG_DIRECTORY } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";

/** Compile a package's catalogs by path, keeping compiled ones and refusing misnamed ones. */
export async function compileCatalogs(
    directory: string,
    owner: PackageId,
    files: FileSystem,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // list the catalog directory's files and its bundled dependencies' catalogs, which a package without catalogs lacks
    const names = await listCatalogs(join(directory, CATALOG_DIRECTORY), files);
    const catalogs = new Map<string, Uint8Array<ArrayBuffer>>();
    for (const name of names.toSorted()) {
        // require a catalog file: the package's own, or a bundled dependency's in a build
        const path = `${CATALOG_DIRECTORY}/${name}`;
        if (!Catalog.isPath(path)) {
            throw BuildError.of(
                "CATALOG_INVALID",
                `catalog path '${path}' is invalid: it is not a catalog`,
            );
        }
        // compile a source to the catalog the build ships, keeping a catalog a build compiled already
        const text = await files.readTextFile(join(directory, path));
        let catalog: Catalog;
        try {
            const parsed: unknown = JSON.parse(text);
            const compiled = Catalog.safeParse(parsed);
            catalog = compiled.success
                ? Catalog.of(path, compiled.data, owner)
                : Catalog.of(path, CatalogSource.compile(CatalogSource.parse(parsed)), owner);
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

/** List a catalog directory's files and its dependencies' files, none for a missing one. */
async function listCatalogs(directory: string, files: FileSystem): Promise<string[]> {
    // read the directory, which a package without catalogs lacks
    let entries: Awaited<ReturnType<FileSystem["readDirectory"]>>;
    try {
        entries = await files.readDirectory(directory);
    } catch (error) {
        if (!FileSystemError.is(error, "FILE_NOT_FOUND")) {
            throw error;
        }

        return [];
    }

    // take each file, and each file of a dependency's directory below its name
    const names = await Promise.all(
        entries.map(async (entry) =>
            entry.kind === "directory"
                ? (await files.readDirectory(join(directory, entry.name))).map(
                      (nested) => `${entry.name}/${nested.name}`,
                  )
                : [entry.name],
        ),
    );

    return names.flat();
}
