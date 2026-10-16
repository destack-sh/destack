import { placement } from "@destack/account/object";
import { identityKey } from "@destack/account/identity";
import { canonicalize, Digest, type Identifier } from "@destack/schema";
import { accessInvitation, decisionTables } from "@destack/access";
import { call, journal, storeTables } from "@destack/object";
import {
    and,
    type DatabaseConnection,
    eq,
    Order,
    type Row,
    TABLE,
    type Table,
    type Transaction,
} from "@destack/db";
import { Recipient } from "@destack/identity";
import { CallError } from "@destack/runtime/error";
import { Feed, Replica, replica, replicaPosition } from "@destack/sync";
import { type Fence, restore, snapshot, transfer } from "../../object/index.ts";
import type { Handoff } from "./handoff.ts";
import type { Clock } from "@destack/environment";

/** The rows one digest page reads: 5000 rows of about 500 B keep a page near 2.5 MB. */
const DIGEST_PAGE_ROWS = 5000;

/** The tables outside a replica's digest. */
const UNDIGESTED_TABLES: ReadonlySet<Table> = new Set([
    transfer.table,
    identityKey,
    journal,
    ...call.tables,
]);

/** The name of the replicas a transfer's target follows its source's rows through. */
const TRANSFER_REPLICA = "transfer";

/** The tables a machine keeps for itself. */
const MACHINE_TABLES: ReadonlySet<Table> = new Set([
    ...storeTables.filter((table) => !decisionTables.includes(table) && table !== accessInvitation),
    snapshot.table,
    restore.table,
    placement.table,
]);

/** A space's move of its database. */
export class SpaceReplica {
    /** The space's database on this side of the move. */
    readonly database: DatabaseConnection;
    /** The space moving. */
    readonly spaceId: Identifier<"space">;
    /** The tables with the space's rows, parents before the tables referencing them. */
    readonly tables: readonly Table[];
    /** The replica the target follows the space's rows through. */
    readonly replica: Replica;

    /** Plan the replica of a space's rows in every table whose scope admits the space. */
    static of(database: DatabaseConnection, spaceId: Identifier<"space">): SpaceReplica {
        // take the tables the database owns with the space's rows
        const tables = database.tables.filter((table) => {
            const scope = table[TABLE].columns["scope"];

            return (
                scope !== undefined &&
                scope.definition.schema.safeParse(spaceId).success &&
                !database.replicates(table) &&
                !MACHINE_TABLES.has(table) &&
                table !== transfer.table
            );
        });

        // move the transfers ahead of the rows referencing them where the database keeps them
        const ordered = database.tables.includes(transfer.table)
            ? [transfer.table, ...tables]
            : tables;

        return new SpaceReplica({ database, spaceId, tables: ordered });
    }

    /** Plan the replica of a space's rows in some tables of a database. */
    constructor(options: {
        readonly database: DatabaseConnection;
        readonly spaceId: Identifier<"space">;
        readonly tables: readonly Table[];
    }) {
        // keep the database, the space and the tables with its rows
        this.database = options.database;
        this.spaceId = options.spaceId;
        this.tables = options.tables;
        this.replica = new Replica({
            name: TRANSFER_REPLICA,
            scope: options.spaceId,
            tables: options.tables,
        });
    }

    /** Follow the space's rows and the replica's registration and position at a clock. */
    feed(clock: Clock): Feed {
        return new Feed(this.database, [...this.replica.tables, replica, replicaPosition], {
            clock,
        });
    }

    /** Apply the source's capture of every column of the space's rows. */
    async capture(
        source: Handoff,
        fenced: Fence,
        recipient: Recipient,
        open: (table: Table, row: Row) => Promise<Row>,
        clock: Clock,
    ): Promise<void> {
        // read the source's capture and apply it
        const pages = source.capture({ spaceId: this.spaceId, recipient: recipient.key });
        let isComplete = false;
        for await (const page of this.replica.apply(this.database, pages, { open, clock })) {
            isComplete = page.complete;
        }

        // refuse a capture that stopped before its end
        if (!isComplete) {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `the capture of ${this.spaceId} stopped before its end`,
            });
        }

        // verify the rows
        const digest = await SpaceReplica.digest(this.database, this.digestTables(), this.spaceId);
        if (digest !== fenced.digest) {
            throw new CallError("CONFLICT", {
                message: `the replica of ${this.spaceId} differs from its fenced source`,
            });
        }
    }

    /** Own the replicated rows unless owned already. */
    async promote(transaction: Transaction): Promise<void> {
        if (await this.replica.isRecorded(transaction.view())) {
            await this.replica.promote(transaction);
        }
    }

    /** Delete the space's rows children first, as the replica wrote them. */
    async delete(): Promise<void> {
        await this.database.transaction(
            async (transaction) => {
                await transaction.log.asReplica(async () => {
                    for (const table of this.tables.toReversed()) {
                        const scope = table[TABLE].column("scope");
                        await transaction.delete(table).where(eq(scope, this.spaceId));
                    }
                });
            },
            { constraints: "deferred" },
        );
    }

    /** List the tables the digest covers: those keeping the space's rows alike on both sides. */
    digestTables(): Table[] {
        return this.tables.filter((table) => !UNDIGESTED_TABLES.has(table));
    }

    /** Digest every column of a space's rows in some tables page by page in key order. */
    static async digest(
        database: DatabaseConnection,
        tables: readonly Table[],
        spaceId: string,
    ): Promise<string> {
        // digest each table's rows of the space a page at a time and seek past the last key read
        const pages: string[] = [];
        for (const table of tables) {
            const order = Order.complete([], table);
            const inSpace = eq(table[TABLE].column("scope"), spaceId);
            let last: Row | undefined;
            do {
                // read the next page of the space's rows in key order
                const rows = await database
                    .select()
                    .from(table)
                    .where(and(inSpace, last && Order.after(order, table, last)))
                    .orderBy(...Order.render(order, table))
                    .limit(DIGEST_PAGE_ROWS);
                last = rows.length === DIGEST_PAGE_ROWS ? rows.at(-1) : undefined;

                // keep the digest of every column of a page with rows
                if (rows.length > 0) {
                    const values = rows.map((row) => table[TABLE].encode(row));
                    pages.push(await Digest.of(canonicalize([table[TABLE].sqlName, values])));
                }
            } while (last !== undefined);
        }

        return Digest.of(pages.join(""));
    }
}
