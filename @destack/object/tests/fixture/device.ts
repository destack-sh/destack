import { onTestFinished } from "@destack/test";
import { isNull } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../../src/client/index.ts";
import { schema, type Identifier } from "@destack/schema";
import { type ClientOptions } from "@destack/object/client";
import { note, notebook, notesPackage } from "./note.ts";
import { principal } from "@destack/access";
import { Scope, type Subject } from "@destack/sync";
import { space, unmoved } from "./space.ts";
import { TestObjectStore } from "../../src/test/index.ts";
import type { Environment } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";

/** The space with the notes. */
export const spaceId = schema
    .identifier("space")
    .parse(`space-01996ab0-0000-7000-8000-00000000a00f`);

/** The object types a device replicates, by key. */
const DEVICE_OBJECTS = { notebook, note };

/** How a device stores, pushes and runs: its storage and push budgets and its world. */
interface DeviceOptions {
    /** The most rows its storage keeps. */
    readonly storage?: { readonly rows: number };
    /** The most mutations one push sends. */
    readonly push?: { readonly mutations: number };
    /** The world it runs in. */
    readonly environment: Environment;
}

/** Serve the space's notes to bearer-identified users, in the vouched groups and a world. */
export async function serveNotes(
    options: {
        /** The simulated world the store runs in, its own by default. */
        readonly simulation?: Simulation;
        /** The node the store runs as in its world. */
        readonly node?: string;
        /** The groups each user's credential vouches for, by user. */
        readonly groups?: ReadonlyMap<string, readonly Subject[]>;
    } = {},
) {
    // store and serve the space's notes, public to anyone
    const objects = await TestObjectStore.open({
        packages: [notesPackage],
        scope: space.reference(Scope.universe.id, spaceId),
        ...options,
    });
    onTestFinished(() => objects[Symbol.asyncDispose]());

    // open devices in the store's world, reaching it as their users
    const endpoint = (user: Identifier<"user">) => objects.as(user).endpoint(notesPackage);
    const openDevice = (
        user: Identifier<"user">,
        queried?: readonly (typeof notebook | typeof note)[],
        device: Omit<DeviceOptions, "environment"> = {},
    ) =>
        Device.open(user, endpoint(user), queried, { ...device, environment: objects.environment });

    return {
        connect: (user: Identifier<"user">) => objects.as(user).client(notesPackage),
        endpoint,
        openDevice,
        server: objects.server(notesPackage),
        database: objects.database,
        environment: objects.environment,
        clock: objects.clock,
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
        options: DeviceOptions,
    ) {
        // create the local database in the device's world and replicate the space's notebooks and notes in it
        const { environment, ...rest } = options;
        const tables = ObjectClient.tables({ notebook, note });
        const storage = await TestDatabase.create(tables, {
            environment: environment,
            dialect: "sqlite",
            storage: "file",
        });
        const client = await ObjectClient.open({
            database: storage.database,
            objects: DEVICE_OBJECTS,
            scope: spaceId,
            caller: principal.user.reference("universe", user),
            endpoint,
            reconnect: unmoved,
            ...rest,
            environment,
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
