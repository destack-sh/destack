import { schema, toJsonSchema, type JsonValue } from "@destack/schema";
import { AuditActionName, type AuditAction } from "../audit/action.ts";
import { Package } from "@destack/package";

/** A serializable action declaration. */
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
/** A serializable action declaration. */
export type AuditActionDescription = schema.Infer<typeof AuditActionDescription>;

/** Describe an action. */
export function describeAuditAction(action: AuditAction): AuditActionDescription {
    return AuditActionDescription.parse({
        name: action.name,
        package: action.package,
        ...(action.description === undefined ? {} : { description: action.description }),
        target: toJsonSchema(action.target),
        details: toJsonSchema(action.details),
    });
}

/** List an action's term: its name, with the shapes of its calls. */
export function auditActionVocabulary(action: AuditAction): Record<string, JsonValue> {
    const description = describeAuditAction(action);

    return { [description.name]: { target: description.target, details: description.details } };
}
