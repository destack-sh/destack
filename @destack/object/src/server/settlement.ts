import { ObjectError } from "../error/error.ts";
import {
    and,
    eq,
    isNotNull,
    isNull,
    lte,
    or,
    type DatabaseConnection,
    type ReconcileController,
} from "@destack/db";
import { schema, Duration, type JsonValue, Uuid } from "@destack/schema";

import type { MethodContext } from "../method/context.ts";
import { settlement, SettlementContext } from "../method/settlement.ts";
import { Method } from "../method/method.ts";
import type { ObjectStore } from "./store.ts";

import type {} from "@destack/package/import-meta";

/** The settlement and claim grace, in milliseconds: a minute, above any live server's settling. */
const SETTLE_GRACE_MILLISECONDS = 60_000;

/** One recorded settlement. */
type Settlement = typeof settlement.$inferSelect;

/** Settle calls' prepared work once, committing or rolling back by the recorded outcome. */
export const Settlement = {
    /** Reserve a settlement for a call's external work by idempotency key, returning its id. */
    async reserve(
        database: DatabaseConnection,
        context: MethodContext,
        key: string,
    ): Promise<string> {
        const id = Uuid.v7(context.environment);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(settlement).values({
                    id,
                    key,
                    object: context.object.name,
                    method: context.name,
                    scope: context.scope,
                    target: context.id ?? null,
                    prepared: null,
                    committedAt: null,
                    createdAt: context.now,
                }),
        );

        return id;
    },

    /** Record a settlement's prepared value, refusing one a settler claimed meanwhile. */
    async record(
        database: DatabaseConnection,
        id: string,
        context: MethodContext,
        prepared: JsonValue,
    ): Promise<void> {
        const recorded = await database.transaction(
            async (transaction) =>
                await transaction
                    .update(settlement)
                    .set({ prepared: { value: prepared } })
                    .where(and(eq(settlement.id, id), isNull(settlement.claimedAt)))
                    .returning({ id: settlement.id }),
        );
        if (recorded.length === 0) {
            Settlement.refuseOutlasted(context);
        }
    },

    /** Mark a settlement's call committed in its transaction, refusing a claimed settlement. */
    async commit(
        transaction: DatabaseConnection,
        id: string,
        context: MethodContext,
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
            Settlement.refuseOutlasted(context);
        }
    },

    /** Claim a settlement unless another claim is fresh. */
    async claim(
        database: DatabaseConnection,
        id: string,
        now: number,
        grace = SETTLE_GRACE_MILLISECONDS,
    ): Promise<Settlement | undefined> {
        const [claimed] = await database.transaction(
            async (transaction) =>
                await transaction
                    .update(settlement)
                    .set({ claimedAt: now })
                    .where(
                        and(
                            eq(settlement.id, id),
                            or(
                                isNull(settlement.claimedAt),
                                lte(settlement.claimedAt, now - grace),
                            ),
                        ),
                    )
                    .returning(),
        );

        return claimed;
    },

    /** Forget a settlement once its call settled. */
    async forget(database: DatabaseConnection, id: string): Promise<void> {
        await database.transaction(
            async (transaction) =>
                await transaction.delete(settlement).where(eq(settlement.id, id)),
        );
    },

    /** Settle one claimed settlement as recorded, committing or rolling back, then forget it. */
    async settle(store: ObjectStore, row: Settlement, now: number): Promise<void> {
        // find the method
        const object = store.objects.find((served) => served.name === row.object);
        if (object === undefined) {
            throw new TypeError(
                `settlement ${row.id} has object ${row.object}, which nothing here serves`,
            );
        }
        const method = object.method(row.method);
        const declared = method.prepared;
        if (declared === undefined || !Method.settles(method)) {
            throw new TypeError(
                `settlement ${row.id} has ${row.object}.${row.method}, which has no prepared work to settle`,
            );
        }

        // rebuild the recorded call and its prepared value
        const call = new SettlementContext({
            object,
            name: row.method,
            scope: row.scope,
            id: row.target ?? undefined,
            idempotencyKey: row.key,
            database: store.database,
            now,
            store,
        });
        const prepared = row.prepared === null ? undefined : declared.parse(row.prepared.value);

        // settle a committed call's work as committed, and any other's as rolled back
        if (row.committedAt !== null) {
            if (prepared === undefined) {
                throw new TypeError(`settlement ${row.id} committed without its prepared value`);
            }
            await method.settle?.(call, { status: "committed", prepared });
        } else {
            await method.settle?.(call, { status: "rolledBack", prepared });
        }
        await Settlement.forget(store.database, row.id);
    },

    /** Settle what servers left once the grace passes. */
    controller(
        store: ObjectStore,
        options: { readonly grace?: Duration } = {},
    ): ReconcileController {
        const grace =
            options.grace === undefined
                ? SETTLE_GRACE_MILLISECONDS
                : Duration.milliseconds(options.grace);

        return {
            name: "settlement",
            watches: [settlement],
            keys: (change) => [schema.string().parse(change.key["id"])],
            list: async () =>
                (await store.database.select({ id: settlement.id }).from(settlement)).map(
                    (row) => row.id,
                ),
            reconcile: async (key) => {
                // wait out the grace
                const [row] = await store.database
                    .select()
                    .from(settlement)
                    .where(eq(settlement.id, key));
                const now = store.clock.now();
                if (row === undefined) {
                    return undefined;
                } else if (row.createdAt > now - grace) {
                    return row.createdAt + grace - now;
                }

                // claim and settle
                const claimed = await Settlement.claim(store.database, key, now, grace);
                if (claimed === undefined) {
                    return row.claimedAt === null ? 0 : row.claimedAt + grace - now;
                }
                await Settlement.settle(store, claimed, now);

                return undefined;
            },
        };
    },

    /** Refuse a call whose settlement a settler claimed after its grace passed. */
    refuseOutlasted(context: MethodContext): never {
        throw ObjectError.of(
            "SETTLEMENT_EXPIRED",
            `${context.object.name}.${context.name} exceeded its settlement grace period`,
        );
    },
};
