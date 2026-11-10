import { graph } from "@destack/package";
import { ViewDescription, ViewRender } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import type { View } from "../declare/view.ts";

/** Describe a view by its name, presented types, roles, called packages and render mode. */
export function describeView(view: View): schema.Infer<typeof ViewDescription.declaration> {
    return {
        name: view.name,
        presents: view.presents.map(({ object, priority }) => ({
            packageId: object.package.id,
            type: object.name,
            priority,
        })),
        categories: [...view.categories],
        audience: view.audience,
        packages: view.packages.map((handle) => handle.id),
        render: renderedOf(view),
    };
}

/** Describe how a view renders, with a static view's origin and hydration. */
function renderedOf(view: View): schema.Infer<typeof ViewRender.declaration> {
    // describe a client or server view by its mode
    const { render } = view;
    if (render.mode !== "static") {
        return { mode: render.mode };
    }

    // describe a static view by its origin and hydration
    return { mode: "static", origin: render.origin, hydrate: render.hydrate ?? "page" };
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
