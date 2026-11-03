import type { ResourceDeclaration } from "../declare/declaration.ts";
import { ResourceDescription } from "../declare/declaration.ts";

/** Describe a declared resource by its name, its resource kind and its specification. */
export function describeResource(
    resource: Pick<ResourceDeclaration<unknown>, "name" | "kind" | "spec">,
): ResourceDescription {
    return ResourceDescription.parse({
        name: resource.name,
        kind: resource.kind,
        spec: resource.spec,
    });
}
