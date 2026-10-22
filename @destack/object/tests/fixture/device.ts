import { onTestFinished } from "@destack/test";
import { isNull } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../../src/client/index.ts";
import { schema, type Identifier } from "@destack/schema";
import { type ClientOptions } from "@destack/object/client";
import { v7 } from "uuid";
import { note, notebook, notesPackage } from "./note.ts";
import { principal } from "@destack/access";
import { Scope, type Subject } from "@destack/sync";
import { space, unmoved } from "./space.ts";
import { TestObjectStore } from "../../src/test/index.ts";
import { Clock } from "@destack/environment";

/** The space with the notes. */
export const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The object types a device replicates, by key. */
const DEVICE_OBJECTS = { notebook, note };

/** Serve the space's notes to bearer-identified users, in the vouched groups. */
export async function serveNotes(
    clock: Clock = Clock.system,
    groups: ReadonlyMap<string, readonly Subject[]> = new Map(),
) {
    // store and serve the space's notes, public to anyone
    const objects = await TestObjectStore.open({
        packages: [notesPackage],
        scope: space.reference(Scope.universe.id, spaceId),
        clock,
        groups,
    });
    onTestFinished(() => objects[Symbol.asyncDispose]());

    return {
        connect: (user: Identifier<"user">) => objects.client(notesPackage, user),
        endpoint: (user: Identifier<"user">) => objects.endpoint(notesPackage, user),
        server: objects.server(notesPackage),
        database: objects.database,
    };
}

/** A user's device with notebooks and notes in a local database file. */
export class Device {
    /** The local database file. */
    readonly storage: TestDatabase;
    /** The objects the device replicates. */
    readonly client: ObjectClient<typeof DEVICE_OBJECTS>;
    /** The failures pushing and following reported. */
    readonly errors: unknown[] = [];
    /** Stop pushing and following. */
    #controller = new AbortController();
    /** The running push and follow loops. */
    #loops: Promise<void>[] = [];

    /** Store a device's storage and objects. */
    private constructor(storage: TestDatabase, client: ObjectClient<typeof DEVICE_OBJECTS>) {
        this.storage = storage;
        this.client = client;
    }

    /** Open a user's device on a new local database file, offline until it goes online. */
    static async open(
        user: Identifier<"user">,
        endpoint: ClientOptions,
        queried: readonly (typeof notebook | typeof note)[] = [notebook, note],
        options: {
            readonly storage?: { readonly rows: number };
            readonly push?: { readonly mutations: number };
            readonly clock?: Clock;
        } = {},
    ) {
        // create the local database and replicate the space's notebooks and notes in it
        const tables = ObjectClient.tables({ notebook, note });
        const storage = await TestDatabase.create(tables, { dialect: "sqlite", storage: "file" });
        const client = await ObjectClient.open({
            database: storage.database,
            objects: DEVICE_OBJECTS,
            scope: spaceId,
            caller: principal.user.reference("universe", user),
            endpoint,
            reconnect: unmoved,
            ...options,
            clock: options.clock ?? Clock.system,
        });
        for (const object of queried) {
            client.queryType(object).findMany().subscribe();
        }
        const device = new Device(storage, client);
        onTestFinished(() => device.close());

        return device;
    }

    /** Start pushing queued mutations and following the space. */
    online(): void {
        this.follow();
        this.push();
    }

    /** Start following the space, rebasing predictions onto what arrives. */
    follow(): void {
        this.#loops.push(
            this.client.follow(this.#controller.signal, (error) => this.errors.push(error)),
        );
    }

    /** Start pushing queued mutations. */
    push(): void {
        this.#loops.push(
            this.client.push(this.#controller.signal, (error) => this.errors.push(error)),
        );
    }

    /** Stop pushing and following and retain the queue and the replica. */
    async offline(): Promise<void> {
        this.#controller.abort();
        await Promise.all(this.#loops);
        this.#controller = new AbortController();
        this.#loops = [];
    }

    /** Read the titles of the notes outside the trash the device has. */
    async titles(): Promise<string[]> {
        const rows = await this.client.database
            .select({ title: note.table.title })
            .from(note.table)
            .where(isNull(note.table.deletedAt));

        return rows.map((row) => row.title).toSorted();
    }

    /** Stop and remove the device. */
    async close(): Promise<void> {
        await this.offline();
        await this.storage.close();
    }
}
