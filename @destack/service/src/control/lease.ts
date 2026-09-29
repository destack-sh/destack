import {
    and,
    defineTable,
    eq,
    integer,
    lt,
    or,
    primaryKey,
    sql,
    text,
    type DatabaseConnection,
} from "@destack/db";

/**
 * The default lease duration, in milliseconds.
 *
 * Kubernetes leases last 15 seconds and renew at a third of that.
 */
export const LEASE_MILLISECONDS = 15_000;

/** The instance reconciling a controller's key, and until when. */
export const controllerLease = defineTable(
    "controller_lease",
    {
        /** The controller's name. */
        controller: text("controller").notNull(),
        /** The key. */
        key: text("key").notNull(),
        /** The instance holding the lease. */
        holder: text("holder").notNull(),
        /** The count of takeovers, which fences writes. */
        epoch: integer("epoch").notNull(),
        /** The lapse time, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    {
        constraints: (lease) => [
            primaryKey({ name: "controller_lease_key", columns: [lease.controller, lease.key] }),
        ],
    },
);

/** The leases of instances over one database. */
export const Leases = {
    /** Acquire or renew a lease, returning its epoch or the time the other holder's lease lapses. */
    async acquire(
        database: DatabaseConnection,
        controller: string,
        key: string,
        holder: string,
        duration: number,
    ): Promise<{ readonly epoch: number } | { readonly lapsesAt: number }> {
        // take the lease when free, lapsed or held
        const now = Date.now();
        const [held] = await database
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
        if (held !== undefined) {
            return { epoch: held.epoch };
        }

        // read the other holder's lapse time
        const [other] = await database
            .select({ expiresAt: controllerLease.expiresAt })
            .from(controllerLease)
            .where(and(eq(controllerLease.controller, controller), eq(controllerLease.key, key)));

        return { lapsesAt: other?.expiresAt ?? now };
    },

    /** Release a holder's leases. */
    async release(database: DatabaseConnection, holder: string): Promise<void> {
        await database.delete(controllerLease).where(eq(controllerLease.holder, holder));
    },
};
