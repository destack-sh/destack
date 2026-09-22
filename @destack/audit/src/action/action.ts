import { defineSchema, schema } from "@destack/schema";
import { AuditTarget } from "../event/target.ts";
import { Package } from "@destack/package";

/** A package-local action named noun.verb, with a present-tense verb and optional nested nouns. */
export const AuditActionName = defineSchema(
    schema.string().regex(/^[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*)+$/),
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
    /** The declaring package, normally import.meta.destack.package. */
    readonly package: Package;
    /** The package-local noun.verb action name, with a present-tense verb. */
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
>(action: AuditAction<Targets, Details>): AuditAction<Targets, Details> {
    AuditActionName.parse(action.name);
    Package.parse(action.package);
    schema.number().int().positive().parse(action.version);

    return Object.freeze({ ...action });
}
