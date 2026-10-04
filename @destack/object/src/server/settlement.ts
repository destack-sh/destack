import { and, eq, isNotNull, isNull, lte, or, type DatabaseConnection } from "@destack/db";
import { schema, Duration, type JsonValue } from "@destack/schema";
import { ServiceError } from "@destack/service";
import type { Controller } from "@destack/service/control";
import { v7 } from "uuid";
import type { Call } from "../method/call.ts";
import { settlement, SettlementCall } from "../method/settlement.ts";
import { Method } from "../method/method.ts";
import type { ObjectServer } from "./server.ts";

/** The settlement and claim grace, in milliseconds: a minute, above any live server's settling. */
const SETTLE_GRACE_MILLISECONDS = 60_000;

/** One recorded settlement. */
type Settlement = typeof settlement.$inferSelect;

/** Settle calls' prepared work once: commit it or roll it back, as the settlement records the call's outcome. */
export const Settlement = {
    /** Reserve a settlement for a call's external work under its idempotency key, returning the settlement's identifier. */
    async reserve(database: DatabaseConnection, call: Call, key: string): Promise<string> {
        const id = v7();
        await database.insert(settlement).values({
            id,
            key,
            object: call.object.name,
            method: call.name,
            scope: call.scope,
            target: call.id ?? null,
            prepared: null,
            committedAt: null,
            createdAt: call.now,
        });

        return id;
    },

    /** Record a settlement's prepared value, refusing one a settler claimed meanwhile. */
    async record(
        database: DatabaseConnection,
        id: string,
        call: Call,
        prepared: JsonValue,
    ): Promise<void> {
        const recorded = await database
            .update(settlement)
            .set({ prepared: { value: prepared } })
            .where(and(eq(settlement.id, id), isNull(settlement.claimedAt)))
            .returning({ id: settlement.id });
        if (recorded.length === 0) {
            refuseOutlasted(call);
        }
    },

    /** Mark a settlement's call committed inside its transaction, refusing one a settler claimed meanwhile. */
    async commit(
        transaction: DatabaseConnection,
        id: string,
        call: Call,
        now: number,
    ): Promise<void> {
        const committed = await transaction
            .update(settlement)
            .set({ committedAt: now })
            .where(
                and(
                    eq(settlement.id, id),
                    isNotNull(settlement.prepared),
                    isNull(settlement.claimedAt),
                ),
            )
            .returning({ id: settlement.id });
        if (committed.length === 0) {
            refuseOutlasted(call);
        }
    },

    /** Claim a settlement unless another claim is fresh. */
    async claim(
        database: DatabaseConnection,
        id: string,
        now: number,
        grace = SETTLE_GRACE_MILLISECONDS,
    ): Promise<Settlement | undefined> {
        const [claimed] = await database
            .update(settlement)
            .set({ claimedAt: now })
            .where(
                and(
                    eq(settlement.id, id),
                    or(isNull(settlement.claimedAt), lte(settlement.claimedAt, now - grace)),
                ),
            )
            .returning();

        return claimed;
    },

    /** Forget a settlement once its call settled. */
    async forget(database: DatabaseConnection, id: string): Promise<void> {
        await database.delete(settlement).where(eq(settlement.id, id));
    },

    /** Settle one claimed settlement as its row records: commit a committed call's work, roll back any other, then forget it. */
    async settle(
        server: Pick<ObjectServer, "objects" | "database">,
        row: Settlement,
        now: number,
    ): Promise<void> {
        // find the method
        const object = server.objects.find((served) => served.name === row.object);
        if (object === undefined) {
            throw new TypeError(`settlement ${row.id} names unserved object ${row.object}`);
        }
        const method = object.method(row.method);
        const declared = method.prepared;
        if (declared === undefined || !Method.settles(method)) {
            throw new TypeError(
                `settlement ${row.id} names ${row.object}.${row.method}, which settles no prepared work`,
            );
        }

        // rebuild the recorded call and its prepared value
        const call = new SettlementCall({
            object,
            name: row.method,
            scope: row.scope,
            id: row.target ?? undefined,
            idempotencyKey: row.key,
            database: server.database,
            now,
        });
        const prepared = row.prepared === null ? undefined : declared.parse(row.prepared.value);

        // commit a committed call's work, and roll back the work of any other
        if (row.committedAt !== null) {
            if (prepared === undefined) {
                throw new TypeError(`settlement ${row.id} committed without its prepared value`);
            }
            await method.commit?.(call, prepared);
        } else {
            await method.rollback?.(call, prepared);
        }
        await Settlement.forget(server.database, row.id);
    },

    /** Settle what servers left once the grace passes. */
    controller(
        server: Pick<ObjectServer, "objects" | "database">,
        options: { readonly grace?: Duration } = {},
    ): Controller {
        const grace =
            options.grace === undefined
                ? SETTLE_GRACE_MILLISECONDS
                : Duration.milliseconds(options.grace);

        return {
            name: "settlement",
            watches: [settlement],
            keys: (change) => [schema.string().parse(change.key["id"])],
            list: async () =>
                (await server.database.select({ id: settlement.id }).from(settlement)).map(
                    (row) => row.id,
                ),
            reconcile: async (key) => {
                // wait out the grace
                const [row] = await server.database
                    .select()
                    .from(settlement)
                    .where(eq(settlement.id, key));
                const now = Date.now();
                if (row === undefined) {
                    return undefined;
                } else if (row.createdAt > now - grace) {
                    return row.createdAt + grace - now;
                }

                // claim and settle
                const claimed = await Settlement.claim(server.database, key, now, grace);
                if (claimed === undefined) {
                    return row.claimedAt === null ? 0 : row.claimedAt + grace - now;
                }
                await Settlement.settle(server, claimed, now);

                return undefined;
            },
        };
    },
};

/** Refuse a call whose settlement a settler claimed after its grace passed. */
function refuseOutlasted(call: Call): never {
    throw new ServiceError("SERVICE_UNAVAILABLE", {
        message: `${call.object.name}.${call.name} outlasted its settlement grace`,
    });
}
