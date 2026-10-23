import { Issuer } from "@destack/account/native";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { Inbox, OPERATOR_EMAIL, STACKS, TestPerson } from "@destack/account/test";
import { MemoryBucket } from "@destack/bucket/test";
import { TestDatabase } from "@destack/db/test";
import { Clock, Network } from "@destack/environment";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import { MachineIdentity } from "@destack/account/client";
import type { ResidencyCode } from "@destack/account/object";
import { CallKey, ObjectType, PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import type { Example } from "@destack/package/declare";
import { Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import type { UniverseDefinition } from "../universe/definition.ts";
import {
    type ReleaseRegistry,
    Universe,
    UNIVERSE_PACKAGES,
    universeDatabase,
    type UniverseOptions,
} from "../universe/universe.ts";
import { TestMachine } from "./machine.ts";

/** The origin a test universe answers at, which no request leaves the process for. */
const TEST_ORIGIN = "https://universe.test";

/** The residency a test universe's people live in by default. */
const TEST_RESIDENCY = "eu";

/** The kind and hardware of the personal machines a test universe enrolls. */
const PERSONAL_MACHINE = {
    kind: "personal",
    platform: "macOS",
    architecture: "arm64",
    model: "Mac14,2",
} as const;

/** What a machine declares of itself as it enrolls: personal, or a cloud machine of a residency. */
export type MachineDeclaration = Parameters<MachineIdentity["enroll"]>[1] extends infer Enrolled
    ? Enrolled extends unknown
        ? Omit<Enrolled, "scope" | "requestId" | "name">
        : never
    : never;

/** The cloud provider and location a test universe's machines declare. */
const TEST_LOCATION = { providerCode: "test", location: "test-1" } as const;

/** What a test universe declares and runs at. */
export interface TestUniverseOptions {
    /** The universe's residencies, Europe alone by default, each served by one cloud machine. */
    readonly residencies?: readonly { readonly code: ResidencyCode; readonly name: string }[];
    /** The clock it runs at, the system's by default. */
    readonly clock?: Clock;
    /** The registry it opens packages' releases from, refusing every release by default. */
    readonly releases?: ReleaseRegistry;
    /** The stacks its spaces are born with, the account test kit's by default. */
    readonly stacks?: UniverseDefinition["stacks"];
    /** The policies of other packages it decides access for, none by default. */
    readonly policies?: UniverseOptions["policies"];
    /** The access token lifetimes longer than a minute, by audience, none by default. */
    readonly lifetimes?: UniverseOptions["lifetimes"];
}

/** A residency's declared cloud machine, with the keychain keeping its key. */
interface DeclaredMachine {
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The keychain keeping the machine's declared key. */
    readonly keychain: MemoryKeychain;
}

/** The universe served in process over a test database, signed in to as its native clients do. */
export class TestUniverse implements AsyncDisposable {
    /** The universe. */
    readonly universe: Universe;
    /** The test database. */
    readonly storage: TestDatabase;
    /** The mail the universe sends. */
    readonly inbox: Inbox;
    /** The clock it runs at. */
    readonly clock: Clock;
    /** The origin the universe answers at, which no request leaves the process for. */
    readonly origin = TEST_ORIGIN;
    /** The cloud machine of each residency, with the keychain keeping its key. */
    readonly #machines: ReadonlyMap<ResidencyCode, DeclaredMachine>;
    /** The machines opened as spaces' hosts, which disposal stops first. */
    readonly #opened = new AsyncDisposableStack();
    /** The universe and its database, which disposal stops and closes after the machines. */
    readonly #disposables: AsyncDisposableStack;
    /** The issuer people sign in with, as native clients reach it. */
    readonly #issuer: Issuer;
    /** The failures the universe reported to no one, which settling throws. */
    readonly #failures: unknown[];
    /** The object types the universe serves by name, each with its package. */
    readonly #types: ReadonlyMap<
        string,
        { readonly type: ObjectType; readonly handle: (typeof UNIVERSE_PACKAGES)[number] }
    >;

    /** Keep a started universe and its adapters. */
    private constructor(
        universe: Universe,
        storage: TestDatabase,
        inbox: Inbox,
        clock: Clock,
        machines: ReadonlyMap<ResidencyCode, DeclaredMachine>,
        failures: unknown[],
        disposables: AsyncDisposableStack,
    ) {
        // keep the universe and the adapters standing in for mail, storage and time
        this.universe = universe;
        this.storage = storage;
        this.inbox = inbox;
        this.clock = clock;
        this.#machines = machines;
        this.#disposables = disposables;
        this.#failures = failures;

        // reach the issuer through the universe, and index the object types it serves
        this.#issuer = new Issuer(
            `${PackageMount.url(TEST_ORIGIN, accountPackage.id)}${OAUTH_PATH}`,
            (request) => universe.fetch(request),
        );
        this.#types = new Map(
            UNIVERSE_PACKAGES.flatMap((handle) =>
                Object.values(handle.objects)
                    .filter((declared) => declared instanceof ObjectType)
                    .map((type) => [type.name, { type, handle }] as const),
            ),
        );
    }

    /** Start a universe over a database of the project's target, mail and storage in memory. */
    static async open(options: TestUniverseOptions = {}): Promise<TestUniverse> {
        // migrate the universe's database, closed again should the start fail, and keep its mail, keys and call events in memory
        await using opening = new AsyncDisposableStack();
        const clock = options.clock ?? Clock.system;
        const storage = await TestDatabase.create(universeDatabase, { isMigrated: true });
        opening.defer(() => storage.close());
        const keyring = await LocalKeyring.read(LocalKeyring.generate());
        const key = CallKey.of(keyring);
        const inbox = new Inbox();
        const failures: unknown[] = [];
        const failed = Promise.withResolvers<never>();

        // declare one cloud machine per residency, keeping its keychain to serve as it
        const residencies = options.residencies ?? [{ code: TEST_RESIDENCY, name: "Europe" }];
        const machines = new Map<ResidencyCode, DeclaredMachine>();
        const declared = await Promise.all(
            residencies.map(async (residency) => {
                // generate the machine's key
                const id = Identifier.create("machine");
                const keychain = new MemoryKeychain();
                const { publicKey } = await new MachineIdentity(id, keychain, clock).generate();
                machines.set(residency.code, { id, keychain });

                return {
                    ...TEST_LOCATION,
                    id,
                    name: `${residency.code}-cloud`,
                    residencyId: residency.code,
                    key: publicKey,
                    origin: `https://${residency.code}.machine.test/`,
                };
            }),
        );
        const definition: UniverseDefinition = {
            residencies: [...residencies],
            platform: {
                name: "Destack",
                residencyId: TEST_RESIDENCY,
                operator: OPERATOR_EMAIL,
                reserved: [],
                publishers: [],
                handle: "destack",
                machines: declared,
            },
            stacks: options.stacks ?? STACKS,
        };

        // start the universe, refusing requests to other origins
        const universe = opening.use(
            await Universe.start({
                definition,
                origin: TEST_ORIGIN,
                database: storage.database,
                keyring,
                callKey: () => key,
                mail: inbox,
                events: new MemoryBucket(clock),
                ipAddress: { disableIpTracking: true },
                environment: {
                    clock,
                    network: {
                        ...Network.system,
                        fetch: async () => new Response(null, { status: 502 }),
                    },
                },
                report: (error) => {
                    failures.push(error);
                    failed.reject(error);
                },
                releases: () =>
                    options.releases ?? {
                        open: async (packageId, release) => {
                            throw new TypeError(
                                `the test universe opens no release ${packageId} ${release}`,
                            );
                        },
                    },
                policies: options.policies ?? [],
                lifetimes: options.lifetimes ?? new Map(),
            }),
        );

        // serve once the run applied the definition, failing on the universe's first failure
        failed.promise.catch(() => undefined);
        await Promise.race([
            Promise.all([...universe.servers.values()].map((server) => server.serving())),
            failed.promise,
        ]);

        return new TestUniverse(
            universe,
            storage,
            inbox,
            clock,
            machines,
            failures,
            opening.move(),
        );
    }

    /** Sign a person up or in with a mailed code, as a native client does, on a new client. */
    signIn(email: string): Promise<TestPerson> {
        return TestPerson.signIn(
            {
                issuer: this.#issuer,
                inbox: this.inbox,
                origin: TEST_ORIGIN,
                fetch: (request) => this.universe.fetch(request),
                clock: this.clock,
                residencyId: TEST_RESIDENCY,
            },
            email,
        );
    }

    /** Make an example's calls in the person's account as the person, each through their client. */
    async given(example: Pick<Example, "objects">, person: TestPerson): Promise<void> {
        for (const [scope, calls] of Object.entries(example.objects)) {
            // refuse calls in a scope the universe does not serve
            if (scope !== "account") {
                throw new TypeError(
                    `the test universe serves accounts alone, not the example's ${scope}`,
                );
            }

            // call each method as the person in their account, through the package's client
            for (const { object, method, input } of calls) {
                const served = this.#types.get(object.name);
                if (served === undefined) {
                    throw new TypeError(`the test universe serves no object type ${object.name}`);
                }
                const { type, handle } = served;
                const procedure: unknown = Reflect.get(
                    Reflect.get(person.client(handle), type.name),
                    method,
                );
                if (typeof procedure !== "function") {
                    throw new TypeError(`${type.name} has no method ${method}`);
                }
                await Reflect.apply(procedure, undefined, [
                    {
                        ...type.in(person.account),
                        ...(type.method(method).mutates
                            ? { requestId: RequestId.create(this.clock.now()) }
                            : {}),
                        ...input,
                    },
                ]);
            }
        }
    }

    /** Open a residency's cloud machine, following the directory, as spaces are hosted on. */
    async machine(residency: ResidencyCode = TEST_RESIDENCY): Promise<TestMachine> {
        // prove the residency's declared machine, calling the universe in process
        const declared = present(this.#machines.get(residency), `a cloud machine of ${residency}`);
        const machine = await TestMachine.open({
            ...declared,
            origin: TEST_ORIGIN,
            send: (request) => this.universe.fetch(request),
            clock: this.clock,
        });
        this.#opened.use(machine);

        return machine;
    }

    /** Enroll a machine of an account as a person, personal by default, published at its origin. */
    async enroll(
        person: TestPerson,
        accountId: Identifier<"account">,
        origin: string,
        declared: MachineDeclaration = PERSONAL_MACHINE,
    ): Promise<TestMachine> {
        // enroll the machine with a new key as the person
        const id = Identifier.create("machine");
        const keychain = new MemoryKeychain();
        const identity = new MachineIdentity(id, keychain, this.clock);
        await identity.enroll(person.client(accountPackage), {
            ...declared,
            scope: accountId,
            requestId: RequestId.create(this.clock.now()),
            name: `machine-${id.slice(-12)}`,
        });

        // follow the directory as it, and publish where it answers
        const machine = await TestMachine.open({
            id,
            keychain,
            origin: TEST_ORIGIN,
            send: (request) => this.universe.fetch(request),
            clock: this.clock,
        });
        this.#opened.use(machine);
        const accounts = createClient(accountPackage, {
            url: machine.universe.accounts,
            fetch: machine.fetch(accountPackage.id),
        });
        await accounts.machine.publish({
            scope: accountId,
            id,
            url: origin,
            relay: null,
            requestId: RequestId.create(this.clock.now()),
        });

        return machine;
    }

    /** Settle once no controller of any package has work due, failing on a reported failure. */
    async settle(): Promise<void> {
        await this.universe.idle();
        const failures = this.#failures.splice(0);
        if (failures.length > 0) {
            throw new AggregateError(failures, "the test universe failed");
        }
    }

    /** Stop the machines, then the universe, then remove its database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#opened.disposeAsync();
        await this.#disposables.disposeAsync();
    }
}
