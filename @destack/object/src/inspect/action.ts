import { toJsonSchema, type JsonValue } from "@destack/schema";
import { type AuditAction, AuditActionDescription } from "../audit/action.ts";

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
