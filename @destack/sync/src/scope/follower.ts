import { eq, type DatabaseConnection, type Table } from "@destack/db";
import type { LogPosition } from "@destack/db/log";
import type { QueryPage } from "../query/index.ts";
import type { Replica } from "../replica/index.ts";
import { Scope } from "./scope.ts";

/** Copies each scope of a database's chain from a source, one follow key per scope. */
export class ChainFollower {
    /** The follower's name in reports. */
    readonly name = "chain";
    /** The scope rows with changes that update the chain. */
    readonly watches: readonly Table[] = [Scope.table];
    /** Follow every scope at once. */
    readonly concurrency = Infinity;
    /** The database holding the copies. */
    readonly database: DatabaseConnection;
    /** The scope at the bottom of the chain. */
    readonly scope: string;
    /** Build a scope's copy. */
    readonly #replica: (scope: string) => Replica;
    /** Stream a scope's copy from a position, or a snapshot without one. */
    readonly #watch: (
        scope: string,
        after: LogPosition | undefined,
        signal: AbortSignal,
    ) => AsyncIterable<QueryPage>;

    /** Copy the chain above a scope into a database. */
    constructor(
        database: DatabaseConnection,
        scope: string,
        replica: (scope: string) => Replica,
        watch: (
            scope: string,
            after: LogPosition | undefined,
            signal: AbortSignal,
        ) => AsyncIterable<QueryPage>,
    ) {
        // hold the database, the bottom scope, and how to build and stream each copy
        this.database = database;
        this.scope = scope;
        this.#replica = replica;
        this.#watch = watch;
    }

    /** List the scope and, once its copy arrives, the scopes above it up to the universe. */
    async list(): Promise<readonly string[]> {
        // read the ancestors the scope's copy holds
        const [copy] = await this.database
            .select({ ancestors: Scope.table.ancestors })
            .from(Scope.table)
            .where(eq(Scope.table.scope, this.scope));

        return copy === undefined
            ? [this.scope]
            : [this.scope, ...copy.ancestors, Scope.universe.id];
    }

    /** Follow one scope's copy from its recorded position until the signal aborts. */
    async follow(scope: string, signal: AbortSignal): Promise<void> {
        await this.#replica(scope).follow(
            this.database,
            (after, stream) => this.#watch(scope, after, stream),
            signal,
        );
    }
}
