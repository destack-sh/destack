import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import { AuditActionName, type AuditAction } from "../action/index.ts";
import { Package } from "@destack/package";

/** A serializable action declaration collected by the package build. */
export const AuditActionDescription = defineSchema(
    schema.object({
        name: AuditActionName,
        package: Package,
        version: schema.number().int().positive(),
        description: schema.string().optional(),
        targets: schema.json(),
        details: schema.json(),
    }),
);
/** An inspected action declaration. */
export type AuditActionDescription = schema.Infer<typeof AuditActionDescription>;

/** Describe the action's target roles and explicitly allowed details. */
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
