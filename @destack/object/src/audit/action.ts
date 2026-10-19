import { AuditTarget } from "./execution.ts";
import { schema, type JsonValue } from "@destack/schema";
import { ModuleMetadata, Package } from "@destack/package";

/** A package-local action name in the form noun.verb. */
export const AuditActionName = schema
    .string()
    .regex(/^[a-z][A-Za-z0-9]*(?:\.[a-z][A-Za-z0-9]*)+$/u);

/** An action as the manifest describes it. */
export const AuditActionDescription = schema.object({
    /** The package-local noun.verb action name. */
    name: AuditActionName,
    /** The declaring package. */
    package: Package,
    /** A short description. */
    description: schema.string().exactOptional(),
    /** The JSON Schema of the object each call acts on. */
    target: schema.json(),
    /** The JSON Schema of the action details. */
    details: schema.json(),
});
/** An action as the manifest describes it. */
export type AuditActionDescription = schema.Infer<typeof AuditActionDescription>;

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
