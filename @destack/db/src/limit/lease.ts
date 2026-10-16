import type { DatabaseConnection } from "../database/index.ts";
import { DatabaseError } from "../error/index.ts";
import { and, eq, lte, or, sql } from "../sql/index.ts";
import { defineTable, integer, text } from "../table/index.ts";

/** The share of a lease's time left above which its holder reuses it, renewing a third in. */
const REUSE_REMAINING = 2 / 3;

/** The holders of leased keys: one row per slot, each until it lapses, fencing a lapsed holder. */
export const lease = defineTable("destack_lease", {
    /** What is leased, such as a controller's name. */
    name: text("name").primaryKey(),
    /** Whose lease it is within the name, such as a controller's key. */
    key: text("key").primaryKey(),
    /** The slot, from zero to the key's slots minus one. */
    slot: integer("slot").primaryKey(),
    /** The holder of the slot, such as an instance or a run. */
    holder: text("holder").notNull(),
    /** The count of takeovers of the slot, which fences a lapsed holder's writes. */
    epoch: integer("epoch").notNull(),
    /** The lapse time, in UTC epoch milliseconds. */
    expiresAt: integer("expires_at").notNull(),
});

/** A holder's request for one slot of a leased key. */
export interface LeaseRequest {
    /** What is leased. */
    readonly name: string;
    /** Whose lease it is within the name. */
    readonly key: string;
    /** The holder taking a slot. */
    readonly holder: string;
    /** How long the slot lasts from now, in milliseconds. */
    readonly duration: number;
    /** The current time, in UTC epoch milliseconds. */
    readonly now: number;
    /** How many holders lease the key at once, one by default. */
    readonly slots?: number;
}

/** A slot the holder took, or the time the earliest taken slot lapses. */
export type LeaseOutcome =
    | {
          /** The holder acquired the slot. */
          readonly kind: "acquired";
          /** The slot. */
          readonly slot: number;
          /** The slot's epoch, which the holder's writes carry. */
          readonly epoch: number;
          /** When the slot lapses unless renewed, in UTC epoch milliseconds. */
          readonly expiresAt: number;
      }
    | {
          /** Every slot is taken. */
          readonly kind: "busy";
          /** When the earliest taken slot lapses, in UTC epoch milliseconds. */
          readonly lapsesAt: number;
      };

/** Leases of keys to holders, single or in slots, over one database. */
export const Lease = {
    /** Take a free, lapsed or own slot of a key, reusing an own slot with most of its time left. */
    async acquire(database: DatabaseConnection, request: LeaseRequest): Promise<LeaseOutcome> {
        // read the key's slots within its count, again after a racing holder took the chosen one
        const { name, key, holder, duration, now, slots = 1 } = request;
        for (let attempt = 0; attempt <= slots; attempt++) {
            const read = await database
                .select({
                    slot: lease.slot,
                    holder: lease.holder,
                    epoch: lease.epoch,
                    expiresAt: lease.expiresAt,
                })
                .from(lease)
                .where(and(eq(lease.name, name), eq(lease.key, key)));
            const rows = read.filter((row) => row.slot < slots);

            // reuse an own slot with most of its time left, without writing
            const own = rows.find((row) => row.holder === holder && row.expiresAt > now);
            if (own !== undefined && own.expiresAt - now > duration * REUSE_REMAINING) {
                return {
                    kind: "acquired",
                    slot: own.slot,
                    epoch: own.epoch,
                    expiresAt: own.expiresAt,
                };
            }

            // choose the own slot, else a missing one, else a lapsed one, else report the earliest lapse
            const missing = Array.from({ length: slots }, (_, slot) => slot).find(
                (slot) => !rows.some((row) => row.slot === slot),
            );
            const lapsed = rows.find((row) => row.expiresAt <= now);
            const slot = own?.slot ?? missing ?? lapsed?.slot;
            if (slot === undefined) {
                const lapsesAt = Math.min(...rows.map((row) => row.expiresAt));

                return { kind: "busy", lapsesAt };
            }

            // take the slot when free, lapsed or already ours, counting a takeover's epoch
            const expiresAt = now + duration;
            const [taken] = await database
                .insert(lease)
                .values({ name, key, slot, holder, epoch: 1, expiresAt })
                .onConflictDoUpdate({
                    target: [lease.name, lease.key, lease.slot],
                    set: {
                        holder,
                        expiresAt,
                        epoch: sql`CASE WHEN ${lease.holder} = ${holder} THEN ${lease.epoch} ELSE ${lease.epoch} + 1 END`,
                    },
                    setWhere: or(eq(lease.holder, holder), lte(lease.expiresAt, now)),
                })
                .returning({ epoch: lease.epoch });
            if (taken !== undefined) {
                return { kind: "acquired", slot, epoch: taken.epoch, expiresAt };
            }
        }

        // lose the race once every attempt lost its chosen slot to another holder
        throw new DatabaseError(
            "CONCURRENT_UPDATE",
            `every slot of lease ${name} ${key} changed while taken`,
        );
    },

    /** Release a holder's slots, of one key or of every key it has. */
    async release(
        database: DatabaseConnection,
        release: { readonly holder: string; readonly name?: string; readonly key?: string },
    ): Promise<void> {
        await database
            .delete(lease)
            .where(
                and(
                    eq(lease.holder, release.holder),
                    release.name === undefined ? undefined : eq(lease.name, release.name),
                    release.key === undefined ? undefined : eq(lease.key, release.key),
                ),
            );
    },
};
