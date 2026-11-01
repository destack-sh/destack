import type { DatabaseHost, DatabaseKind } from "@destack/db";
import type { ResourceRecord } from "@destack/resource";
import { type Identifier, schema } from "@destack/schema";
import { SPACE_PROVIDER } from "@destack/stack/object";

/** Store each space's main database in the space's own storage on its machine. */
export class SpaceDatabaseHost implements DatabaseHost {
    /** The provider code of the spaces' own databases. */
    readonly provider = SPACE_PROVIDER;
    /** The host of the machine's databases, one per reference. */
    readonly #host: DatabaseHost;
    /** Read the reference of a space's own storage. */
    readonly #reference: (spaceId: Identifier<"space">) => string;
    /** Resolve a reference as the host of the machine's databases does, for local files. */
    readonly resolve?: (reference: string) => string;

    /** Serve each space's own storage through a host of the machine's databases. */
    constructor(host: DatabaseHost, reference: (spaceId: Identifier<"space">) => string) {
        this.#host = host;
        this.#reference = reference;

        // resolve as the host does, when it stores local files
        if (host.resolve !== undefined) {
            this.resolve = host.resolve;
        }
    }

    /** Point a space's main database at the space's own storage. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        return { reference: this.#reference(schema.identifier("space").parse(record.scope)) };
    }

    /** Remove the space's storage. */
    destroy(record: ResourceRecord<typeof DatabaseKind>): Promise<void> {
        return this.#host.destroy(record);
    }

    /** Diff the storage's applied state against the desired states. */
    plan(...plan: Parameters<DatabaseHost["plan"]>): ReturnType<DatabaseHost["plan"]> {
        return this.#host.plan(...plan);
    }

    /** Apply the reviewed plan to the storage. */
    apply(...apply: Parameters<DatabaseHost["apply"]>): ReturnType<DatabaseHost["apply"]> {
        return this.#host.apply(...apply);
    }

    /** Open the storage's desired tables. */
    open(...open: Parameters<DatabaseHost["open"]>): ReturnType<DatabaseHost["open"]> {
        return this.#host.open(...open);
    }

    /** Copy the storage's rows into a content store. */
    snapshot(
        ...replica: Parameters<DatabaseHost["snapshot"]>
    ): ReturnType<DatabaseHost["snapshot"]> {
        return this.#host.snapshot(...replica);
    }

    /** Restore a copy's rows into the storage. */
    restore(...replica: Parameters<DatabaseHost["restore"]>): ReturnType<DatabaseHost["restore"]> {
        return this.#host.restore(...replica);
    }
}
