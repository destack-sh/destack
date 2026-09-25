import { existsSync, readFileSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { DeclarationConstructorMap } from "../definition/constructor.ts";

/** Declaration constructors read so far, by package directory. */
const DECLARED = new Map<string, DeclarationConstructorMap>();

/** Package directories resolved so far, by requesting directory and package name. */
const RESOLVED = new Map<string, string | undefined>();

/** Read the declaration constructors a package declares, resolving it by name from a directory. */
export function packageConstructors(name: string, directory: string): DeclarationConstructorMap {
    // find the package itself or its installation above the directory
    const key = `${directory}\0${name}`;
    if (!RESOLVED.has(key)) {
        RESOLVED.set(key, resolvePackage(name, directory));
    }
    const root = RESOLVED.get(key);
    if (root === undefined) {
        return {};
    }

    // read and cache the constructors its destack.json declares
    let declared = DECLARED.get(root);
    if (!declared) {
        const path = join(root, "destack.json");
        const definition = existsSync(path) ? JSON.parse(readFileSync(path, "utf8")) : {};
        declared = DeclarationConstructorMap.parse(definition.declarations ?? {});
        DECLARED.set(root, declared);
    }

    return declared;
}

/** Find a package's directory: an ancestor named like it, or its installation in node_modules. */
function resolvePackage(name: string, directory: string): string | undefined {
    for (let current = directory; ; current = dirname(current)) {
        // accept the package containing the directory
        const manifest = join(current, "package.json");
        if (existsSync(manifest) && JSON.parse(readFileSync(manifest, "utf8")).name === name) {
            return current;
        }

        // accept an installation of the package
        const installed = join(current, "node_modules", name);
        if (existsSync(join(installed, "package.json"))) {
            return realpathSync(installed);
        }

        // stop at the filesystem root
        if (dirname(current) === current) {
            return undefined;
        }
    }
}
