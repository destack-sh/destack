import { defineSchema, schema } from "@destack/schema";
import { AuditTarget } from "../event/target.ts";
import { declaringModule, Package, type ModuleMetadata } from "@destack/package";

/** A package-local action name of the form Noun.verb. */
export const AuditActionName = defineSchema(
    schema.string().regex(/^[A-Z][A-Za-z0-9]*(?:\.[A-Z][A-Za-z0-9]*)*\.[a-z][A-Za-z0-9]*$/),
);

/** A reference to an action of a package release, whose schemas read its targets and details. */
export const AuditActionReference = defineSchema(
    schema.object({
        /** The declaring package at the release that recorded the event. */
        package: Package,
        /** The package-local Noun.verb action name. */
        name: AuditActionName,
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
    // validate the action name
    AuditActionName.parse(definition.name);

    return Object.freeze({
        ...definition,
        package: Package.parse(declaringModule(module, "defineAuditAction").package),
    });
}
