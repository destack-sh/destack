import { PackageHandle } from "@destack/runtime";
import { nth } from "@destack/schema";
import type { ObjectType } from "../object/object.ts";
import { defineServer, type PackageServer, type ServerDefinition } from "../server/server.ts";

/** Serve object types through one server per package, the first one taking the other parts. */
export function serversOf(
    objects: readonly ObjectType[],
    definition: Omit<ServerDefinition, "objects"> = {},
): PackageServer[] {
    // refuse parts no package's server would carry
    if (objects.length === 0 && Object.keys(definition).length > 0) {
        throw new TypeError("a server's parts need a package of object types");
    }

    // group the types by their package, in order of first appearance
    const byPackage = Map.groupBy(objects, (type) => type.package.id);

    return [...byPackage.values()].map((types, index) => {
        // declare the package's types and serve them as handled
        const owner = nth(types, 0).package;
        const declared = Object.fromEntries(types.map((type) => [type.name, type]));
        const handle = PackageHandle.of(owner, { objects: declared });

        return defineServer(
            handle,
            index === 0 ? { ...definition, objects: types } : { objects: types },
        );
    });
}
