import type {} from "@destack/stack/declare";
import type { StackInputOf } from "@destack/object";
import type { alertRule } from "../object/alert-rule.ts";

/** Add the alert rules collection to stacks. */
declare module "@destack/stack/declare" {
    interface StackDefinition {
        /** The space's alert rules, by name. */
        readonly alertRules?: Readonly<Record<string, StackInputOf<typeof alertRule>>>;
    }
}
