import {
    type DatabaseConnection,
    type Row,
    type Table,
    inArray,
    Snapshot,
    CHAIN_TERMS,
    TABLE,
} from "@destack/db";
import { found, schema } from "@destack/schema";
import type { Directory } from "@destack/directory";
import type { Controller } from "@destack/service/control";
import { ObjectType } from "../object/object.ts";

/** Finishes the expired reservations of a database's indexed objects with their current names. */
export class ClaimController implements Controller {
    /** The controller's name in reports. */
    readonly name = "claims";
    /** The indexed objects' tables. */
    readonly watches: readonly Table[];
    /** The directory with the claims. */
    readonly #directory: Directory;
    /** The database with the objects. */
    readonly #database: DatabaseConnection;
    /** The indexed object types. */
    readonly #objects: readonly ObjectType[];

    /** Finish the reservations of some object types' indexes. */
    constructor(
        directory: Directory,
        database: DatabaseConnection,
        objects: readonly ObjectType[],
    ) {
        // keep the indexed types and watch their tables
        this.#directory = directory;
        this.#database = database;
        this.#objects = objects.filter((object) => Object.keys(object.indexes).length > 0);
        this.watches = this.#objects.map((object) => object.table);
    }

    /** List the key a watched change affects: the one key of the reservations. */
    keys(): readonly string[] {
        return ["reservations"];
    }

    /** List the one key to reconcile. */
    async list(): Promise<readonly string[]> {
        return ["reservations"];
    }

    /** Finish expired reservations, returning the delay until the next. */
    async reconcile(): Promise<number | undefined> {
        // list expired reservations
        const now = Date.now();
        const byIndex = new Map(
            this.#objects.flatMap((object) =>
                Object.keys(object.indexes).map((name) => [object.index(name), object] as const),
            ),
        );
        const expiry = await this.#directory.expired([...byIndex.keys()]);

        // finish one chain per run with enclosing scopes from one snapshot
        const due = expiry.claims.slice(0, CHAIN_TERMS);
        const snapshot = Snapshot.live(this.#database);
        const byObject = Map.groupBy(due, (entry) => found(byIndex, entry.index));
        for (const [object, entries] of byObject) {
            // read present rows
            const table = object.table;
            const ids = [...new Set(entries.map((entry) => entry.objectId))];
            const rows: readonly Row[] = await this.#database
                .select()
                .from(table)
                .where(inArray(table[TABLE].column("id"), ids));
            const present = new Map(rows.map((row) => [schema.string().parse(row["id"]), row]));

            // claim each object's names as its row has them now
            for (const id of ids) {
                const owned = await object.claimsOf(id, present.get(id), snapshot);
                ObjectType.refuse(
                    this.#objects,
                    await this.#directory.replace(owned, `finish-${id}`),
                );
            }
        }

        // schedule the next run
        if (expiry.claims.length > due.length) {
            return 0;
        }

        return expiry.next === undefined ? undefined : Math.max(0, expiry.next - now);
    }
}
