import { quote } from "../dialect/quote.ts";
import type { Session } from "../database/session.ts";
import { v7 } from "uuid";
import { schema } from "@destack/schema";
import type { ConnectionState } from "../database/connection.ts";
import { LOG_TRANSACTION } from "./schema.ts";
import { relation } from "../table/namespace.ts";
import { FullTextIndex } from "../search/search.ts";

/** A transaction's log identity: a SQLite marker, else PostgreSQL's transaction id. */
export class TransactionIdentity {
    /** The session the transaction runs in. */
    readonly #session: Session;
    /** The connection's state: whether it has a log, and the searched tables. */
    readonly #connection: ConnectionState;
    /** The stamped SQLite marker, absent before the first logged write. */
    #stamped: Promise<string | undefined> | undefined;
    /** Whether the transaction looked for its marker before, so a log it finds may be its log. */
    #isMarking = false;

    /** Prepare the identity of a transaction in a session. */
    constructor(session: Session, connection: ConnectionState) {
        this.#session = session;
        this.#connection = connection;
    }

    /** Read the identity, stamping it as PostgreSQL's `pg_current_xact_id` assigns it. */
    async id(): Promise<string | undefined> {
        // assign PostgreSQL's id, else stamp the SQLite marker, absent without a log
        if (this.#session.dialect === "postgresql") {
            return this.#assigned();
        }
        await this.stamp();

        return await this.#stamped;
    }

    /** Whether a SQLite marker is stamped. */
    get isStamped(): boolean {
        return this.#stamped !== undefined;
    }

    /** Submit a SQLite marker once ahead of the first logged write, in statement order. */
    stamp(): Promise<unknown> | undefined {
        // leave PostgreSQL, whose trigger reads the transaction id
        if (this.#session.dialect === "postgresql") {
            return undefined;
        }

        // submit the marker in statement order, waiting only while finding the log
        const isKnown = this.#connection.isLogged;
        if (this.#stamped === undefined) {
            // forget a stamp that found no log
            const marking = this.#mark();
            this.#stamped = marking;
            marking.then(
                (id) => {
                    if (id === undefined && this.#stamped === marking) {
                        this.#stamped = undefined;
                    }
                },
                () => undefined,
            );
        }

        return isKnown ? undefined : this.#stamped;
    }

    /** Forget a stamp that a rolled back savepoint undid. */
    unstamp(): void {
        this.#stamped = undefined;
    }

    /** Update a stamped transaction's full-text rows, then clear its marker before commit. */
    async close(): Promise<void> {
        // leave a transaction that logged nothing
        if ((await this.#stamped) === undefined) {
            return;
        }

        // update the full-text rows, then clear the marker
        await FullTextIndex.update(
            this.#session,
            this.#connection.searched,
            this.#connection.namespace,
        );
        const marker = quote(relation(LOG_TRANSACTION, this.#connection.namespace));
        await this.#session.run(`DELETE FROM ${marker} WHERE slot = 1`, []);
    }

    /** Read PostgreSQL's transaction id, assigning it. */
    async #assigned(): Promise<string> {
        const [row] = await this.#session.all("SELECT pg_current_xact_id()::TEXT AS id", []);

        return schema.string().parse(row?.["id"]);
    }

    /** Write the SQLite marker the change triggers stamp each entry with, once the log exists. */
    async #mark(): Promise<string | undefined> {
        // look for the log, remembering one found on the transaction's first mark
        const connection = this.#connection;
        const isFirst = !this.#isMarking;
        this.#isMarking = true;
        if (!connection.isLogged) {
            const found = await this.#session.values(
                "SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ?",
                [relation(LOG_TRANSACTION, connection.namespace)],
            );
            if (found.length === 0) {
                return undefined;
            }
            if (isFirst) {
                connection.isLogged = true;
            }
        }

        // write the marker
        const marker = quote(relation(LOG_TRANSACTION, connection.namespace));
        const id = v7();
        await this.#session.run(`INSERT INTO ${marker} (slot, id) VALUES (1, ?)`, [id]);

        return id;
    }
}
