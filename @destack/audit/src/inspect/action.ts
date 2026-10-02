import { defineSchema, schema, toJsonSchema, type JsonValue } from "@destack/schema";
import { AuditActionName, type AuditAction } from "../action/index.ts";
import { Package } from "@destack/package";

/** A serializable action declaration. */
export const AuditActionDescription = defineSchema(
    schema.object({
        /** The package-local noun.verb action name. */
        name: AuditActionName,
        /** The declaring package. */
        package: Package,
        /** A short description. */
        description: schema.string().exactOptional(),
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
        ...(action.description === undefined ? {} : { description: action.description }),
        targets: toJsonSchema(action.targets),
        details: toJsonSchema(action.details),
    });
}

/** List an action's term: its name, with the shapes of its calls. */
export function auditActionVocabulary(input: Record<string, JsonValue>): Record<string, JsonValue> {
    const description = AuditActionDescription.parse(input);

    return { [description.name]: { targets: description.targets, details: description.details } };
}
