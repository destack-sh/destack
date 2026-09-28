import { defineSchema, schema } from "@destack/schema";
import { AuditTarget } from "../event/target.ts";
import { declaringModule, Package, type ModuleMetadata } from "@destack/package";

/** A package-local action name of the form Noun.verb. */
export const AuditActionName = defineSchema(
    schema.string().regex(/^[A-Z][A-Za-z0-9]*(?:\.[A-Z][A-Za-z0-9]*)*\.[a-z][A-Za-z0-9]*$/),
);

/** A reference to a versioned action of a package. */
export const AuditActionReference = defineSchema(
    schema.object({
        /** The declaring package. */
        package: Package,
        /** The package-local Noun.verb action name. */
        name: AuditActionName,
        /** The version of the targets and details schemas. */
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
    /** The declaring package. */
    readonly package: Package;
    /** The package-local Noun.verb action name. */
    readonly name: string;
    /** The version of the targets and details schemas. */
    readonly version: number;
    /** A short description for inspection. */
    readonly description?: string;
    /** Named affected objects. */
    readonly targets: Targets;
    /** The serializable action details. */
    readonly details: Details;
}

/** Declare an action. */
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
