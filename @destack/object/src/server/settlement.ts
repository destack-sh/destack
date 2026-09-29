import { and, eq, isNull, lte, or, type DatabaseConnection, type Table } from "@destack/db";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service";
import type { Controller } from "@destack/service/control";
import { Call } from "../method/call.ts";
import { settlement } from "../method/settlement.ts";
import type { Method } from "../method/method.ts";
import type { ObjectServer } from "./server.ts";
import { SystemAuthorization } from "./authorization.ts";
import { Duration } from "../object/duration.ts";

/** The settlement and claim grace, in milliseconds: a minute, above any live server's settling. */
const SETTLE_GRACE_MILLISECONDS = 60_000;

/** The parts of a server settling calls. */
type SettlingServer = Pick<ObjectServer, "objects" | "database" | "authorizer">;

/** One settlement row. */
type SettlementRow = typeof settlement.$inferSelect;

/** Settle calls' external work once, as try, confirm and cancel. */
export const Settlement = {
    /** Reserve the call's settlement under its key. */
    async reserve(database: DatabaseConnection, call: Call): Promise<Call> {
        const key = call.key!;
        await database
            .insert(settlement)
            .values({
                id: key,
                object: call.object.name,
                method: call.name,
                scope: call.scope,
                target: call.id ?? null,
                prepared: null,
                createdAt: call.now,
            })
            .onConflictDoNothing();

        return call;
    },

    /** Commit the call's prepared value and refuse a claimed settlement. */
    async commit(transaction: DatabaseConnection, call: Call, prepared: unknown): Promise<void> {
        const committed = await transaction
            .update(settlement)
            .set({ prepared: { value: schema.json().parse(prepared) } })
            .where(
                and(
                    eq(settlement.id, call.key!),
                    isNull(settlement.prepared),
                    isNull(settlement.claimedAt),
                ),
            )
            .returning({ id: settlement.id });
        if (committed.length === 0) {
            throw new ServiceError("UNAVAILABLE", {
                message: `${call.object.name}.${call.name} outlasted its settlement grace`,
            });
        }
    },

    /** Claim a settlement unless another claim is fresh. */
    async claim(
        database: DatabaseConnection,
        key: string,
        now: number,
        grace = SETTLE_GRACE_MILLISECONDS,
    ): Promise<SettlementRow | undefined> {
        const [claimed] = await database
            .update(settlement)
            .set({ claimedAt: now })
            .where(
                and(
                    eq(settlement.id, key),
                    or(isNull(settlement.claimedAt), lte(settlement.claimedAt, now - grace)),
                ),
            )
            .returning();

        return claimed;
    },

    /** Forget a settlement once its call settled. */
    async forget(database: DatabaseConnection, key: string): Promise<void> {
        await database.delete(settlement).where(eq(settlement.id, key));
    },

    /** Settle one left settlement as the system, then forget it. */
    async settle(server: SettlingServer, row: SettlementRow, now: number): Promise<void> {
        // find the method
        const object = server.objects.find((served) => served.name === row.object);
        if (object === undefined) {
            throw new TypeError(`settlement ${row.id} names unserved object ${row.object}`);
        }
        const method = (object.methods as Readonly<Record<string, Method>>)[row.method]!;

        // rebuild the call as the system
        const table = object.table as Table & Record<string, never>;
        const [target] =
            row.target === null
                ? []
                : await server.database.select().from(table).where(eq(table.id, row.target));
        const authorization = await SystemAuthorization.open(
            server.authorizer,
            server.database,
            row.scope,
            now,
        );
        const call = new Call({
            object,
            name: row.method,
            method,
            scope: row.scope,
            chain: authorization.chain,
            input: {},
            ...(row.target === null ? {} : { id: row.target }),
            ...(target === undefined ? {} : { target: target as never }),
            key: row.id,
            database: server.database,
            now,
            isPredicted: false,
            authorization,
            objects: server.objects,
        });

        // confirm a committed call
        if (row.prepared !== null) {
            const prepared = row.prepared.value;
            await method.settle!(call.with({ prepared }), prepared, true);
        }
        // cancel an uncommitted call
        else {
            await method.settle!(call, undefined, false);
        }
        await Settlement.forget(server.database, row.id);
    },

    /** Settle what servers left once the grace passes. */
    controller(server: SettlingServer, options: { readonly grace?: Duration } = {}): Controller {
        const grace =
            options.grace === undefined
                ? SETTLE_GRACE_MILLISECONDS
                : Duration.milliseconds(options.grace);

        return {
            name: "settlement",
            watches: [settlement as Table],
            keys: (change) => [String((change.key as { id: string }).id)],
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
