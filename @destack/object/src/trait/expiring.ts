import { earliest } from "@destack/access";
import {
    and,
    type Column,
    type DatabaseConnection,
    eq,
    isNotNull,
    lte,
    min,
    TABLE,
    type Table,
} from "@destack/db";
import { Condition } from "@destack/db/query";
import type { Controller } from "@destack/service/control";
import type { Call } from "../method/call.ts";
import { defineMethod, type Method } from "../method/method.ts";
import { Empty } from "../method/procedure.ts";
import { Duration } from "../object/duration.ts";
import type { ObjectType } from "../object/object.ts";
import { type ObjectServer, SystemCall } from "../server/server.ts";
import type { Trait } from "./trait.ts";

/** The most rows one expiry removes per transaction: about a millisecond of deletes. */
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

/** Objects the system removes once a rule's window passes. */
export const expiring: Trait<readonly ExpiryRule[]> & {
    /** Copy an expiring object type with the rules its host sets. */
    after<Self extends ObjectType>(object: Self, rules: readonly ExpiryRule[]): Self;
    /** Remove expired objects as the system, returning the count. */
    expire(
        server: Pick<ObjectServer, "objects" | "database" | "executeAsSystem">,
        now: number,
    ): Promise<number>;
    /** Remove objects as their windows pass. */
    controller(server: Pick<ObjectServer, "objects" | "database" | "executeAsSystem">): Controller;
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
        if (object.expiring === undefined) {
            throw new TypeError(`object ${object.name} does not expire`);
        }

        // copy the type with the rules
        requireRules(object, rules);
        const traits = object.traits.map((applied) =>
            applied.trait === expiring ? { trait: applied.trait, options: rules } : applied,
        );

        return object.with({ expiring: rules, traits });
    },
    async expire(server, now) {
        // remove each rule's rows in batches until one comes back short
        let removed = 0;
        for (const object of server.objects) {
            for (const rule of object.expiring ?? []) {
                for (let batch = EXPIRE_ROWS; batch === EXPIRE_ROWS;) {
                    const rows = await expired(server.database, object, rule, now);
                    if (rows.length > 0) {
                        await server.executeAsSystem(
                            object,
                            "expire",
                            rows.map(SystemCall.of),
                            now,
                        );
                    }
                    batch = rows.length;
                    removed += batch;
                }
            }
        }

        return removed;
    },
    controller(server) {
        // watch expiring objects
        const types = server.objects.filter((object) => object.expiring !== undefined);

        return {
            name: "expiry",
            watches: types.map((object) => object.table as Table),
            keys: (change) => {
                // look again when a rule's time field changes
                const object = types.find((type) => type.table === change.table);
                const after = change.after as Record<string, unknown> | undefined;
                const before = change.before as Record<string, unknown> | undefined;
                const isTimed = (object?.expiring ?? []).some(
                    (rule) =>
                        after?.[rule.from] !== undefined &&
                        after[rule.from] !== null &&
                        before?.[rule.from] !== after[rule.from],
                );

                return isTimed ? ["expiry"] : [];
            },
            list: async () => ["expiry"],
            reconcile: async () => {
                // expire, then schedule the earliest window end
                const now = Date.now();
                await expiring.expire(server, now);
                const passes = await Promise.all(
                    types.flatMap((object) =>
                        object.expiring!.map((rule) => passing(server.database, object, rule)),
                    ),
                );
                const next = earliest(passes);

                return next === undefined ? undefined : Math.max(0, next - now);
            },
        };
    },
};

/** Remove an expired object at any revision, bypassing the trash. */
const expiry: Method = defineMethod<Method<"delete", null, never, never, true>>({
    kind: "delete",
    permission: null,
    isSystem: true,
    mutates: true,
    target: true,
    result: "value",
    procedure: () => ({ route: { method: "DELETE", path: "/{id}" }, input: Empty, output: Empty }),
    effect: async (call: Call) => {
        const table = call.object.table as Table & Record<string, Column>;
        await call.database.delete(table).where(eq(table.id!, call.id!));

        return {};
    },
});

/** Require at least one valid rule. */
function requireRules(object: ObjectType, rules: readonly ExpiryRule[]): void {
    // refuse an object expiring by no rule
    if (rules.length === 0) {
        throw new TypeError(`object ${object.name} expires by no rule`);
    }

    // refuse invalid windows and unknown fields
    const columns = (object.table as Table)[TABLE].columns;
    for (const rule of rules) {
        Duration.require(rule.after, `expiry window of ${object.name}`);
        if (!Object.hasOwn(columns, rule.from)) {
            throw new TypeError(`object ${object.name} expires from unknown field ${rule.from}`);
        }
    }
}

/** Read a batch of a type's rows a rule's window released. */
async function expired(
    database: DatabaseConnection,
    object: ObjectType,
    rule: ExpiryRule,
    now: number,
): Promise<Record<string, unknown>[]> {
    const table = object.table as Table & Record<string, Column>;

    return (await database
        .select()
        .from(table)
        .where(
            and(
                lte(table[rule.from]!, now - Duration.milliseconds(rule.after)),
                matching(table, rule),
            ),
        )
        .limit(EXPIRE_ROWS)) as Record<string, unknown>[];
}

/** Read when a rule's window next passes for a type. */
async function passing(
    database: DatabaseConnection,
    object: ObjectType,
    rule: ExpiryRule,
): Promise<number | undefined> {
    // read the earliest time
    const table = object.table as Table & Record<string, Column>;
    const [row] = await database
        .select({ from: min(table[rule.from]!) })
        .from(table)
        .where(and(isNotNull(table[rule.from]!), matching(table, rule)));
    const from = row?.from as number | null | undefined;

    return from == null ? undefined : from + Duration.milliseconds(rule.after);
}

/** Render a rule's condition on its table. */
function matching(table: Table, rule: ExpiryRule) {
    return rule.where === undefined
        ? undefined
        : Condition.render(rule.where, Condition.bind(table));
}
