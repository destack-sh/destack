import type { FileSystem } from "@destack/environment";
import { resolve } from "node:path";

/** The directories of installed dependencies and build outputs, as the repository ignores them. */
export const OUTPUT_DIRECTORIES: readonly string[] = [
    "node_modules",
    "dist",
    "target",
    "coverage",
    "out",
];

/** The extensions of the TypeScript modules a package keeps at its root. */
const MODULE_EXTENSIONS = [".ts", ".tsx"] as const;

/** A package's top-level sources: its folders, its root modules and the packages nested in it. */
export interface SourceRoots {
    /** The folders with its code, tests, scripts and other sources. */
    readonly folders: readonly string[];
    /** The TypeScript modules at its root, such as its configuration files. */
    readonly modules: readonly string[];
    /** The packages nested at its root, which check their own sources. */
    readonly packages: readonly string[];
}

/** List a package's top-level sources, passing over outputs, hidden entries and nested packages. */
export async function readSourceRoots(directory: string, files: FileSystem): Promise<SourceRoots> {
    // sort each visible entry that is no output into folders, nested packages and root modules
    const folders: string[] = [];
    const modules: string[] = [];
    const packages: string[] = [];
    for (const entry of await files.readDirectory(directory)) {
        const { name, kind } = entry;
        const isHidden = name.startsWith(".") || OUTPUT_DIRECTORIES.includes(name);

        // keep a nested package apart, else a folder of sources
        if (kind === "directory" && !isHidden) {
            const isPackage = await files.exists(resolve(directory, name, "package.json"));
            (isPackage ? packages : folders).push(name);
        }
        // keep a module at the root
        else if (
            kind === "file" &&
            MODULE_EXTENSIONS.some((extension) => name.endsWith(extension))
        ) {
            modules.push(name);
        }
    }

    return {
        folders: folders.toSorted(),
        modules: modules.toSorted(),
        packages: packages.toSorted(),
    };
}
