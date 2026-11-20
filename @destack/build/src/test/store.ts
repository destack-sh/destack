import { join } from "node:path";
import { LocalBlobStorage } from "@destack/bucket/bun";
import { BlobStore } from "@destack/db";
import { type BlobStorage, MemoryBlobStorage } from "@destack/resource";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { Environment } from "@destack/environment";
import { testHost } from "@destack/test/host";
import { LocalBuildStore } from "../store/local.ts";
import { BuildStore } from "../store/store.ts";
import { fixtureGraph } from "./graph.ts";
import { BuildImage, BuildLayer, type BuildReader } from "@destack/package/manifest";
import type { Digest } from "@destack/schema";

/** Open the store of builds tests share across runs, the host's, or a world's view of it. */
export async function testStore(
    world?: Environment<"files" | "process" | "host">,
): Promise<LocalBuildStore> {
    // open and maintain the shared store on the host, as one operation in a world
    if (world === undefined) {
        return await LocalBuildStore.user("test", testHost());
    }

    // give the world a view of it through its own effects
    const store = await world.scheduler.run(() => LocalBuildStore.user("test", testHost()));

    return store.view(world, world.scheduler);
}

/** A build store over a test's blobs and index, in memory or below a directory. */
export class TestBuildStore extends BuildStore implements AsyncDisposable {
    /** The database with the index. */
    readonly #database: SqliteDatabase;

    /** Store builds in blobs indexed in a database the store closes, over a layer. */
    private constructor(
        storage: BlobStorage,
        database: SqliteDatabase,
        environment: Environment,
        layer: BuildLayer = BuildLayer.empty(),
    ) {
        super({ blobs: new BlobStore({ storage, database, environment }), environment, layer });
        this.#database = database;
    }

    /** Open an empty store in memory. */
    static async create(environment: Environment): Promise<TestBuildStore> {
        return new TestBuildStore(
            new MemoryBlobStorage(environment.crypto),
            await TestBuildStore.#migrated(":memory:", environment),
            environment,
        );
    }

    /** Open the store below a directory, created once, as a template copies it, over a layer. */
    static async open(
        directory: string,
        environment: Environment<"files">,
        layer: BuildLayer = BuildLayer.empty(),
    ): Promise<TestBuildStore> {
        const storage = await LocalBlobStorage.open(join(directory, "files"), environment, {
            synchronous: "off",
        });
        const database = await TestBuildStore.#migrated(join(directory, "store.db"), environment);

        return new TestBuildStore(storage, database, environment, layer);
    }

    /** Write a layer shipping each build as an image of its own, as a distribution does. */
    static async layer(
        builds: readonly BuildReader[],
        directory: string,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<{ readonly layer: BuildLayer; readonly images: readonly Digest[] }> {
        // put each build and its image in a store beside the layer, which links into it
        await using distributor = await LocalBuildStore.open(join(directory, "store"), environment);
        const images: Digest[] = [];
        for (const build of builds) {
            images.push(await distributor.putImage(BuildImage.of(await distributor.put(build))));
        }
        await distributor.writeLayer(images, ["bun"], join(directory, "layer"), "linked");

        return { layer: await BuildLayer.open(join(directory, "layer"), environment), images };
    }

    /** Put a build with its fixture graph attached, as a builder stores what it ships. */
    async putIndexed(reader: BuildReader): Promise<Digest> {
        const digest = await this.put(reader);
        await this.attachGraph(digest, await fixtureGraph(reader));

        return digest;
    }

    /** Close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#database.close();
    }

    /** Connect to an index database, migrated to the store's tables. */
    static async #migrated(file: string, environment: Environment): Promise<SqliteDatabase> {
        const database = await connectBunSqlite(file, BuildStore.database, { environment });
        await database.migrate(BuildStore.database.tables);

        return database;
    }
}
