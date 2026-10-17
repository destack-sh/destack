import type { schema } from "@destack/schema";
import type {} from "@destack/stack/declare";
import type { AlertRuleDefinition } from "../object/alert-rule.ts";

/** Add the alert rules collection to stacks. */
declare module "@destack/stack/declare" {
    interface StackDefinition {
        /** The space's alert rules, by name. */
        readonly alertRules?: Readonly<Record<string, schema.Input<typeof AlertRuleDefinition>>>;
    }
}
