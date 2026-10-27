import { placement } from "@destack/account/object";
import { identityKey } from "@destack/account/identity";
import { canonicalize, Digest, type Identifier } from "@destack/schema";
import { accessInvitation, decisionTables } from "@destack/access";
import { call, journal, SYSTEM_TABLES } from "@destack/object";
import {
    and,
    type DatabaseConnection,
    inArray,
    Order,
    type Row,
    type RowImage,
    Snapshot,
    TABLE,
    type Table,
    type Transaction,
} from "@destack/db";
import { Recipient } from "@destack/identity";
import { Feed, Replica, replica, replicaPosition, Scope } from "@destack/sync";
import { type Fence, restore, snapshot, transfer } from "../../object/index.ts";
import type { Handoff } from "./handoff.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** The rows one digest page reads: 5000 rows of about 500 B give a page near 2.5 MB. */
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

/** The tables a machine stores for itself. */
const MACHINE_TABLES: ReadonlySet<Table> = new Set([
    ...SYSTEM_TABLES.filter(
        (table) => !decisionTables.includes(table) && table !== accessInvitation,
    ),
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
    /** The space and the scopes below it, whose rows the replica moves. */
    readonly scopes: readonly string[];
    /** The replica the target follows the space's rows through. */
    readonly replica: Replica;

    /** Plan the replica of a space's subtree's rows in the database's tables and the given ones. */
    static async of(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        beside: readonly Table[],
    ): Promise<SpaceReplica> {
        // take the tables of the database and the given ones that have the space's rows
        const tables = [...database.tables, ...beside].filter((table) => {
            const scope = table[TABLE].columns["scope"];

            return (
                scope !== undefined &&
                scope.definition.schema.safeParse(spaceId).success &&
                !database.replicates(table) &&
                !MACHINE_TABLES.has(table) &&
                table !== transfer.table
            );
        });

        // move the transfers ahead of the rows referencing them when the database has them
        const ordered = database.tables.includes(transfer.table)
            ? [transfer.table, ...tables]
            : tables;

        // replicate the rows of the space and of every scope below it
        const scopes = await Scope.subtree(Snapshot.live(database), spaceId);

        return new SpaceReplica({ database, spaceId, scopes, tables: ordered });
    }

    /** Plan the replica of a space's rows in some tables of a database. */
    constructor(options: {
        readonly database: DatabaseConnection;
        readonly spaceId: Identifier<"space">;
        readonly scopes: readonly string[];
        readonly tables: readonly Table[];
    }) {
        // store the database, the space and the tables with its subtree's rows
        this.database = options.database;
        this.spaceId = options.spaceId;
        this.tables = options.tables;
        this.scopes = options.scopes;
        this.replica = new Replica({
            name: TRANSFER_REPLICA,
            scope: options.spaceId,
            tables: options.tables,
            scopes: new Map(options.tables.map((table) => [table, options.scopes])),
        });
    }

    /** Plan the same replica over a known subtree, such as the one a source fenced. */
    within(scopes: readonly string[]): SpaceReplica {
        return new SpaceReplica({
            database: this.database,
            spaceId: this.spaceId,
            scopes,
            tables: this.tables,
        });
    }

    /** Follow the space's rows and the replica's registration and position. */
    feed(): Feed {
        return new Feed(this.database, [...this.replica.tables, replica, replicaPosition]);
    }

    /** Apply the source's capture of every column of the space's rows. */
    async capture(
        source: Pick<Handoff, "capture">,
        fenced: Fence,
        recipient: Recipient,
        open: (table: Table, row: Row) => Promise<Row>,
    ): Promise<void> {
        // read the source's capture and apply it
        const pages = source.capture({ spaceId: this.spaceId, recipient: recipient.key });
        let isComplete = false;
        for await (const page of this.replica.apply(this.database, pages, { open })) {
            isComplete = page.complete;
        }

        // refuse a capture that stopped before its end
        if (!isComplete) {
            throw StackError.of(
                "CAPTURE_STOPPED",
                `the capture of ${this.spaceId} stopped before its end`,
            );
        }

        // verify the rows
        const digest = await SpaceReplica.digest(this.database, this.digestTables(), this.scopes);
        if (digest !== fenced.digest) {
            throw StackError.of(
                "REPLICA_DIFFERS",
                `the replica of ${this.spaceId} differs from its fenced source`,
            );
        }
    }

    /** Promote the replicated rows unless already promoted. */
    async promote(transaction: Transaction): Promise<void> {
        if (await this.replica.isRecorded(transaction.view())) {
            await this.replica.promote(transaction);
        }
    }

    /** Rewrite a row of a takeover outside methods: a moved resource's, or its own restore. */
    async rewrite<Rewritten extends Table>(
        transaction: Transaction,
        table: Rewritten,
        before: RowImage<Rewritten>,
        after: RowImage<Rewritten> | null,
    ): Promise<void> {
        await transaction.write({ table, before, after });
    }

    /** Delete the space's subtree's rows children first, as the replica wrote them. */
    async delete(): Promise<void> {
        const { scopes } = this;
        await this.database.transaction(
            async (transaction) => {
                await transaction.log.asReplica(async () => {
                    for (const table of this.tables.toReversed()) {
                        const scope = table[TABLE].column("scope");
                        await transaction.delete(table).where(inArray(scope, scopes));
                    }
                });
            },
            { constraints: "deferred" },
        );
    }

    /** List the tables the digest covers: those that store the space's rows alike on both sides. */
    digestTables(): Table[] {
        return this.tables.filter((table) => !UNDIGESTED_TABLES.has(table));
    }

    /** Digest every column of a subtree's rows in some tables page by page in key order. */
    static async digest(
        database: DatabaseConnection,
        tables: readonly Table[],
        scopes: readonly string[],
    ): Promise<Digest> {
        // digest each table's rows of the subtree a page at a time and seek past the last key read
        const { crypto } = database.state.environment;
        const pages: string[] = [];
        for (const table of tables) {
            const order = Order.complete([], table);
            const inSpace = inArray(table[TABLE].column("scope"), scopes);
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

                // add the digest of every column of a page with rows
                if (rows.length > 0) {
                    const values = rows.map((row) => table[TABLE].encode(row));
                    pages.push(
                        await Digest.of(canonicalize([table[TABLE].sqlName, values]), crypto),
                    );
                }
            } while (last !== undefined);
        }

        return Digest.of(pages.join(""), crypto);
    }
}
