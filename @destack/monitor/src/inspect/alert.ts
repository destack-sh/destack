import type { JsonObject } from "@destack/schema";
import type { AlertRuleDeclaration } from "../declare/alert.ts";

/** Describe a declared alert rule for the build graph: its definition. */
export function describeAlertRule(rule: AlertRuleDeclaration): JsonObject {
    return rule.definition;
}
