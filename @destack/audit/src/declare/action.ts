import { AuditTarget } from "../record/execution.ts";
import { defineSchema, schema, type JsonValue } from "@destack/schema";
import { ModuleMetadata, Package } from "@destack/package";

/** A package-local action name in the form noun.verb. */
export const AuditActionName = defineSchema(
    schema.string().regex(/^[a-z][A-Za-z0-9]*(?:\.[a-z][A-Za-z0-9]*)+$/u),
);

/** A typed action declared by a package. */
export interface AuditAction<
    Target extends schema.Schema<AuditTarget> = schema.Schema<AuditTarget>,
    Details extends schema.Schema<JsonValue> = schema.Schema<JsonValue>,
> {
    /** The declaring package. */
    readonly package: Package;
    /** The package-local noun.verb action name. */
    readonly name: string;
    /** A short description for inspection. */
    readonly description?: string;
    /** The object each call acts on. */
    readonly target: Target;
    /** The serializable action details. */
    readonly details: Details;
}

/** Declare an action. */
export function defineAuditAction<
    Target extends schema.Schema<AuditTarget>,
    Details extends schema.Schema<JsonValue>,
>(
    definition: Omit<AuditAction<Target, Details>, "package">,
    module?: ModuleMetadata,
): AuditAction<Target, Details> {
    // validate the action name
    AuditActionName.parse(definition.name);

    return Object.freeze({
        ...definition,
        package: Package.parse(ModuleMetadata.require(module, "defineAuditAction").package),
    });
}
