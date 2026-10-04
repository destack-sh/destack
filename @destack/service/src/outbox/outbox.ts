import {
    Change,
    and,
    asc,
    defineTable,
    eq,
    index,
    integer,
    text,
    type DatabaseConnection,
    inArray,
} from "@destack/db";
import { canonicalize, schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import type { Controller } from "../control/index.ts";

/** The longest delivery, in milliseconds. */
const DELIVERY_TIMEOUT_MILLISECONDS = 30_000;

/** The messages waiting for their destination. */
export const outbox = defineTable(
    "outbox",
    {
        /** The message key. */
        id: text("id").primaryKey(),
        /** The destination's name. */
        destination: text("destination").notNull(),
        /** The message as canonical JSON. */
        message: text("message").notNull(),
        /** The commit time. */
        recordedAt: integer("recorded_at").notNull(),
    },
    {
        constraints: (entry) => [
            index("outbox_order").on(entry.destination, entry.recordedAt, entry.id),
        ],
    },
);

/** An outbox destination. */
export interface Address<Message = unknown> {
    /** The destination's name. */
    readonly name: string;
    /** The messages' schema. */
    readonly message: schema.Schema<Message>;
}

/** A receiver of an outbox's messages, accepting each once by key. */
export interface Destination<Message = unknown> extends Address<Message> {
    /** The most messages one delivery sends. */
    readonly batch: number;
    /** Accept messages oldest first and once per key. */
    accept(
        messages: readonly Message[],
        options: { readonly signal: AbortSignal },
    ): Promise<unknown>;
}

/** Messages sent with their transactions and delivered at least once, in order. */
export class Outbox {
    /** The database with the outbox. */
    readonly database: DatabaseConnection;

    /** Bind the outbox to its database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Append a message inside a transaction or in its own, once per key. */
    async append<Message>(
        destination: Address<Message>,
        id: string,
        message: Message,
        transaction?: DatabaseConnection,
    ): Promise<void> {
        // insert in the caller's transaction on this database
        if (transaction !== undefined) {
            if (!transaction.driver.transaction || transaction.state !== this.database.state) {
                throw new TypeError("outbox appends need a transaction on the outbox's database");
            }
            await this.#insert(destination, id, message, transaction);
        }
        // insert in a transaction of its own
        else {
            await this.database.transaction((own) => this.#insert(destination, id, message, own));
        }
    }

    /** Read a destination's pending messages, oldest first. */
    async read<Message>(destination: Address<Message>, limit: number): Promise<Message[]> {
        const rows = await this.#pending(destination, limit);

        return rows.map((row) => row.message);
    }

    /** Deliver a destination's oldest pending messages as one batch, returning how many. */
    async deliver<Message>(
        destination: Destination<Message>,
        signal?: AbortSignal,
    ): Promise<number> {
        // read the oldest batch
        const rows = await this.#pending(destination, destination.batch);
        if (rows.length === 0) {
            return 0;
        }

        // deliver within a timeout and remove the accepted messages
        const timeout = AbortSignal.timeout(DELIVERY_TIMEOUT_MILLISECONDS);
        await destination.accept(
            rows.map((row) => row.message),
            { signal: signal === undefined ? timeout : AbortSignal.any([signal, timeout]) },
        );
        await this.database.delete(outbox).where(
            inArray(
                outbox.id,
                rows.map((row) => row.id),
            ),
        );

        return rows.length;
    }

    /** Deliver a destination's messages as they commit. */
    controller<Message>(destination: Destination<Message>): Controller {
        return {
            name: destination.name,
            watches: [outbox],
            keys: (change) =>
                Change.after(change)?.["destination"] === destination.name
                    ? [destination.name]
                    : [],
            list: async () =>
                (await this.#pending(destination, 1)).length > 0 ? [destination.name] : [],
            reconcile: async () => {
                // look again while full batches remain
                const delivered = await this.deliver(destination);

                return delivered === destination.batch ? 0 : undefined;
            },
        };
    }

    /** Read a destination's pending messages, oldest first. */
    async #pending<Message>(destination: Address<Message>, limit: number) {
        const rows = await this.database
            .select({ id: outbox.id, message: outbox.message })
            .from(outbox)
            .where(eq(outbox.destination, destination.name))
            .orderBy(asc(outbox.recordedAt), asc(outbox.id))
            .limit(limit);

        return rows.map((row) => ({
            id: row.id,
            message: destination.message.parse(JSON.parse(row.message)),
        }));
    }

    /** Insert a message once per key. */
    async #insert<Message>(
        destination: Address<Message>,
        id: string,
        message: Message,
        transaction: DatabaseConnection,
    ): Promise<void> {
        // insert the message as canonical JSON
        const content = canonicalize(destination.message.parse(message));
        const inserted = await transaction
            .insert(outbox)
            .values({
                id,
                destination: destination.name,
                message: content,
                recordedAt: Date.now(),
            })
            .onConflictDoNothing()
            .returning({ id: outbox.id });
        if (inserted.length > 0) {
            return;
        }

        // refuse an existing key with another destination or other contents
        const [existing] = await transaction
            .select({ message: outbox.message })
            .from(outbox)
            .where(and(eq(outbox.id, id), eq(outbox.destination, destination.name)));
        if (existing === undefined || existing.message !== content) {
            throw new ServiceError("CONFLICT", {
                message: `${destination.name} message ${id} has conflicting contents`,
            });
        }
    }
}
