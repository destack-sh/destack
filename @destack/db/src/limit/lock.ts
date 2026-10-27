import type { DatabaseConnection } from "../database/index.ts";
import { and, eq, sql } from "../sql/index.ts";
import { defineTable, text } from "../table/index.ts";

/** The holders of named locks, after Chubby's locks. */
export const lock = defineTable("destack_lock", {
    /** What is locked, such as a controller. */
    name: text("name").primaryKey(),
    /** The instance of the holder, whose liveness keeps the lock. */
    instance: text("instance").notNull(),
    /** The holder within its instance, such as a control loop. */
    holder: text("holder").notNull(),
});

/** A named lock over one database for one holder. */
export class Lock {
    /** The database keeping the lock. */
    readonly database: DatabaseConnection;
    /** What is locked. */
    readonly name: string;
    /** The holder's instance. */
    readonly instance: string;
    /** The holder. */
    readonly holder: string;

    /** Describe a holder's lock of a name. */
    constructor(database: DatabaseConnection, name: string, instance: string, holder: string) {
        // keep the database, the name and the holder
        this.database = database;
        this.name = name;
        this.instance = instance;
        this.holder = holder;
    }

    /** Take the lock unless another live holder has it, writing only to take it over. */
    async take(isLive: (instance: string) => boolean): Promise<boolean> {
        // keep the lock this holder has, and refuse one another live holder has
        const { name, instance, holder } = this;
        const [row] = await this.database
            .select({ instance: lock.instance, holder: lock.holder })
            .from(lock)
            .where(eq(lock.name, name));
        if (row?.holder === holder) {
            return true;
        } else if (row !== undefined && isLive(row.instance)) {
            return false;
        }

        // take a free lock or a stopped holder's, unless another holder took it meanwhile
        const [taken] = await this.database.transaction(
            async (transaction) =>
                await transaction
                    .insert(lock)
                    .values({ name, instance, holder })
                    .onConflictDoUpdate({
                        target: [lock.name],
                        set: { instance, holder },
                        setWhere: row === undefined ? sql`FALSE` : eq(lock.holder, row.holder),
                    })
                    .returning({ holder: lock.holder }),
        );

        return taken !== undefined;
    }

    /** Release the lock while this holder has it. */
    async release(): Promise<void> {
        await this.database.transaction(
            async (transaction) =>
                await transaction
                    .delete(lock)
                    .where(and(eq(lock.name, this.name), eq(lock.holder, this.holder))),
        );
    }
}
