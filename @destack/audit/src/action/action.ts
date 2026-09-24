import { defineSchema, schema } from "@destack/schema";
import { AuditTarget } from "../event/target.ts";
import { declaringModule, Package, type ModuleMetadata } from "@destack/package";

/** A package-local action named Noun.verb, with PascalCase nouns and a camelCase present-tense verb. */
export const AuditActionName = defineSchema(
    schema.string().regex(/^[A-Z][A-Za-z0-9]*(?:\.[A-Z][A-Za-z0-9]*)*\.[a-z][A-Za-z0-9]*$/),
);

/** Declaring package, action name, and action schema version. */
export const AuditActionReference = defineSchema(
    schema.object({
        package: Package,
        name: AuditActionName,
        version: schema.number().int().positive(),
    }),
);
/** A serializable action declaration reference. */
export type AuditActionReference = schema.Infer<typeof AuditActionReference>;

/** A typed action declared by a package. */
export interface AuditAction<
    Targets extends schema.Schema = schema.Schema,
    Details extends schema.Schema = schema.Schema,
> {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The package-local Noun.verb action name, with a present-tense verb. */
    readonly name: string;
    /** The version of the targets and details schemas. */
    readonly version: number;
    /** A short description for inspection. */
    readonly description?: string;
    /** Named affected objects. */
    readonly targets: Targets;
    /** Explicitly selected, serializable action details. */
    readonly details: Details;
}

/** Declare an action without recording an event or acquiring authority. */
export function defineAuditAction<
    Targets extends schema.Schema<Record<string, AuditTarget>>,
    Details extends schema.Schema,
>(
    definition: Omit<AuditAction<Targets, Details>, "package">,
    module?: ModuleMetadata,
): AuditAction<Targets, Details> {
    // validate the action name and schema version
    AuditActionName.parse(definition.name);
    schema.number().int().positive().parse(definition.version);

    return Object.freeze({
        ...definition,
        package: Package.parse(declaringModule(module, "defineAuditAction").package),
    });
}
