import { existsSync } from "node:fs";
import { dirname, join } from "node:path";

/** Package roots found above source directories, cached by directory. */
const roots = new Map<string, boolean>();

/** Report whether a source file belongs to a Destack package, marked by a destack.json above it. */
export function isPackageFile(path: string): boolean {
    return isInPackage(dirname(path));
}

/** Report whether a directory or one of its parents holds destack.json. */
function isInPackage(directory: string): boolean {
    // reuse earlier answers for shared parent directories
    const cached = roots.get(directory);
    if (cached !== undefined) {
        return cached;
    }

    // stop at dependency installations and the filesystem root
    const parent = dirname(directory);
    const isPackage =
        !directory.endsWith("node_modules") &&
        (existsSync(join(directory, "destack.json")) ||
            (parent !== directory && isInPackage(parent)));
    roots.set(directory, isPackage);

    return isPackage;
}
