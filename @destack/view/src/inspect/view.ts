import { graph } from "@destack/package";
import { ViewPresentation, type ViewDescription } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import type { View } from "../declare/view.ts";

/** Describe a view by its name, the types it presents and the packages it calls. */
export function describeView(view: View): {
    readonly name: string;
    readonly presents: readonly ViewPresentation[];
    readonly packages: ViewDescription["packages"];
} {
    return {
        name: view.name,
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
