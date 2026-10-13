import { graph } from "@destack/package";
import { ViewPresentation, type ViewDescription } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import type { View } from "../declare/view.ts";

/** Describe a view by its name and the permissions it requests by scope. */
export function describeView(view: View): {
    readonly name: string;
    readonly permissions: ViewDescription["permissions"];
    readonly presents: readonly ViewPresentation[];
    readonly packages: ViewDescription["packages"];
} {
    // describe the permissions of each scope as plain references
    const permissions: ViewDescription["permissions"] = Object.fromEntries(
        Object.entries(view.permissions).map(([scope, requested]) => [
            scope,
            requested.map(({ packageId, type, name }) => ({ packageId, type, name })),
        ]),
    );

    return {
        name: view.name,
        permissions,
        presents: view.presents.map(({ object, priority }) => ({
            packageId: object.package.id,
            type: object.name,
            priority,
        })),
        packages: view.packages.map((handle) => handle.id),
    };
}

/** Present each object type a view presents. */
export function viewSymbols(view: View): graph.MemberSymbol[] {
    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: view.presents.map(({ object }) => ({
                kind: "presents",
                symbol: { packageId: object.package.id, kind: "object", name: object.name },
            })),
        },
    ]);
}
