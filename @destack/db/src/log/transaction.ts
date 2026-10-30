import { quote } from "../dialect/quote.ts";
import type { Session } from "../database/session.ts";
import { schema, Uuid } from "@destack/schema";
import type { ConnectionState } from "../database/connection.ts";
import { CHANGED_AT_SETTING, COMMIT_TOKEN_SETTING, LOG, LOG_TRANSACTION } from "./schema.ts";
import { relation } from "../table/namespace.ts";
import { FullTextIndex } from "../search/search.ts";

/** The token a PostgreSQL commit notification sends back to its writer. */
interface CommitToken {
    /** The token. */
    readonly value: string;
    /** Settles once the token reached the writer's connection. */
    readonly delivered: Promise<void>;
    /** Whether the transaction wrote log rows, which makes its commit notify. */
    isLogged: Promise<boolean>;
}

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
    /** The PostgreSQL transaction's stamped time, absent before its first logged write. */
    #timed: Promise<unknown> | undefined;
    /** The token PostgreSQL's commit notification sends back, while the connection listens. */
    #token: CommitToken | undefined;

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

    /** Whether the transaction is stamped: its SQLite marker or its PostgreSQL time. */
    get isStamped(): boolean {
        return this.#stamped !== undefined || this.#timed !== undefined;
    }

    /** Stamp the transaction's time, and a SQLite marker, once ahead of its first logged write. */
    stamp(): Promise<unknown> | undefined {
        // submit PostgreSQL's transaction-local time once, in statement order
        if (this.#session.dialect === "postgresql") {
            this.#timed ??= this.#time();

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
        this.abandon();
        this.#stamped = undefined;
        this.#timed = undefined;
    }

    /** Count a committed transaction's token in flight on the clock until it is delivered. */
    commit(): void {
        // wait for the token of a transaction that logged, and forget one that logged nothing
        const token = this.#token;
        if (token === undefined) {
            return;
        }
        const { commits } = this.#connection;
        const delivered = token.isLogged.then(
            (isLogged) => (isLogged ? token.delivered : commits.forget(token.value)),
            () => commits.forget(token.value),
        );
        void this.#connection.clock.trackPeer(delivered).catch(() => commits.forget(token.value));
    }

    /** Stop waiting for the token of a transaction that rolled back. */
    abandon(): void {
        if (this.#token !== undefined) {
            this.#connection.commits.forget(this.#token.value);
            this.#token = undefined;
        }
    }

    /** Update a stamped transaction's full-text rows, then clear its marker before commit. */
    async close(): Promise<void> {
        // surface a failed PostgreSQL stamp, and ask in statement order whether the transaction logged
        if (this.#session.dialect === "postgresql") {
            await this.#timed;
            if (this.#token !== undefined) {
                this.#token.isLogged = this.#session
                    .values(
                        `SELECT EXISTS (SELECT FROM ${quote(LOG)} WHERE "transaction" = pg_current_xact_id_if_assigned()::TEXT AND sequence IS NULL)`,
                        [],
                    )
                    .then(([row]) => row?.[0] === true);
                this.#token.isLogged.catch(() => undefined);
            }

            return;
        }

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

    /** Set PostgreSQL's transaction-local time, and the token while the connection listens. */
    #time(): Promise<unknown> {
        // set the time alone without a listener to receive the token
        const time = String(this.#connection.clock.now());
        const { commits } = this.#connection;
        let timed: Promise<unknown>;
        if (!commits.isShared) {
            timed = this.#session.run("SELECT set_config($1, $2, true)", [
                CHANGED_AT_SETTING,
                time,
            ]);
        }
        // set the token beside the time, for the stamp trigger's commit notification to send
        else {
            const value = Uuid.v4(this.#connection.environment.random);
            this.#token = {
                value,
                delivered: commits.expect(value),
                isLogged: Promise.resolve(false),
            };
            timed = this.#session.run("SELECT set_config($1, $2, true), set_config($3, $4, true)", [
                CHANGED_AT_SETTING,
                time,
                COMMIT_TOKEN_SETTING,
                value,
            ]);
        }

        // surface a failure at the close
        timed.catch(() => undefined);

        return timed;
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
        const id = Uuid.v7(connection.environment);
        await this.#session.run(`INSERT INTO ${marker} (slot, id, changed_at) VALUES (1, ?, ?)`, [
            id,
            connection.clock.now(),
        ]);

        return id;
    }
}
