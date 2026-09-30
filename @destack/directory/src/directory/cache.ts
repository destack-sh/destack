import type { Table } from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import { ReadCache, type Change } from "@destack/db/log";
import {
    claimTable,
    type Claim,
    type ClaimOwner,
    type Expiry,
    type ObjectClaims,
} from "../claim/claim.ts";
import { cellTable, zoneTable, type Cell, type Zone } from "../zone/zone.ts";
import type { DirectoryDatabase } from "./database.ts";
import { Directory } from "./directory.ts";

/** The most entries of each kind a cache keeps: about 3 MiB at 200 bytes an entry. */
const CAPACITY = 16_384;

/** The global tables of the cached reads, with logged changes that invalidate them. */
const CACHED_TABLES: readonly Table[] = [claimTable, zoneTable, cellTable];

/** The directory in the global database that keeps its reads until their rows change. */
export class DirectoryCache extends Directory {
    /** The directory the reads fall through to. */
    readonly #directory: DirectoryDatabase;
    /** Claim owners by index and key. */
    readonly #owners = new ReadCache<ClaimOwner | undefined>(CAPACITY);
    /** Zones by scope. */
    readonly #zones = new ReadCache<Zone | undefined>(CAPACITY);
    /** Cells by identifier. */
    readonly #cells = new ReadCache<Cell | undefined>(CAPACITY);

    /** Keep the reads of a directory in the global database. */
    constructor(directory: DirectoryDatabase) {
        super();
        this.#directory = directory;
    }

    /** Place a zone in its cell at an epoch. */
    place(zone: Zone): Promise<void> {
        return this.#directory.place(zone);
    }

    /** Withdraw a zone its cell serves at an epoch. */
    withdraw(zone: Zone): Promise<void> {
        return this.#directory.withdraw(zone);
    }

    /** Find the zone of a scope's databases. */
    locate(scope: string): Promise<Zone | undefined> {
        return this.#zones.get(scope, () => this.#directory.locate(scope));
    }

    /** List the zones a scope contains. */
    list(scope: string): Promise<readonly Zone[]> {
        return this.#directory.list(scope);
    }

    /** Mark a zone as moving to a target cell. */
    move(zone: Zone, target: string): Promise<void> {
        return this.#directory.move(zone, target);
    }

    /** Record the URL a cell answers at. */
    publish(cell: string, scope: string, endpoint: string): Promise<void> {
        return this.#directory.publish(cell, scope, endpoint);
    }

    /** Read a cell, absent before it published an endpoint. */
    cell(id: string): Promise<Cell | undefined> {
        return this.#cells.get(id, () => this.#directory.cell(id));
    }

    /** Reserve a request's claims. */
    claim(claims: readonly Claim[], requestId: string, now: number): Promise<void> {
        return this.#directory.claim(claims, requestId, now);
    }

    /** Confirm a request's reserved claims. */
    confirm(requestId: string, owned: readonly ObjectClaims[]): Promise<void> {
        return this.#directory.confirm(requestId, owned);
    }

    /** Release the reservations of a request with a failed write. */
    release(requestId: string): Promise<void> {
        return this.#directory.release(requestId);
    }

    /** Replace an object's claims after a write that reserved none. */
    replace(owned: ObjectClaims, requestId: string, now: number): Promise<void> {
        return this.#directory.replace(owned, requestId, now);
    }

    /** Find the object owning a confirmed name. */
    owner(index: string, key: string): Promise<ClaimOwner | undefined> {
        return this.#owners.get(`${index}\n${key}`, () => this.#directory.owner(index, key));
    }

    /** List the expired reservations of some indexes. */
    expired(indexes: readonly string[], now: number): Promise<Expiry> {
        return this.#directory.expired(indexes, now);
    }

    /** Forget the reads each logged change affects, following the log until the signal aborts. */
    async follow(signal: AbortSignal): Promise<void> {
        const log = this.#directory.database.log;
        while (!signal.aborted) {
            // forget every read, and follow the changes committed since
            const after = (await log.position()).sequence;
            this.#clear();
            try {
                for await (const page of log.follow({ tables: CACHED_TABLES, after }, signal)) {
                    for (const change of page.changes) {
                        this.#forget(change);
                    }
                }
            } catch (error) {
                // start over once the changes were compacted away
                if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                    throw error;
                }
            }
        }
    }

    /** Forget every read. */
    #clear(): void {
        for (const cache of [this.#owners, this.#zones, this.#cells]) {
            cache.clear();
        }
    }

    /** Forget the reads one change affects, before and after it. */
    #forget(change: Change): void {
        const images = [change.before, change.after] as (
            | Readonly<Record<string, unknown>>
            | undefined
        )[];
        for (const image of images.filter((row) => row !== undefined)) {
            // forget the read the row's table keeps it under
            if (change.table === claimTable) {
                this.#owners.forget(`${String(image.index)}\n${String(image.key)}`);
            } else if (change.table === zoneTable) {
                this.#zones.forget(String(image.id));
            } else {
                this.#cells.forget(String(image.id));
            }
        }
    }
}
