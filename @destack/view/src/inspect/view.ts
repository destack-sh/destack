import { graph } from "@destack/package";
import { ViewPresentation, type ViewDescription } from "@destack/package/view";
import { schema, type JsonObject } from "@destack/schema";
import type { View } from "../declare/view.ts";

/** The object types a view's description presents, beside its name and permissions. */
const Presents = schema.looseObject({ presents: schema.array(ViewPresentation) });

/** Describe a view by its name, the permissions it requests and the object types it presents. */
export function describeView(view: View): {
    readonly name: string;
    readonly permissions: ViewDescription["permissions"];
    readonly presents: readonly ViewPresentation[];
} {
    return {
        name: view.name,
        permissions: view.permissions.map(({ packageId, type, name }) => ({
            packageId,
            type,
            name,
        })),
        presents: view.presents.map(({ object, priority }) => ({
            packageId: object.package.id,
            type: object.name,
            priority,
        })),
    };
}

/** Present each object type a view presents. */
export function viewSymbols(input: JsonObject): graph.MemberSymbol[] {
    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: Presents.parse(input).presents.map((presented) => ({
                kind: "presents",
                symbol: { packageId: presented.packageId, kind: "object", name: presented.type },
            })),
        },
    ]);
}
