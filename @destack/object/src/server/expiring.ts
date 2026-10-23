import { earliest } from "@destack/access";
import { Duration } from "@destack/schema";
import {
    Change,
    and,
    type DatabaseConnection,
    isNotNull,
    lte,
    notInArray,
    min,
    type Row,
    type Select,
    TABLE,
    type Table,
    Condition,
} from "@destack/db";
import type { ReconcileController, Reconciliation } from "@destack/db";
import { Replica, RequestId } from "@destack/sync";
import { ObjectKey, type ObjectType } from "../object/object.ts";
import type { ExpiryRule } from "../trait/expiring.ts";
import type { ObjectStore } from "./store.ts";

/** The most rows one expiry removes per transaction: about a millisecond of deletes. */
const EXPIRE_ROWS = 100;

/** The component name the expiry runs and records its calls under. */
const EXPIRY = "expiry";

/** Remove expiring objects as their windows pass, scheduling the next window's end. */
export class ExpiringController implements ReconcileController {
    /** The controller's name. */
    readonly name = EXPIRY;
    /** The tables of the expiring objects. */
    readonly watches: readonly Table[];
    /** The store the expiry runs through. */
    readonly #store: ObjectStore;
    /** The expiring object types. */
    readonly #types: readonly ObjectType[];

    /** Watch the expiring objects a store serves. */
    constructor(store: ObjectStore) {
        this.#store = store;
        // leave the replicated rows to the deletes their home sends
        this.#types = store.objects.filter(
            (object) =>
                object.lifecycle.expiring !== undefined && !store.database.replicates(object.table),
        );
        this.watches = this.#types.map((object) => object.table);
    }

    /** Look again when a rule's time field changes. */
    keys(change: Change): string[] {
        // select the expiry once a changed row's rule time moves
        const object = this.#types.find((type) => type.table === change.table);
        const after: Row | null = Change.after(change);
        const before: Row | null = Change.before(change);
        const isTimed = (object?.lifecycle.expiring ?? []).some(
            (rule) =>
                after?.[rule.from] !== undefined &&
                after[rule.from] !== null &&
                before?.[rule.from] !== after[rule.from],
        );

        return isTimed ? ["expiry"] : [];
    }

    /** List the one key the expiry runs under. */
    async list(): Promise<string[]> {
        return ["expiry"];
    }

    /** Expire what passed and schedule the earliest window end. */
    async reconcile(_key: string, _reconciliation: Reconciliation): Promise<number | undefined> {
        // expire what passed and read each rule's earliest window end
        const now = this.#store.clock.now();
        await ExpiringController.expire(this.#store, now);
        const passes = await Promise.all(
            this.#types.flatMap((object) =>
                (object.lifecycle.expiring ?? []).map((rule) =>
                    ExpiringController.#passing(this.#store.database, object, rule),
                ),
            ),
        );
        const next = earliest(passes);

        return next === undefined ? undefined : Math.max(0, next - now);
    }

    /** Remove expired objects as the code serving their type, returning the count. */
    static async expire(store: ObjectStore, now: number): Promise<number> {
        // remove each rule's rows in batches until one comes back short
        let removed = 0;
        for (const object of store.objects.filter(
            (each) => !store.database.replicates(each.table),
        )) {
            for (const rule of object.lifecycle.expiring ?? []) {
                for (let batch = EXPIRE_ROWS; batch === EXPIRE_ROWS;) {
                    const rows = await ExpiringController.#expired(
                        store.database,
                        object,
                        rule,
                        now,
                    );
                    await store.mutate(
                        store.serving(object, now),
                        {
                            id: RequestId.create(now),
                            calls: rows.map((row) =>
                                object.call("expire", object.target(ObjectKey.parse(row))),
                            ),
                        },
                        { component: EXPIRY },
                    );
                    batch = rows.length;
                    removed += batch;
                }
            }
        }

        return removed;
    }

    /** Read a batch of a type's rows a rule's window released. */
    static async #expired(
        database: DatabaseConnection,
        object: ObjectType,
        rule: ExpiryRule,
        now: number,
    ): Promise<Select<Table>[]> {
        // leave the rows of scopes the database replicates to the deletes their home sends
        const table = object.table;
        const replicated = await Replica.scopes(database, table);
        const unreplicated =
            replicated.length === 0
                ? undefined
                : notInArray(table[TABLE].column("scope"), replicated);

        return database
            .select()
            .from(table)
            .where(
                and(
                    lte(table[TABLE].column(rule.from), now - Duration.milliseconds(rule.after)),
                    ExpiringController.#matching(table, rule),
                    unreplicated,
                ),
            )
            .limit(EXPIRE_ROWS);
    }

    /** Read when a rule's window next passes for a type. */
    static async #passing(
        database: DatabaseConnection,
        object: ObjectType,
        rule: ExpiryRule,
    ): Promise<number | undefined> {
        // read the earliest time
        const table = object.table;
        const column = table[TABLE].column(rule.from);
        const [row] = await database
            .select({ from: min(column) })
            .from(table)
            .where(and(isNotNull(column), ExpiringController.#matching(table, rule)));
        if (row === undefined) {
            throw new TypeError(
                `reading the earliest ${rule.from} of ${object.name} returned no row`,
            );
        }

        // require a time in UTC epoch milliseconds
        const from = row.from;
        if (from === null) {
            return undefined;
        } else if (typeof from !== "number") {
            throw new TypeError(`${object.name}.${rule.from} is not a time`);
        }

        return from + Duration.milliseconds(rule.after);
    }

    /** Render a rule's condition on its table. */
    static #matching(table: Table, rule: ExpiryRule) {
        return rule.where === undefined ? undefined : Condition.render(rule.where, table);
    }
}
