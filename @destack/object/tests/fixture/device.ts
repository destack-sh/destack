import { ObjectServer } from "../../src/server/index.ts";
import { onTestFinished } from "@destack/test";
import { isNull, type Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../../src/client/index.ts";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { schema, present } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { createClient, type ClientOptions } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { Server, type ServiceImplementation } from "@destack/service/server";
import { v7 } from "uuid";
import { note, notebook, notesDatabase, notesService } from "./note.ts";
import { principal } from "@destack/access";
import { openSpace, unmoved } from "./space.ts";
import { testCallKey } from "@destack/service/test";

/** The space with the notes. */
export const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The object types a device keeps, by key. */
const DEVICE_OBJECTS = { notebook, note };

/** The package serving the notes. */
const audience = PackageId.parse("package-01a0d5eb-fb8a-74f4-ba37-8a4d6970e238");

/** Serve a space's notes to bearer-named users, returning a client per user. */
export async function serveNotes(dialect: Dialect) {
    // keep the space's notes in memory
    const storage = await TestDatabase.create(dialect, notesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // serve the notes over HTTP to bearer-named users
    const served = serveObjects(
        ObjectServer.serve(notesService, {
            callKey: testCallKey,
            database,
        }),
        spaceId,
    );

    const connect = (user: string) => createClient(notesService, served.endpoint(user));

    return { connect, endpoint: served.endpoint, server: served.server, database };
}

/** Serve an object service over HTTP to bearer-named users in a space, with each user's endpoint. */
export function serveObjects(implementation: ServiceImplementation, scope: string) {
    // authenticate each request as the user its bearer credential names
    const server = Server.start({
        ...implementation,
        audience,
        scope,
        resources: new ResourceContext(),
        health: new Health(implementation.service.name),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: async (request) => {
            const id = present(
                request.headers.get("authorization"),
                "the authorization header",
            ).slice("Bearer ".length);
            const subject = principal.user.reference("universe", id);
            const now = Date.now();

            return new Authentication({
                subject,
                subjects: [subject],
                credential: { kind: "user", id },
                audience,
                scope,
                verifiedAt: now,
                expiresAt: now + 60_000,
            });
        },
    });
    onTestFinished(() => server.close());

    // reach the server as a user
    const endpoint = (user: string): ClientOptions => ({
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
    /** The objects the device keeps. */
    readonly client: ObjectClient<typeof DEVICE_OBJECTS>;
    /** The failures pushing and following reported. */
    readonly errors: unknown[] = [];
    /** Stop pushing and following. */
    #controller = new AbortController();
    /** The running push and follow loops. */
    #loops: Promise<void>[] = [];

    /** Keep a device's storage and objects. */
    private constructor(storage: TestDatabase, client: ObjectClient<typeof DEVICE_OBJECTS>) {
        this.storage = storage;
        this.client = client;
    }

    /** Open a user's device on a new local database file, offline until it goes online. */
    static async open(
        user: string,
        endpoint: ClientOptions,
        queried: readonly (typeof notebook | typeof note)[] = [notebook, note],
        options: {
            readonly storage?: { readonly rows: number };
            readonly push?: { readonly mutations: number };
        } = {},
    ) {
        // create the local database and keep the space's notebooks and notes in it
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
        });
        for (const object of queried) {
            client.queryOf(object).findMany().subscribe();
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

    /** Stop pushing and following and keep the queue and the replica. */
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
            .where(isNull(note.table.deletionRequestedAt));

        return rows.map((row) => row.title).toSorted();
    }

    /** Stop and remove the device. */
    async close(): Promise<void> {
        await this.offline();
        await this.storage.close();
    }
}
