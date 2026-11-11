import type { ResourceDeclaration } from "../declare/declaration.ts";
import type { ResourceDescription } from "../declare/declaration.ts";

/** Describe a declared resource as its kind's declarations are: its name and specification. */
export function describeResource(resource: ResourceDeclaration<unknown>): ResourceDescription {
    return resource.describe();
}
