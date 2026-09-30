import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import type { SpaceInstallation } from "./installation.ts";
import type { SpaceResource } from "./resource.ts";
import type { SpaceRelationship, SpaceRole } from "./permission.ts";
import type { SpacePolicy } from "../policy/index.ts";

/** The collections a stack may declare; packages declaring objects add theirs by module augmentation. */
export interface SpaceDocument {
    /** Package admission and network policies for the space. */
    readonly policies?: schema.Input<typeof SpacePolicy>;
    /** Resources created or adopted by the stack. */
    readonly resources?: Readonly<Record<string, schema.Input<typeof SpaceResource>>>;
    /** Independently configured package installations. */
    readonly installations?: Readonly<Record<string, schema.Input<typeof SpaceInstallation>>>;
    /** Roles defined in the space. */
    readonly roles?: Readonly<Record<string, schema.Input<typeof SpaceRole>>>;
    /** Roles bound and relations granted to members, groups, workloads and services. */
    readonly relationships?: Readonly<Record<string, schema.Input<typeof SpaceRelationship>>>;
}

/** A stack's declarations as JSON, keyed by collection and validated by each object when applied. */
export const SpaceDefinition = defineSchema(schema.record(schema.string().min(1), schema.json()));
/** A stack's declarations as JSON, keyed by collection. */
export type SpaceDefinition = schema.Infer<typeof SpaceDefinition>;

/** A stack's declarations with the package declaring them. */
export interface SpaceDeclaration {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The declarations as JSON, keyed by collection. */
    readonly definition: SpaceDefinition;
}

/** Define a stack's declarations, serialising each declared value to its JSON form. */
export function defineSpace(collections: SpaceDocument, module?: ModuleMetadata): SpaceDeclaration {
    return {
        package: declaringModule(module, "defineSpace").package,
        definition: SpaceDefinition.parse(JSON.parse(JSON.stringify(collections))),
    };
}
