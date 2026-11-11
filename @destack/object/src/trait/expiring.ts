import { none } from "@destack/access";
import { Duration } from "@destack/schema";
import { TABLE, Condition, Now } from "@destack/db";
import { ObjectKey, type ObjectType } from "../object/object.ts";
import { expiry } from "../method/expiring.ts";
import { type TraitBehavior, callableTrait } from "./trait.ts";

/** The most objects one expiry mutation removes: about a millisecond of deletes. */
const EXPIRE_ROWS = 100;

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
const expiringTrait: TraitBehavior<readonly ExpiryRule[]> & {
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
    // let the code serving the type expire its objects, through its role
    policy: (_options, _object, permissions) =>
        permissions.includes("expire") ? {} : { permissions: { expire: none() } },
    methods: () => ({ expire: expiry }),
    validate: (rules, object) => expiring.requireRules(object, rules),
    // expire each scope's objects once a rule's window passed
    controllers: (rules, object) =>
        rules.map((rule, index) => ({
            name: `expiry-${String(index)}`,
            pending: { ...rule.where, [rule.from]: { lte: new Now().minus(rule.after) } },
            key: ["scope"],
            reconcile: async ({ rows, mutate }) => {
                const keys = rows.map((row) => ObjectKey.parse(row));

                // expire them a batch per mutation
                for (let start = 0; start < keys.length; start += EXPIRE_ROWS) {
                    await mutate(
                        keys
                            .slice(start, start + EXPIRE_ROWS)
                            .map((key) => object.call("expire", object.target(key))),
                    );
                }
            },
        })),
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
                    `object ${object.name} expires from unknown field '${rule.from}'`,
                );
            } else if (rule.where !== undefined && Object.hasOwn(rule.where, rule.from)) {
                throw new TypeError(
                    `an expiry rule of ${object.name} compares its time field '${rule.from}' in its condition`,
                );
            }
        }
    },
};

/** The expiring trait, callable to declare it on an object type. */
export const expiring = callableTrait("expiring", expiringTrait);
