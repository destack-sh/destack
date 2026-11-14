import type { Clock } from "@destack/environment";
import type { Simulation, SimulatedTelemetry } from "@destack/environment/simulation";
import type { Given, SetStep } from "@destack/package/declare";
import { MemoryBlobStorage } from "@destack/resource";
import { type Digest, type JsonValue, present } from "@destack/schema";
import type { Driver } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import {
    type DatabaseObservation,
    type DatabaseStep,
    databaseInteraction,
} from "../interaction.ts";
import { BLOB_TABLES, BlobStore } from "../blob/store.ts";
import { Transaction } from "../log/commit.ts";
import { eq } from "../sql/operator.ts";
import { text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { TestDatabase } from "./database.ts";

/** The scope every note of a database world lives in. */
const SCOPE = "notes";

/** The values a byte takes. */
const BYTE_VALUES = 256;

/** The notes a database world writes and reads. */
const note = defineTable(
    "note",
    {
        /** The note's key. */
        id: text("id").primaryKey(),
        /** The scope the note lives in. */
        scope: text("scope").notNull(),
        /** The note's text. */
        body: text("body").notNull(),
    },
    { log: {} },
);

/** A driver that plays database scenarios over one database and its blobs in a simulated world. */
export class DatabaseDriver implements Driver<DatabaseStep, DatabaseObservation> {
    /** The interaction the driver speaks. */
    static readonly interaction = databaseInteraction;
    /** The settings of the worlds: test databases commit without flushing to disk. */
    static readonly configuration = { durability: "off" };

    /** The world the database runs in. */
    readonly #simulation: Simulation;
    /** The database. */
    readonly #database: TestDatabase;
    /** The database's blobs, over memory storage. */
    readonly #blobs: BlobStore;
    /** The notes inserted so far, numbering each one's key. */
    #notes = 0;
    /** The blobs stored so far, in order. */
    readonly #stored: Digest[] = [];
    /** The reads of notes so far, each reading the next one, past the read cache. */
    #noteReads = 0;
    /** The reads of blobs so far, each reading the next one, past the read cache. */
    #blobReads = 0;

    /** Take a started world, its database and its blobs. */
    constructor(simulation: Simulation, database: TestDatabase, blobs: BlobStore) {
        this.#simulation = simulation;
        this.#database = database;
        this.#blobs = blobs;
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#simulation.clock;
    }

    /** Start a world with a database of notes and blobs on the test's dialect. */
    static async start(_given: Given): Promise<DatabaseDriver> {
        // open the database in a world, and its blob store over memory
        const simulation = testSimulation();
        const environment = simulation.environment("database");
        const database = await TestDatabase.create([note, ...BLOB_TABLES], {
            environment,
            isMigrated: true,
        });
        const memory = new MemoryBlobStorage(environment.crypto);
        const blobs = new BlobStore({
            storage: memory,
            database: database.database,
            environment,
        });

        return new DatabaseDriver(simulation, database, blobs);
    }

    /** Close the database, then the world. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#database.close();
        await this.#simulation[Symbol.asyncDispose]();
    }

    /** Take one step, then run the world until the work it started settles. */
    async act(step: SetStep | DatabaseStep): Promise<void> {
        // insert notes in one transaction
        const { database } = this.#database;
        if (step.action === "insert") {
            const rows = Array.from({ length: step.rows }, () => this.#next());
            await database.transaction((transaction) =>
                transaction.insert(note).values(rows).execute(),
            );
        }
        // read the next note in turn, which no earlier read cached
        else if (step.action === "read") {
            this.#requireNotes("read");
            this.#noteReads += 1;
            const key = `note-${(this.#noteReads % this.#notes) + 1}`;
            await database.select().from(note).where(eq(note.id, key));
        }
        // update the note inserted last
        else if (step.action === "update") {
            this.#requireNotes("update");
            const body = `note ${this.#notes} updated`;
            const key = this.#latest();
            await database.transaction((transaction) =>
                transaction.update(note).set({ body }).where(eq(note.id, key)).execute(),
            );
        }
        // insert a note through the log, as an object call commits
        else if (step.action === "transact") {
            const row = this.#next();
            await Transaction.run(database, (transaction) =>
                transaction.write({ table: note, before: null, after: row }),
            );
        }
        // store a blob of the given bytes, filled by its number
        else if (step.action === "write") {
            const bytes = new Uint8Array(step.bytes).fill(this.#stored.length % BYTE_VALUES);
            this.#stored.push(await this.#blobs.write(chunked(bytes)));
        }
        // read the next stored blob in turn, which no earlier read cached
        else if (step.action === "fetch") {
            this.#blobReads += 1;
            const blob = this.#stored[this.#blobReads % this.#stored.length];
            await this.#blobs.bytes(present(blob, "a blob stored before the fetch"));
        }
        // refuse the set step, as a database world has no example properties
        else {
            throw new TypeError("a database world takes no example properties");
        }

        // settle the work the step started
        await this.#simulation.run(this.clock.now());
    }

    /** Count the notes or the stored blobs. */
    async observe(observation: DatabaseObservation): Promise<JsonValue> {
        // count the rows of the notes' or the blobs' table
        const { database } = this.#database;
        const rows = await database
            .select()
            .from(observation.count === "notes" ? note : BlobStore.table);

        return rows.length;
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#simulation.fingerprint;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#simulation.telemetry;
    }

    /** Refuse a step on notes before any note was inserted. */
    #requireNotes(action: string): void {
        if (this.#notes === 0) {
            throw new TypeError(`a database world cannot '${action}' a note before inserting one`);
        }
    }

    /** Number the next note. */
    #next(): { readonly id: string; readonly scope: string; readonly body: string } {
        this.#notes += 1;

        return { id: `note-${this.#notes}`, scope: SCOPE, body: `note ${this.#notes}` };
    }

    /** Read the key of the note inserted last. */
    #latest(): string {
        return `note-${this.#notes}`;
    }
}

/** Stream bytes as one chunk. */
async function* chunked(bytes: Uint8Array): AsyncIterable<Uint8Array> {
    yield bytes;
}
