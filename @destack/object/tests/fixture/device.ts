import { bearerUser } from "./user.ts";
import { defineServer, ObjectStore } from "../../src/server/index.ts";
import { onTestFinished } from "@destack/test";
import { isNull, type Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../../src/client/index.ts";
import { Package } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { schema, nth, type Identifier } from "@destack/schema";
import { createClient, type ClientOptions } from "@destack/runtime/client";
import { Health } from "@destack/runtime/health";
import { Server, type PackageImplementation } from "@destack/runtime/server";
import { v7 } from "uuid";
import { note, notebook, notesDatabase, notesPackage } from "./note.ts";
import { principal } from "@destack/access";
import type { Subject } from "@destack/sync";
import { openSpace, spacePrincipal, unmoved } from "./space.ts";
import { testCallKey } from "@destack/runtime/test";
import { testEvents } from "../../src/test/index.ts";
import { Clock } from "@destack/environment";

/** The space with the notes. */
export const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The object types a device replicates, by key. */
const DEVICE_OBJECTS = { notebook, note };

/** Serve a space's notes to bearer-identified users, returning a client per user. */
export async function serveNotes(
    dialect: Dialect,
    clock: Clock = Clock.system,
    groups: ReadonlyMap<string, readonly Subject[]> = new Map(),
) {
    // store the space's notes in memory
    const storage = await TestDatabase.create(dialect, notesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // serve the notes over HTTP to bearer-identified users
    const served = serveObjects(
        nth(
            new ObjectStore({
                servers: [defineServer(notesPackage, {})],
                chain: async () => undefined,
                origin: { package: Package.of(notesPackage) },
                principal: spacePrincipal(spaceId),
                callKey: testCallKey,
                events: await testEvents(database),
                database,
                clock,
            }).packages,
            0,
        ),
        spaceId,
        groups,
    );

    const connect = (user: Identifier<"user">) => createClient(notesPackage, served.endpoint(user));

    return { connect, endpoint: served.endpoint, server: served.server, database };
}

/** Serve objects over HTTP to bearer-identified users in a space, with each user's endpoint. */
export function serveObjects(
    implementation: PackageImplementation,
    scope: string,
    groups: ReadonlyMap<string, readonly Subject[]> = new Map(),
) {
    // authenticate each request as the user its bearer credential identifies
    const server = Server.start({
        clock: Clock.system,
        ...implementation,
        scope,
        resources: new ResourceContext(),
        health: new Health(implementation.package.name),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: bearerUser(notesPackage.id, scope, groups),
    });
    onTestFinished(() => server.close());

    // reach the server as a user
    const endpoint = (user: Identifier<"user">): ClientOptions => ({
        url: "https://objects.test",
        headers: { authorization: `Bearer ${user}` },
        fetch: (request: Request) => server.fetch(request),
    });

    return { server, endpoint };
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

    /** Hold a device's storage and objects. */
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
        const storage = await TestDatabase.create("sqlite", tables, { storage: "file" });
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
