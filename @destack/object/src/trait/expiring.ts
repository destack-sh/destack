import { none } from "@destack/access";
import { Duration } from "@destack/schema";
import { TABLE, Condition, Now } from "@destack/db";
import { t } from "@destack/locale";
import { defineMethod } from "../method/method.ts";
import { Empty } from "../method/procedure.ts";
import { ObjectKey, type ObjectType } from "../object/object.ts";
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
const expiringTrait: TraitBehavior<readonly ExpiryRule[]> = {
    key: "expiring",
    storages: ["durable"],
    options: (definition) => definition.expiring,
    columns: () => ({}),
    constraints: () => [],
    // let the code serving the type expire its objects, through its role
    policy: (_options, _object, permissions) =>
        permissions.includes("expire") ? {} : { permissions: { expire: none() } },
    methods: () => ({ expire: Expiring.expire }),
    validate: (rules, object) => Expiring.requireRules(object, rules),
    // expire each scope's objects once a rule's window passed
    controllers: (rules, object) =>
        rules.map((rule, index) => ({
            name: rules.length === 1 ? "expire" : `expire.${String(index)}`,
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
};

/** Remove objects once a rule's window passes. */
export const expiring = callableTrait("expiring", expiringTrait);

/** The expiry of objects: removing them, and checking the rules that time it. */
export const Expiring = {
    /** Remove an expired object at any revision, past the trash. */
    expire: defineMethod<{ kind: "delete"; permission: "expire"; mutates: true }>({
        title: t`Expire`,
        kind: "delete",
        permission: "expire",
        visibility: "package",
        mutates: true,
        target: "object",
        result: "value",
        procedure: (_name, schemas) => ({
            route: { method: "DELETE", path: "/{id}" },
            input: schemas.target.extend(schemas.replay),
            output: Empty,
        }),
        handler: async (context) => {
            await context.remove();

            return {};
        },
    }),

    /** Require at least one valid rule. */
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
