import { AuditTarget } from "./execution.ts";
import { schema, type JsonValue } from "@destack/schema";
import { ModuleMetadata, PackageVersion } from "@destack/package";

/** A package-local action name in the form noun.verb. */
export const AuditActionName = schema
    .string()
    .regex(/^[a-z][A-Za-z0-9]*(?:\.[a-z][A-Za-z0-9]*)+$/u);

/** An action as the manifest describes it. */
export const AuditActionDescription = schema.object({
    /** The package-local noun.verb action name. */
    name: AuditActionName,
    /** The declaring package. */
    package: PackageVersion,
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
    readonly package: PackageVersion;
    /** The package-local noun.verb action name. */
    readonly name: string;
    /** The object each call acts on. */
    readonly target: Target;
    /** The serializable action details. */
    readonly details: Details;
    /** The sensitive values the details mark, which recording leaves out. */
    readonly redaction: schema.Redaction;
}

/** Declare an action. */
export function defineAuditAction<
    Target extends schema.Schema<AuditTarget>,
    Details extends schema.Schema<JsonValue>,
>(
    definition: Omit<AuditAction<Target, Details>, "package" | "redaction">,
    module?: ModuleMetadata,
): AuditAction<Target, Details> {
    // validate the action name
    AuditActionName.parse(definition.name);

    return Object.freeze({
        ...definition,
        redaction: new schema.Redaction(definition.details),
        package: PackageVersion.parse(ModuleMetadata.require(module, "defineAuditAction").package),
    });
}
