import { principal } from "@destack/access";
import { Directory } from "@destack/account/address";
import { MachineIdentity } from "@destack/account/client";
import { IdentityRegistry } from "@destack/account/identity";
import { directoryShape, identityShape, machineShape } from "@destack/account/object";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { defineDatabase, type LogPosition, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { Clock, Environment } from "@destack/environment";
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
import { Identifier, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import universePackage from "../package.ts";

/** A test machine's state beside its database: the keyring secrets of the spaces' keys. */
export const MachineWorld = schema.object({
    /** The secrets of the keychain under the keyring, by name. */
    secrets: schema.record(schema.string(), schema.string()),
});
/** A test machine's state beside its database: the keyring secrets of the spaces' keys. */
export type MachineWorld = schema.Infer<typeof MachineWorld>;

/** A cloud machine of a test universe with its directory replica, which spaces are hosted on. */
export class TestMachine implements AsyncDisposable {
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The clock, network and randomness the machine runs in. */
    readonly environment: Environment;
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
    /** The keychain under the keyring of the spaces' keys. */
    readonly #keychain: MemoryKeychain;
    /** Whether the machine created its replica's database, and closes it. */
    readonly #isOwned: boolean;

    /** The migrated replica database of a test machine, the same for every machine. */
    static readonly database = defineDatabase({
        name: "machine",
        ...tablesOf({
            packages: [objectPackage],
            follows: [
                Follow.of(directoryShape, { keys: [] }),
                Follow.of(identityShape, { keys: [] }),
                Follow.of(machineShape, { keys: [] }),
            ],
            tables: [],
        }),
    });

    /** Keep a machine's identity and its directory replica. */
    private constructor(fields: {
        readonly identity: MachineIdentity;
        readonly keyring: LocalKeyring;
        readonly environment: Environment;
        readonly origin: string;
        readonly send: Fetch;
        readonly replica: TestObjectStore;
        readonly storage: TestDatabase;
        readonly keychain: MemoryKeychain;
        readonly isOwned: boolean;
    }) {
        // keep the machine's identity and its replica
        const accounts = PackageMount.url(fields.origin, accountPackage.id);
        this.id = fields.identity.machineId;
        this.environment = fields.environment;
        this.keyring = fields.keyring;
        this.universe = { accounts, issuer: `${accounts}${OAUTH_PATH}` };
        this.#identity = fields.identity;
        this.#send = fields.send;
        this.#replica = fields.replica;
        this.#storage = fields.storage;
        this.#keychain = fields.keychain;
        this.#isOwned = fields.isOwned;

        // read the directory from the replica, and register identities as the machine
        this.directory = new Directory(Snapshot.live(fields.storage.database));
        this.identities = IdentityRegistry.client(
            createClient(
                accountPackage,
                { url: accounts, fetch: this.fetch(accountPackage.id) },
                fields.environment,
            ),
            fields.environment,
        );
    }

    /** The clock the machine runs at, its environment's. */
    get clock(): Clock {
        return this.environment.clock;
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
        /** The clock, network and randomness the machine runs in. */
        readonly environment: Environment;
        /** The replica database it follows into, which the caller closes, a new one by default. */
        readonly storage?: TestDatabase;
        /** The keyring secrets a template kept, new ones by default. */
        readonly world?: MachineWorld;
    }): Promise<TestMachine> {
        // prove the machine by its declared key, and keep the keys of the spaces it hosts in memory, the world's when restored
        const { id, keychain, origin, send, environment } = options;
        const identity = new MachineIdentity(id, keychain, environment);
        const kept = new MemoryKeychain(options.world?.secrets);
        const keyring = await LocalKeyring.open(kept, id, environment);
        const accounts = PackageMount.url(origin, accountPackage.id);
        const issuer = { url: `${accounts}${OAUTH_PATH}` };
        const store = createClient(
            accountPackage,
            {
                url: accounts,
                fetch: identity.fetch(send, accountPackage.id, issuer),
            },
            environment,
        ).store;

        // follow the directory, the identities and the machine's assignments as the machine, into the given database or a new one
        const follows = TestMachine.#follows(id);
        const objects: Readonly<Record<string, ObjectType>> = {};
        const handle = PackageHandle.of(Package.of(universePackage), { objects });
        const storage =
            options.storage ??
            (await TestDatabase.create(TestMachine.database, {
                environment,
                isMigrated: true,
            }));
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
            environment,
            origin: { package: Package.of(universePackage), machineId: id },
            machine: id,
        });

        return new TestMachine({
            identity,
            keyring,
            environment,
            origin,
            send,
            replica: TestObjectStore.serve(replica),
            storage,
            keychain: kept,
            isOwned: options.storage === undefined,
        });
    }

    /** Create a fetch calling a universe package as the machine. */
    fetch(audience: PackageId): Fetch {
        return this.#identity.fetch(this.#send, audience, { url: this.universe.issuer });
    }

    /** Read the machine's access token for a package, as the universe grants it. */
    token(audience: PackageId): Promise<string> {
        return this.#identity.token(audience, { url: this.universe.issuer }, this.#send);
    }

    /** Send a request to another origin as it is, in process. */
    send(request: Request): Promise<Response> {
        return this.#send(request);
    }

    /** Wait until the replica follows the universe up to a position of its log, then settle it. */
    async follow(head: LogPosition, signal: AbortSignal): Promise<void> {
        await this.#replica.store.wait({ scope: Scope.universe.id, ...head }, signal);
        await this.settle();
    }

    /** Settle once the replica follows the universe's head, failing on a reported failure. */
    async settle(): Promise<void> {
        await this.#replica.settle();
    }

    /** Read the keyring secrets of the spaces' keys, as a template's value keeps them. */
    world(): MachineWorld {
        return { secrets: Object.fromEntries(this.#keychain.secrets) };
    }

    /** Name a test machine after the end of its id, as the universe's kit enrolls it. */
    static nameOf(id: Identifier<"machine">): string {
        return `machine-${id.slice(-12)}`;
    }

    /** List the follows of a machine: the directory, the identities and its assignments. */
    static #follows(id: Identifier<"machine">) {
        return [
            Follow.of(directoryShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
            Follow.of(identityShape, { keys: [{ key: { scope: Scope.universe.id } }] }),
            Follow.of(machineShape, { keys: [{ key: { scope: Scope.universe.id, machine: id } }] }),
        ];
    }

    /** Stop following, then remove the replica's database when the machine created it. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#replica[Symbol.asyncDispose]();
        if (this.#isOwned) {
            await this.#storage.close();
        }
    }
}
