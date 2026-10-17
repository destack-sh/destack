import { contained, none } from "@destack/access";
import { Duration } from "@destack/schema";
import { TABLE, Condition } from "@destack/db";
import type { ObjectType } from "../object/object.ts";
import { expiry } from "../method/expiring.ts";
import { type Trait } from "./trait.ts";

/** A rule expiring matching objects a window after a time field. */
export interface ExpiryRule {
    /** How long after the field's time the object expires. */
    readonly after: Duration;
    /** The time field the window runs from. */
    readonly from: string;
    /** The rows the rule applies to, every row when absent. */
    readonly where?: Condition;
}

/** Objects the code serving them removes once a rule's window passes. */
export const expiring: Trait<readonly ExpiryRule[]> & {
    /** Replicate an expiring object type with the rules its host sets. */
    readonly after: <Self extends ObjectType>(object: Self, rules: readonly ExpiryRule[]) => Self;
    /** Require at least one valid rule. */
    readonly requireRules: (object: ObjectType, rules: readonly ExpiryRule[]) => void;
} = {
    key: "expiring",
    storages: ["durable"],
    options: (definition) => definition.expiring,
    columns: () => ({}),
    constraints: () => [],
    // let the code serving the type expire its objects, where code is contained outside the universe
    policy: (_options, object, permissions) =>
        permissions.includes("expire")
            ? {}
            : { permissions: { expire: object.isUniverseOnly ? none() : contained() } },
    methods: () => ({ expire: expiry }),
    validate: (rules, object) => expiring.requireRules(object, rules),
    after(object, rules) {
        // require an expiring type
        if (object.lifecycle.expiring === undefined) {
            throw new TypeError(`object ${object.name} does not expire`);
        }

        // replicate the type with the rules
        expiring.requireRules(object, rules);

        return object.with({
            definition: {
                ...object.definition,
                expiring: rules,
                source: { ...object.source, expiring: rules },
            },
        });
    },
    requireRules(object: ObjectType, rules: readonly ExpiryRule[]): void {
        // refuse an object expiring by no rule
        if (rules.length === 0) {
            throw new TypeError(`object ${object.name} has no expiry rule`);
        }

        // refuse invalid windows and unknown fields
        const columns = object.table[TABLE].columns;
        for (const rule of rules) {
            Duration.require(rule.after, `expiry window of ${object.name}`);
            if (!Object.hasOwn(columns, rule.from)) {
                throw new TypeError(
                    `object ${object.name} expires from unknown field ${rule.from}`,
                );
            }
        }
    },
};
