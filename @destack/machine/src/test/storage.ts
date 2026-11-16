import { join } from "node:path";
import { LocalBlobStorage, type LocalBucketHost } from "@destack/bucket/bun";
import { connectBunSqlite } from "@destack/db/bun";
import { TestDatabase } from "@destack/db/test";
import type { defineDatabase, Table } from "@destack/db";
import type { Environment, HostCapability } from "@destack/environment";
import type { EventKind } from "@destack/event";
import { ObjectType } from "@destack/object";
import { testEvents } from "@destack/object/test";
import type { Connector } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import { SettingValue } from "@destack/setting/object";
import type { SpaceProvider, StoreProvider } from "@destack/stack/object";
import { BunGraphStorage } from "../computer/graph.ts";
import { SocketRegistry } from "../space/socket.ts";
import type { SpaceStorage, Volume } from "../space/storage.ts";

/** A test machine's space volumes: one test database per space. */
export class TestSpaceStorage implements SpaceStorage {
    /** The main database every space stores, with the world's packages' tables. */
    readonly main: ReturnType<typeof defineDatabase>;
    /** The tables every space replicates of packages a space can install. */
    readonly replicated: readonly Table[];
    /** The event kinds every space records. */
    readonly kinds: readonly EventKind[] = [];
    /** The ephemeral object types a space stores in memory with its sockets. */
    readonly ephemeral: readonly ObjectType[] = [];
    /** The database of each space the machine stores. */
    readonly #databases = new Map<Identifier<"space">, TestDatabase>();
    /** The options the volumes open with. */
    readonly #options: TestSpaceStorageOptions;

    /** Store volumes below a machine's files directory, the world's spaces' databases given. */
    constructor(options: TestSpaceStorageOptions) {
        // keep what the volumes open with
        this.main = options.main;
        this.replicated = options.replicated;
        this.#options = options;
        for (const [space, database] of options.databases) {
            this.#databases.set(space, database);
        }
    }

    /** The machine's buckets, which every volume opens. */
    get buckets(): LocalBucketHost {
        return this.#options.files.buckets;
    }

    /** The machine's files directory, below which the volumes' blobs live. */
    get directory(): string {
        return this.#options.files.directory;
    }

    /** Read the database of a space the machine stores, refusing one it stores not. */
    database(space: Identifier<"space">): TestDatabase {
        const database = this.#databases.get(space);
        if (database === undefined) {
            throw new TypeError(`the test machine stores no volume of '${space}'`);
        }

        return database;
    }

    /** List the spaces whose volumes the machine stores. */
    async list(): Promise<readonly Identifier<"space">[]> {
        return [...this.#databases.keys()];
    }

    /** Open a space's volume over its database, a new one for a space it stores not yet. */
    async open(space: Identifier<"space">): Promise<Volume> {
        // take the space's database, creating it migrated for a new space
        const { environment, files } = this.#options;
        const test = this.#databases.get(space) ?? (await this.#options.create(space));
        this.#databases.set(space, test);
        const { database } = test;
        await database.declareApplied(this.replicated);

        // open its events, sockets, blobs and graphs, closed with the volume, each served type registering its events
        const objects = this.#options.objects;
        const events = await testEvents(
            database,
            environment,
            [],
            SettingValue.retention(database),
        );
        const failures: unknown[] = [];
        const sockets = await SocketRegistry.open({
            objects,
            environment,
            tracker: database.tracker,
            memory: (tables) => connectBunSqlite(":memory:", tables, { environment }),
            report: (error) => failures.push(error),
        });
        const blobs = await LocalBlobStorage.open(
            join(files.directory, "blobs", space),
            environment,
            {
                synchronous: "off",
            },
        );
        const opened: { graphs?: BunGraphStorage } = {};

        return {
            database,
            events,
            blobs,
            sockets,
            providers: this.#options.providers,
            connectors: this.#options.connectors,
            buckets: () => files.buckets,
            graphs: (builds) => {
                opened.graphs ??= new BunGraphStorage(database, builds, environment.files);

                return opened.graphs;
            },
            measure: async () => [],
            // close the sockets and graphs, failing on what the sockets reported
            close: async () => {
                await opened.graphs?.[Symbol.asyncDispose]();
                await sockets[Symbol.asyncDispose]();
                if (failures.length > 0) {
                    throw new AggregateError(failures, "the space's sockets reported failures");
                }
            },
        };
    }

    /** Delete a closed space's volume: its databases and its blobs. */
    async delete(space: Identifier<"space">): Promise<void> {
        // close and forget the database
        const test = this.#databases.get(space);
        this.#databases.delete(space);
        await test?.close();

        // remove the installations' databases and the blobs
        const { files } = this.#options.environment;
        const { directory } = this.#options.files;
        for (const path of [join(directory, "databases", space), join(directory, "blobs", space)]) {
            await files.remove(path, { recursive: true, force: true });
        }
    }

    /** List the object types of packages the spaces serve in addition to the stack's. */
    static objects(
        packages: readonly { readonly objects: Readonly<Record<string, unknown>> }[],
    ): ObjectType[] {
        return packages
            .flatMap((handle): unknown[] => Object.values(handle.objects))
            .filter((type): type is ObjectType => type instanceof ObjectType);
    }
}

/** What a test machine's volumes open with. */
export interface TestSpaceStorageOptions {
    /** The main database every space stores. */
    readonly main: ReturnType<typeof defineDatabase>;
    /** The tables every space replicates of packages a space can install. */
    readonly replicated: readonly Table[];
    /** The databases of the spaces the machine stores already, by space. */
    readonly databases: ReadonlyMap<Identifier<"space">, TestDatabase>;
    /** Create the database of a space the machine stores not yet, migrated with the main tables. */
    readonly create: (space: Identifier<"space">) => Promise<TestDatabase>;
    /** The object types of the world's packages, whose events and sockets the volumes store. */
    readonly objects: readonly ObjectType[];
    /** The machine's world. */
    readonly environment: Environment<HostCapability>;
    /** The machine's files: its directory and its buckets. */
    readonly files: { readonly directory: string; readonly buckets: LocalBucketHost };
    /** The providers every volume offers. */
    readonly providers: readonly (SpaceProvider | StoreProvider)[];
    /** The connectors every volume offers. */
    readonly connectors: readonly Connector[];
}
