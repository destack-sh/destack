import { earliest } from "@destack/access";
import { Duration } from "@destack/schema";
import {
    Change,
    and,
    type DatabaseConnection,
    isNotNull,
    lte,
    min,
    type Row,
    type Select,
    TABLE,
    type Table,
    Condition,
} from "@destack/db";
import type { Controller, Reconciliation } from "@destack/service/control";
import type { ObjectType } from "../object/object.ts";
import { SystemCall } from "../method/system.ts";
import type { ExpiryRule } from "../trait/expiring.ts";
import type { ObjectServer } from "./server.ts";

/** The most rows one expiry removes per transaction: about a millisecond of deletes. */
const EXPIRE_ROWS = 100;

/** The server an expiry reads and removes through. */
type ExpiringServer = Pick<ObjectServer, "objects" | "database" | "executeAsSystem">;

/** Remove expiring objects as their windows pass, scheduling the next window's end. */
export class ExpiringController implements Controller {
    /** The controller's name. */
    readonly name = "expiry";
    /** The tables of the expiring objects. */
    readonly watches: readonly Table[];
    /** The server the expiry runs through. */
    readonly #server: ExpiringServer;
    /** The expiring object types. */
    readonly #types: readonly ObjectType[];

    /** Watch the expiring objects a server serves. */
    constructor(server: ExpiringServer) {
        this.#server = server;
        // leave the copied rows to the deletes their home sends
        this.#types = server.objects.filter(
            (object) =>
                object.lifecycle.expiring !== undefined && !server.database.copies(object.table),
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
        const now = Date.now();
        await ExpiringController.expire(this.#server, now);
        const passes = await Promise.all(
            this.#types.flatMap((object) =>
                (object.lifecycle.expiring ?? []).map((rule) =>
                    passing(this.#server.database, object, rule),
                ),
            ),
        );
        const next = earliest(passes);

        return next === undefined ? undefined : Math.max(0, next - now);
    }

    /** Remove expired objects as the system, returning the count. */
    static async expire(server: ExpiringServer, now: number): Promise<number> {
        // remove each rule's rows in batches until one comes back short
        let removed = 0;
        for (const object of server.objects.filter((each) => !server.database.copies(each.table))) {
            for (const rule of object.lifecycle.expiring ?? []) {
                for (let batch = EXPIRE_ROWS; batch === EXPIRE_ROWS;) {
                    const rows = await expired(server.database, object, rule, now);
                    if (rows.length > 0) {
                        await server.executeAsSystem(
                            object,
                            "expire",
                            rows.map((row) => SystemCall.of(row)),
                            now,
                        );
                    }
                    batch = rows.length;
                    removed += batch;
                }
            }
        }

        return removed;
    }
}

/** Read a batch of a type's rows a rule's window released. */
async function expired(
    database: DatabaseConnection,
    object: ObjectType,
    rule: ExpiryRule,
    now: number,
): Promise<Select<Table>[]> {
    const table = object.table;

    return database
        .select()
        .from(table)
        .where(
            and(
                lte(table[TABLE].column(rule.from), now - Duration.milliseconds(rule.after)),
                matching(table, rule),
            ),
        )
        .limit(EXPIRE_ROWS);
}

/** Read when a rule's window next passes for a type. */
async function passing(
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
        .where(and(isNotNull(column), matching(table, rule)));
    if (row === undefined) {
        throw new TypeError(`the earliest ${rule.from} of ${object.name} read no row`);
    }

    // require a time in UTC epoch milliseconds
    const from = row.from;
    if (from === null) {
        return undefined;
    } else if (typeof from !== "number") {
        throw new TypeError(`${object.name}.${rule.from} is no time`);
    }

    return from + Duration.milliseconds(rule.after);
}

/** Render a rule's condition on its table. */
function matching(table: Table, rule: ExpiryRule) {
    return rule.where === undefined ? undefined : Condition.render(rule.where, table);
}
