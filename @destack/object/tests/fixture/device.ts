import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { ObjectServer } from "../../src/server/index.ts";
import { onTestFinished } from "@destack/test";
import { AuditRecorder } from "@destack/audit";
import { isNull, type DatabaseConnection, type Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../../src/client/index.ts";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { v7 } from "uuid";
import { note, notebook, notesDatabase, notesService, notesJournal } from "./notes.ts";
import { principal } from "@destack/access";
import { openSpace, unmoved } from "./space.ts";

/** The space holding the notes. */
export const spaceId = identifier("space").parse(`space-${v7()}`);

/** The package serving the notes. */
const audience = PackageId.parse("package-01a0d5eb-fb8a-74f4-ba37-8a4d6970e238");

/** Serve a space's notes to bearer-named users, returning a client per user. */
export async function serveNotes(dialect: Dialect) {
    // hold the space's notes in memory
    const storage = await TestDatabase.create(dialect, notesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // authenticate each request as the user its bearer credential names
    const server = Server.start({
        ...ObjectServer.serve(notesService, {
            journal: notesJournal,
            database,
            audit: AuditRecorder.service(new AuditOutbox(database), {
                package: notesService.package,
                service: "test",
            }),
        }),
        audience,
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("notes"),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            const id = request.headers.get("authorization")!.slice("Bearer ".length);
            const subject = principal.user.reference("global", id);
            const now = Date.now();

            return new Caller({
                subject,
                subjects: [subject],
                credential: { kind: "user", id },
                audience,
                scope: spaceId,
                verifiedAt: now,
                expiresAt: now + 60_000,
            });
        },
    });
    onTestFinished(() => server.close());

    const connect = (user: string) =>
        createClient(notesService.router, {
            url: "https://notes.test",
            headers: { authorization: `Bearer ${user}` },
            fetch: (request: Request) => server.fetch(request),
        });

    return { connect, database };
}

/** A user's device holding notebooks and notes in a local database file. */
export class Device {
    /** The local database file. */
    readonly storage: TestDatabase;
    /** The objects the device holds. */
    readonly client: ObjectClient;
    /** The failures pushing and following reported. */
    readonly errors: unknown[] = [];
    /** Stop pushing and following. */
    #controller = new AbortController();
    /** The running push and follow loops. */
    #loops: Promise<void>[] = [];

    /** Hold a device's storage and objects. */
    private constructor(storage: TestDatabase, client: ObjectClient) {
        this.storage = storage;
        this.client = client;
    }

    /** Open a user's device on a new local database file, offline until it goes online. */
    static async open(
        user: string,
        service: ReturnType<Awaited<ReturnType<typeof serveNotes>>["connect"]>,
        queried: readonly (typeof notebook | typeof note)[] = [notebook, note],
        options: { readonly storage?: { readonly rows: number } } = {},
    ) {
        // create the local database and hold the space's notebooks and notes in it
        const tables = ObjectClient.tables([notebook, note]);
        const storage = await TestDatabase.create("sqlite", tables, { storage: "file" });
        const client = await ObjectClient.open({
            database: storage.database,
            objects: [notebook, note],
            scope: spaceId,
            caller: principal.user.reference("global", user),
            service: service.replica,
            reconnect: unmoved,
            ...options,
        });
        for (const object of queried) {
            client.subscribe(object);
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

    /** Stop pushing and following, keeping the queue and the replica. */
    async offline(): Promise<void> {
        this.#controller.abort();
        await Promise.all(this.#loops);
        this.#controller = new AbortController();
        this.#loops = [];
    }

    /** Read the titles of the notes outside the trash the device holds. */
    async titles(): Promise<string[]> {
        const rows = await this.client.database
            .select({ title: note.table.title })
            .from(note.table)
            .where(isNull(note.table.deletionRequestedAt));

        return rows.map((row) => row.title).sort();
    }

    /** Stop and remove the device. */
    async close(): Promise<void> {
        await this.offline();
        await this.storage.close();
    }
}
