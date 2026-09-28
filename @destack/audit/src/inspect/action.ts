import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import { AuditActionName, type AuditAction } from "../action/index.ts";
import { Package } from "@destack/package";

/** A serializable action declaration. */
export const AuditActionDescription = defineSchema(
    schema.object({
        /** The package-local Noun.verb action name. */
        name: AuditActionName,
        /** The declaring package. */
        package: Package,
        /** The version of the targets and details schemas. */
        version: schema.number().int().positive(),
        /** A short description. */
        description: schema.string().optional(),
        /** The JSON Schema of the named affected objects. */
        targets: schema.json(),
        /** The JSON Schema of the action details. */
        details: schema.json(),
    }),
);
/** An inspected action declaration. */
export type AuditActionDescription = schema.Infer<typeof AuditActionDescription>;

/** Describe an action. */
export function describeAuditAction(action: AuditAction): AuditActionDescription {
    return AuditActionDescription.parse({
        name: action.name,
        package: action.package,
        version: action.version,
        description: action.description,
        targets: toJsonSchema(action.targets),
        details: toJsonSchema(action.details),
    });
}
