import {
    type DatabaseConnection,
    type Row,
    type Table,
    inArray,
    Snapshot,
    CHAIN_TERMS,
    TABLE,
} from "@destack/db";
import { Clock, found } from "@destack/schema";
import type { Controller } from "@destack/service/control";
import { ObjectKey, ObjectType } from "../object/object.ts";
import { type ClaimStore, Reservation } from "./store.ts";
import { Claimant, Claim } from "./claim.ts";

/** The one key the controller reconciles: every expired reservation. */
const RESERVATIONS = "reservations";

/** Finishes the expired reservations of a database's indexed objects with their current names. */
export class ClaimController implements Controller {
    /** The controller's name in reports. */
    readonly name = "claims";
    /** The indexed objects' tables. */
    readonly watches: readonly Table[];
    /** The store with the claims. */
    readonly #claims: ClaimStore;
    /** The database with the objects. */
    readonly #database: DatabaseConnection;
    /** The indexed object types. */
    readonly #objects: readonly ObjectType[];
    /** The store's clock, which reservations expire by. */
    readonly #clock: Clock;

    /** Finish the reservations of some object types' indexes on their store's clock. */
    constructor(
        claims: ClaimStore,
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        clock: Clock,
    ) {
        // keep the indexed types and watch their tables
        this.#claims = claims;
        this.#database = database;
        this.#clock = clock;
        this.#objects = objects.filter((object) => Object.keys(object.indexes).length > 0);
        this.watches = this.#objects.map((object) => object.table);
    }

    /** List the key a watched change affects: the one key of the reservations. */
    keys(): readonly string[] {
        return [RESERVATIONS];
    }

    /** List the one key to reconcile. */
    async list(): Promise<readonly string[]> {
        return [RESERVATIONS];
    }

    /** Finish expired reservations, returning the delay until the next. */
    async reconcile(): Promise<number | undefined> {
        // list expired reservations
        const now = this.#clock.now();
        const byIndex = new Map(
            this.#objects.flatMap((object) =>
                Object.keys(object.indexes).map((name) => [Claim.index(object, name), object] as const),
            ),
        );
        const expiry = await this.#claims.expired([...byIndex.keys()]);

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
            const present = new Map(rows.map((row) => [ObjectKey.parse(row).id, row]));

            // claim each object's names as its row has them now
            for (const id of ids) {
                const claimant = await Claimant.of(object, id, present.get(id), snapshot);
                Reservation.refuse(this.#objects, await this.#claims.replace(claimant));
            }
        }

        // schedule the next run
        if (expiry.claims.length > due.length) {
            return 0;
        }

        return expiry.next === undefined ? undefined : Math.max(0, expiry.next - now);
    }
}
