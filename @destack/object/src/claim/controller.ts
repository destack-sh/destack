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
import type { Controller } from "@destack/runtime/control";
import { ObjectKey, ObjectType } from "../object/object.ts";
import { type Claimer, type ClaimHome, Reservation } from "./reservation.ts";
import { Claimant, Claim } from "./claim.ts";

/** The one key the controller reconciles: every expired reservation. */
const RESERVATIONS = "reservations";

/** Finishes the expired reservations of a database's indexed objects with their current names. */
export class ClaimController implements Controller {
    /** The controller's name in reports. */
    readonly name = "claims";
    /** The indexed objects' tables. */
    readonly watches: readonly Table[];
    /** The claim type and the store serving it. */
    readonly #claims: ClaimHome;
    /** The database with the objects. */
    readonly #database: DatabaseConnection;
    /** The indexed object types. */
    readonly #objects: readonly ObjectType[];
    /** The store's clock, which reservations expire by. */
    readonly #clock: Clock;
    /** Read the principal claiming a type's names. */
    readonly #claimer: (object: ObjectType) => Claimer;

    /** Finish the reservations of some object types' indexes, each as its claimer. */
    constructor(
        claims: ClaimHome,
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        options: {
            /** The store's clock. */
            readonly clock: Clock;
            /** Read the principal claiming a type's names. */
            readonly claimer: (object: ObjectType) => Claimer;
        },
    ) {
        // keep the indexed types and watch their tables
        this.#claims = claims;
        this.#database = database;
        this.#clock = options.clock;
        this.#claimer = options.claimer;
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
        // list a chain's worth of expired reservations
        const now = this.#clock.now();
        const byIndex = new Map(
            this.#objects.flatMap((object) =>
                Object.keys(object.indexes).map(
                    (name) => [Claim.index(object, name), object] as const,
                ),
            ),
        );
        const expiry = await Reservation.expired(
            this.#claims,
            this.#database,
            [...byIndex.keys()],
            now,
        );

        // claim each object's names as its row has them now, with enclosing scopes from one snapshot
        const snapshot = Snapshot.live(this.#database);
        const byObject = Map.groupBy(expiry.claims, (entry) => found(byIndex, entry.index));
        for (const [object, entries] of byObject) {
            // read present rows
            const table = object.table;
            const owners = new Map(entries.map((entry) => [entry.objectId, entry.owner]));
            const rows: readonly Row[] = await this.#database
                .select()
                .from(table)
                .where(inArray(table[TABLE].column("id"), [...owners.keys()]));
            const present = new Map(rows.map((row) => [ObjectKey.parse(row).id, row]));

            // read each object's names, under its reservation's owner once its row is gone
            const claimants: Claimant[] = [];
            for (const [objectId, owner] of owners) {
                const row = present.get(objectId);
                const current = row === undefined ? owner : Claim.owning(object, row);
                claimants.push(
                    await Claimant.of(object, { objectId, owner: current }, row, snapshot),
                );
            }

            // replace their names as the type's claimer, refusing names other objects own
            await Reservation.replace(this.#claims, this.#database, claimants, this.#objects, {
                now,
                claimer: this.#claimer(object),
            });
        }

        // run again at once while a chain's worth expired, else at the next deadline
        if (expiry.claims.length >= CHAIN_TERMS) {
            return 0;
        }

        return expiry.next === undefined ? undefined : Math.max(0, expiry.next - now);
    }
}
