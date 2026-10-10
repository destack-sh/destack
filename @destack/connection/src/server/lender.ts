import { Caller } from "@destack/access";
import { and, type DatabaseConnection, eq } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { schema } from "@destack/schema";
import type { ObjectReference, Subject } from "@destack/sync";
import { ConnectionError } from "../error/error.ts";
import { ConnectionRequest } from "../lend/lend.ts";
import { type Connection, type ConnectionKind, connection } from "../object/connection.ts";
import type { TokenSource } from "./token.ts";

/** The machine side of lending: it applies a connection's current credential to an installation's outbound request, which never sees the credential. */
export class Lender {
    /** The object store keeping the connections. */
    readonly #system: ObjectStore;
    /** The connections' current tokens, refreshed once per connection with the controller. */
    readonly #tokens: TokenSource;

    /** Lend the connections an object store keeps. */
    constructor(system: ObjectStore, tokens: TokenSource) {
        this.#system = system;
        this.#tokens = tokens;
    }

    /** Read the kind of a connection a database keeps, such as a machine refusing mTLS where it cannot present client certificates, absent for a connection it keeps none of. */
    static async kind(
        database: DatabaseConnection,
        reference: ObjectReference,
    ): Promise<ConnectionKind | undefined> {
        const [row] = await database
            .select({ protocol: connection.table.protocol })
            .from(connection.table)
            .where(
                and(
                    eq(connection.table.scope, reference.scope),
                    eq(connection.table.id, schema.identifier("connection").parse(reference.id)),
                ),
            );

        return row?.protocol.kind;
    }

    /** Apply an authorized connection's current credential to an outbound request, of a consumer it is bound to when one is named. */
    async lend(
        reference: ObjectReference,
        request: Request,
        now: number,
        consumer?: Subject,
    ): Promise<ConnectionRequest> {
        // lend a connection of another scope through the token its home issues to the bound consumer
        const home = this.#tokens.options.home;
        if (home !== undefined && consumer !== undefined && reference.scope !== consumer.scope) {
            const token = await home(reference.scope, consumer).connection.requestToken({
                scope: reference.scope,
                id: schema.identifier("connection").parse(reference.id),
            });

            return ConnectionRequest.token(token, request);
        }

        // require an authorized connection bound to the consumer
        const row = await this.#find(reference);
        if (consumer !== undefined && !(await this.#isConsumer(row, consumer))) {
            throw new ConnectionError(
                "UNAUTHORIZED",
                `connection ${row.id} is not bound to ${consumer.type} ${consumer.id}`,
            );
        } else if (row.conditions["Authorized"]?.status !== "true") {
            throw new ConnectionError("UNAUTHORIZED", `connection ${row.id} is not authorized`);
        }

        // apply its current credential
        const credential = await this.#tokens.current(this.#system, row, now);

        return ConnectionRequest.of(row.protocol, credential, request, now);
    }

    /** Report whether a consumer may borrow a connection, holding its lend permission. */
    async #isConsumer(row: Connection, consumer: Subject): Promise<boolean> {
        // decide the consumer's lend permission on the connection
        const { database } = this.#system;
        const now = this.#system.clock.now();
        const authorization = await this.#system.authorize(
            database,
            row.scope,
            Caller.of(consumer, now),
            { now },
        );
        const decision = await authorization.check(
            connection.permission("lend"),
            connection.reference(row.scope, row.id),
        );

        return decision.isAllowed;
    }

    /** Find a connection, refusing a missing one. */
    async #find(reference: ObjectReference): Promise<Connection> {
        const [row] = await this.#system.database
            .select()
            .from(connection.table)
            .where(
                and(
                    eq(connection.table.scope, reference.scope),
                    eq(connection.table.id, schema.identifier("connection").parse(reference.id)),
                ),
            );
        if (row === undefined) {
            throw new ConnectionError(
                "UNAUTHORIZED",
                `no connection ${reference.id} in ${reference.scope}`,
            );
        }

        return row;
    }
}
