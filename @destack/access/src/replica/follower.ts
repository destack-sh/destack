import { eq, type DatabaseConnection, type Table } from "@destack/db";
import type { LogPosition } from "@destack/db/log";
import type { QueryPage } from "@destack/sync";
import { Authorizer } from "../authorizer/authorizer.ts";
import type { TypeReference } from "../policy/policy.ts";
import { accessScope } from "../scope/table.ts";

/** A relay of the access of one scope and the scopes containing it, such as a space's holder relaying it to an installation. */
export interface AccessRelay {
    /** The scope at the bottom of the relayed chain. */
    readonly scope: string;
    /** Stream the copy of one scope in the chain for a database holding some object types, from a position or else a snapshot. */
    watch(
        request: {
            readonly scope: string;
            readonly held: readonly TypeReference[];
            readonly after?: LogPosition;
        },
        signal: AbortSignal,
    ): AsyncIterable<QueryPage>;
}

/** Copies the access of a relay's scope and every scope containing it into a database, one key per scope. */
export class AccessFollower {
    /** The follower's name in reports. */
    readonly name = "access";
    /** The scope copies, whose changes list the containing scopes again. */
    readonly watches: readonly Table[] = [accessScope];
    /** Follow every scope at once. */
    readonly concurrency = Infinity;
    /** The database holding the copies. */
    readonly database: DatabaseConnection;
    /** The object types whose access rows the database holds itself. */
    readonly held: readonly TypeReference[];
    /** The relay streaming each scope's copy. */
    readonly relay: AccessRelay;

    /** Copy the access of a relay's chain into a database holding some object types. */
    constructor(database: DatabaseConnection, held: readonly TypeReference[], relay: AccessRelay) {
        // hold the database, its object types and the relay
        this.database = database;
        this.held = held;
        this.relay = relay;
    }

    /** List the relay's scope and, once its copy arrives, the scopes containing it. */
    async list(): Promise<readonly string[]> {
        // read the ancestors the scope's copy names
        const [copy] = await this.database
            .select({ ancestors: accessScope.ancestors })
            .from(accessScope)
            .where(eq(accessScope.scope, this.relay.scope));

        return copy === undefined ? [this.relay.scope] : [this.relay.scope, ...copy.ancestors];
    }

    /** Follow one scope's copy from its recorded position until the signal aborts. */
    async follow(scope: string, signal: AbortSignal): Promise<void> {
        // resume the relay's stream from the copy's position each time it ends
        const held = this.held;
        await Authorizer.replicaOf(scope, held).follow(
            this.database,
            (after, stream) =>
                this.relay.watch(
                    { scope, held, ...(after === undefined ? {} : { after }) },
                    stream,
                ),
            signal,
        );
    }
}
