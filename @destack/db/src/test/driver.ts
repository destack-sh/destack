import type { Clock } from "@destack/environment";
import type { Simulation } from "@destack/environment/simulation";
import type { SetStep } from "@destack/package/declare";
import { MemoryBlobStorage } from "@destack/resource";
import { type Digest, type JsonValue, present } from "@destack/schema";
import type { Counters, Driver } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import {
    type DatabaseObservation,
    type DatabaseStep,
    databaseInteraction,
} from "../benchmark/interaction.ts";
import { BLOB_TABLES, BlobStore } from "../blob/store.ts";
import { Transaction } from "../log/commit.ts";
import { eq } from "../sql/operator.ts";
import { text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { TestDatabase } from "./database.ts";

/** The scope every note of a database world lives in. */
const SCOPE = "bench";

/** The values a byte takes. */
const BYTE_VALUES = 256;

/** The notes a database world writes and reads. */
const note = defineTable(
    "bench_note",
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

/** A driver that plays database benchmarks over one database and its blobs in a simulated world. */
export class TestDatabaseDriver implements Driver<typeof databaseInteraction> {
    /** The interaction the driver speaks. */
    static readonly interaction = databaseInteraction;
    /** The settings of the worlds: test databases commit without flushing to disk. */
    static readonly configuration = { durability: "off" };

    /** The world the database runs in. */
    readonly #simulation: Simulation;
    /** The database. */
    readonly #storage: TestDatabase;
    /** The database's blobs, over memory storage. */
    readonly #blobs: BlobStore;
    /** The notes inserted so far, numbering each one's key. */
    #notes = 0;
    /** The blobs stored so far, in order. */
    readonly #stored: Digest[] = [];
    /** The reads of notes and blobs so far, each reading the next one, past the read cache. */
    #reads = 0;

    /** Take a started world, its database and its blobs. */
    constructor(simulation: Simulation, storage: TestDatabase, blobs: BlobStore) {
        this.#simulation = simulation;
        this.#storage = storage;
        this.#blobs = blobs;
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#simulation.clock;
    }

    /** Start a world with a database of notes and blobs on the test's dialect. */
    static async start(): Promise<TestDatabaseDriver> {
        // open the database in a world, and its blob store over memory
        const simulation = testSimulation();
        const environment = simulation.environment("database");
        const storage = await TestDatabase.create([note, ...BLOB_TABLES], {
            environment,
            isMigrated: true,
        });
        const memory = new MemoryBlobStorage(environment.crypto);
        const blobs = new BlobStore({
            storage: memory,
            database: storage.database,
            environment,
        });

        return new TestDatabaseDriver(simulation, storage, blobs);
    }

    /** Close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#storage.close();
    }

    /** Take one step, then run the world until the work it started settles. */
    async act(step: SetStep | DatabaseStep): Promise<void> {
        // insert notes in one transaction
        const { database } = this.#storage;
        if (step.action === "insert") {
            const rows = Array.from({ length: step.rows }, () => this.#next());
            await database.transaction((transaction) =>
                transaction.insert(note).values(rows).execute(),
            );
        }
        // read the next note in turn, which no earlier read cached
        else if (step.action === "read") {
            this.#reads += 1;
            const key = `note-${(this.#reads % this.#notes) + 1}`;
            await database.select().from(note).where(eq(note.id, key));
        }
        // update the note inserted last
        else if (step.action === "update") {
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
            this.#reads += 1;
            const blob = this.#stored[this.#reads % this.#stored.length];
            await this.#blobs.bytes(present(blob, "a blob stored before the fetch"));
        }
        // refuse the set step, as a database world has no example properties
        else {
            throw new TypeError("a database world takes no example properties");
        }

        // settle the work the step started
        await this.#simulation.run(this.clock.now());
    }

    /** Count the notes. */
    async observe(observation: DatabaseObservation): Promise<JsonValue> {
        // count the notes the observation asks for
        const rows = await this.#storage.database.select().from(note);

        return observation.kind === "notes" ? rows.length : null;
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#simulation.fingerprint;
    }

    /** Read the world's counters from its telemetry. */
    count(): Promise<Counters> {
        return this.#simulation.telemetry.counts();
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
