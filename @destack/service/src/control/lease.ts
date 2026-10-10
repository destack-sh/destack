import {
    and,
    defineTable,
    eq,
    integer,
    lt,
    or,
    sql,
    text,
    type DatabaseConnection,
} from "@destack/db";
import { present } from "@destack/schema";

/** The default lease duration in milliseconds, renewed at a third of it. */
export const LEASE_MILLISECONDS = 15_000;

/** The share of a lease's duration left below which its holder renews it: two thirds, renewing once a third passed. */
const RENEWAL_REMAINING = 2 / 3;

/** The instance reconciling a controller's key, and until when. */
export const controllerLease = defineTable("controller_lease", {
    /** The controller's name. */
    controller: text("controller").primaryKey(),
    /** The key. */
    key: text("key").primaryKey(),
    /** The instance with the lease. */
    holder: text("holder").notNull(),
    /** The count of takeovers that fences writes. */
    epoch: integer("epoch").notNull(),
    /** The lapse time, in UTC epoch milliseconds. */
    expiresAt: integer("expires_at").notNull(),
});

/** The leases of instances over one database. */
export const Leases = {
    /** Acquire or renew a lease, returning its epoch or the time the other holder's lease lapses. */
    async acquire(
        database: DatabaseConnection,
        controller: string,
        key: string,
        holder: string,
        duration: number,
        now: number,
    ): Promise<{ readonly epoch: number } | { readonly lapsesAt: number }> {
        // keep a recent lease of ours without writing
        const [held] = await database
            .select({
                holder: controllerLease.holder,
                epoch: controllerLease.epoch,
                expiresAt: controllerLease.expiresAt,
            })
            .from(controllerLease)
            .where(and(eq(controllerLease.controller, controller), eq(controllerLease.key, key)));
        if (held?.holder === holder && held.expiresAt - now > duration * RENEWAL_REMAINING) {
            return { epoch: held.epoch };
        }

        // take the lease when free, lapsed or already ours
        const [taken] = await database
            .insert(controllerLease)
            .values({ controller, key, holder, epoch: 1, expiresAt: now + duration })
            .onConflictDoUpdate({
                target: [controllerLease.controller, controllerLease.key],
                set: {
                    holder,
                    expiresAt: now + duration,
                    epoch: sql`CASE WHEN ${controllerLease.holder} = ${holder} THEN ${controllerLease.epoch} ELSE ${controllerLease.epoch} + 1 END`,
                },
                setWhere: or(
                    eq(controllerLease.holder, holder),
                    lt(controllerLease.expiresAt, now),
                ),
            })
            .returning({ epoch: controllerLease.epoch });
        if (taken !== undefined) {
            return { epoch: taken.epoch };
        }

        // read when the other holder's lease lapses
        const [other] = await database
            .select({ expiresAt: controllerLease.expiresAt })
            .from(controllerLease)
            .where(and(eq(controllerLease.controller, controller), eq(controllerLease.key, key)));

        return { lapsesAt: present(other, "the other holder's lease").expiresAt };
    },

    /** Release a holder's leases. */
    async release(database: DatabaseConnection, holder: string): Promise<void> {
        await database.delete(controllerLease).where(eq(controllerLease.holder, holder));
    },
};
