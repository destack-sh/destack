import { principal } from "@destack/access";
import { Directory } from "@destack/account/address";
import { MachineIdentity } from "@destack/account/client";
import { IdentityRegistry } from "@destack/account/identity";
import { directoryShape, identityShape, machineShape } from "@destack/account/object";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { defineDatabase, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { Clock } from "@destack/environment";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import {
    type Fetch,
    Follow,
    type ObjectType,
    PackageHandle,
    PackageMount,
    tablesOf,
} from "@destack/object";
import { createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import { implementPackage, ObjectStore } from "@destack/object/server";
import { TestObjectStore } from "@destack/object/test";
import { Package, type PackageId } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import universePackage from "../package.ts";

/** A cloud machine of a test universe with its directory replica, which spaces are hosted on. */
export class TestMachine implements AsyncDisposable {
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The clock the machine runs at. */
    readonly clock: Clock;
    /** The keyring keeping the keys of the spaces the machine hosts. */
    readonly keyring: LocalKeyring;
    /** The machine's replica of the universe's directory. */
    readonly directory: Directory;
    /** The universe's registry of identities, called as the machine. */
    readonly identities: IdentityRegistry;
    /** The account package's URL at the universe, and the universe's issuer. */
    readonly universe: { readonly accounts: string; readonly issuer: string };
    /** The machine's proven identity, signing its calls to the universe. */
    readonly #identity: MachineIdentity;
    /** Send a request to the universe in process. */
    readonly #send: Fetch;
    /** The store following the directory, its identities and the machine's assignments. */
    readonly #replica: TestObjectStore;
    /** The replica's database. */
    readonly #storage: TestDatabase;

    /** Keep a machine's identity and its directory replica. */
    private constructor(fields: {
        readonly identity: MachineIdentity;
        readonly keyring: LocalKeyring;
        readonly clock: Clock;
        readonly origin: string;
        readonly send: Fetch;
        readonly replica: TestObjectStore;
        readonly storage: TestDatabase;
    }) {
        // keep the machine's identity and its replica
        const accounts = PackageMount.url(fields.origin, accountPackage.id);
        this.id = fields.identity.machineId;
        this.clock = fields.clock;
        this.keyring = fields.keyring;
        this.universe = { accounts, issuer: `${accounts}${OAUTH_PATH}` };
        this.#identity = fields.identity;
        this.#send = fields.send;
        this.#replica = fields.replica;
        this.#storage = fields.storage;

        // read the directory from the replica, and register identities as the machine
        this.directory = new Directory(Snapshot.live(fields.storage.database));
        this.identities = IdentityRegistry.client(
            createClient(accountPackage, { url: accounts, fetch: this.fetch(accountPackage.id) }),
            fields.clock,
        );
    }

    /** Start a declared cloud machine of a universe, following the directory as the machine. */
    static async open(options: {
        /** The machine. */
        readonly id: Identifier<"machine">;
        /** The keychain keeping the machine's declared key. */
        readonly keychain: MemoryKeychain;
        /** The universe's origin. */
        readonly origin: string;
        /** Send a request to the universe in process. */
        readonly send: Fetch;
        /** The clock the machine runs at. */
        readonly clock: Clock;
    }): Promise<TestMachine> {
        // prove the machine by its declared key, and keep the keys of the spaces it hosts in memory
        const { id, keychain, origin, send, clock } = options;
        const identity = new MachineIdentity(id, keychain, clock);
        const keyring = await LocalKeyring.open(new MemoryKeychain(), id);
        const accounts = PackageMount.url(origin, accountPackage.id);
        const issuer = { url: `${accounts}${OAUTH_PATH}` };
        const store = createClient(accountPackage, {
            url: accounts,
            fetch: identity.fetch(send, accountPackage.id, issuer),
        }).store;

        // follow the directory, the identities and the machine's assignments as the machine
        const follows = [
            Follow.of(directoryShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
            Follow.of(identityShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
            Follow.of(machineShape, { keys: [{ key: { scope: Scope.universe.id, machine: id } }] }),
        ];
        const objects: Readonly<Record<string, ObjectType>> = {};
        const handle = PackageHandle.of(Package.of(universePackage), { objects });
        const storage = await TestDatabase.create(
            defineDatabase({
                name: "machine",
                ...tablesOf({ packages: [objectPackage], follows, tables: [] }),
            }),
            { isMigrated: true },
        );
        const replica = new ObjectStore({
            servers: [implementPackage(handle, { follows })],
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            principal: principal.machine.reference(Scope.universe.id, id),
            publisher: () => ({
                async *stream(subscription, signal) {
                    yield* await store.stream(subscription, { signal });
                },
                confirm: async (subscription, watermark) => {
                    await store.confirm({ subscription, watermark });
                },
            }),
            database: storage.database,
            clock,
            origin: { package: Package.of(universePackage), machineId: id },
            machine: id,
        });

        return new TestMachine({
            identity,
            keyring,
            clock,
            origin,
            send,
            replica: TestObjectStore.serve(replica),
            storage,
        });
    }

    /** Create a fetch calling a universe package as the machine. */
    fetch(audience: PackageId): Fetch {
        return this.#identity.fetch(this.#send, audience, { url: this.universe.issuer });
    }

    /** Send a request to another origin as it is, in process. */
    send(request: Request): Promise<Response> {
        return this.#send(request);
    }

    /** Settle once the replica follows the universe's head, failing on a reported failure. */
    async settle(): Promise<void> {
        await this.#replica.settle();
    }

    /** Stop following, then remove the replica's database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#replica[Symbol.asyncDispose]();
        await this.#storage.close();
    }
}
