import type { SpaceDeclaration, SpaceDefinition } from "../declare/space.ts";

/** Describe a declared space configuration for the package manifest. */
export function describeSpace(space: SpaceDeclaration): SpaceDefinition {
    return space.definition;
}
