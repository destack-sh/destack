import { Duration } from "@destack/schema";
import { TABLE, Condition } from "@destack/db";
import type { ObjectType } from "../object/object.ts";
import { expiry } from "../method/expiring.ts";
import type { Trait } from "./trait.ts";

/** A rule expiring matching objects a window after a time field. */
export interface ExpiryRule {
    /** How long after the field's time the object expires. */
    readonly after: Duration;
    /** The time field the window runs from. */
    readonly from: string;
    /** The rows the rule applies to, every row when absent. */
    readonly where?: Condition;
}

/** Objects the system removes once a rule's window passes. */
export const expiring: Trait<readonly ExpiryRule[]> & {
    /** Copy an expiring object type with the rules its host sets. */
    after<Self extends ObjectType>(object: Self, rules: readonly ExpiryRule[]): Self;
} = {
    key: "expiring",
    isDurable: true,
    options: (definition) => definition.expiring,
    columns: () => ({}),
    constraints: () => [],
    methods: () => ({ expire: expiry }),
    validate: (rules, object) => requireRules(object, rules),
    after(object, rules) {
        // require an expiring type
        if (object.lifecycle.expiring === undefined) {
            throw new TypeError(`object ${object.name} does not expire`);
        }

        // copy the type with the rules
        requireRules(object, rules);
        const traits = object.traits.map((applied) =>
            applied.trait === expiring ? { trait: applied.trait, options: rules } : applied,
        );

        return object.with({ lifecycle: { ...object.lifecycle, expiring: rules }, traits });
    },
};

/** Require at least one valid rule. */
function requireRules(object: ObjectType, rules: readonly ExpiryRule[]): void {
    // refuse an object expiring by no rule
    if (rules.length === 0) {
        throw new TypeError(`object ${object.name} expires by no rule`);
    }

    // refuse invalid windows and unknown fields
    const columns = object.table[TABLE].columns;
    for (const rule of rules) {
        Duration.require(rule.after, `expiry window of ${object.name}`);
        if (!Object.hasOwn(columns, rule.from)) {
            throw new TypeError(`object ${object.name} expires from unknown field ${rule.from}`);
        }
    }
}
