import type { Given, SetStep } from "@destack/package/declare";
import type { Clock } from "@destack/environment";
import type { SimulatedTelemetry, Simulation } from "@destack/environment/simulation";
import { TestDatabase } from "@destack/db/test";
import { Identifier, type JsonValue, present, schema } from "@destack/schema";
import type { Driver } from "@destack/test";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { ObjectClient, ProcedureRemote } from "../client/index.ts";
import type { ObjectPackage } from "../server/index.ts";
import { ObjectType } from "../object/index.ts";
import {
    type ClientStep,
    clientInteraction,
    ObjectStep,
    type ObjectObservation,
} from "../scenario/interaction.ts";
import { ObjectDriver } from "./driver.ts";
import { TestObjectStore } from "./store.ts";
import { testSimulation } from "./world.ts";

/** The output of a call that creates or changes an object: its identifier. */
const CallOutput = schema.looseObject({ id: schema.string() });

/** One person's client: its replica's database, the client and its running loops. */
interface PersonClient {
    /** The client. */
    readonly client: ObjectClient;
    /** The live queries it reads, by type: settled once replicated, then read from the replica. */
    readonly queries: Map<
        string,
        { readonly ready: Promise<void>; readonly read: () => Promise<unknown> }
    >;
}

/** A driver that plays client scenarios: each person calls and reads through their own client. */
export class ClientDriver implements Driver<ClientStep, ObjectObservation> {
    /** The interaction the driver plays. */
    static readonly interaction = clientInteraction;
    /** The settings its worlds run with: the test database's. */
    static readonly configuration = {};

    /** The world. */
    readonly #simulation: Simulation;
    /** The store serving the given packages, each person's client's home. */
    readonly #store: TestObjectStore;
    /** The served types, by name. */
    readonly #types: ReadonlyMap<string, ObjectType>;
    /** The served packages. */
    readonly #packages: readonly ObjectPackage[];
    /** Each person's user identifier, by name, created on their first step. */
    readonly #users = new Map<string, Identifier<"user">>();
    /** Each person's client, by name, opened on their first step. */
    readonly #people = new Map<string, Promise<PersonClient>>();
    /** The identifiers each call bound, by name, in call order. */
    readonly #bound = new Map<string, string[]>();
    /** What disposal closes: the clients, their loops and their databases. */
    readonly #disposables = new AsyncDisposableStack();

    /** Take a started world and the store serving the given packages. */
    private constructor(
        simulation: Simulation,
        store: TestObjectStore,
        packages: readonly ObjectPackage[],
    ) {
        // keep the world, the store, the packages and the served types by name
        this.#simulation = simulation;
        this.#store = store;
        this.#packages = packages;
        this.#types = new Map(
            packages
                .flatMap((served) => Object.values(served.objects))
                .filter((type): type is ObjectType => ObjectType.is(type))
                .map((type) => [type.name, type]),
        );
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#simulation.clock;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#simulation.telemetry;
    }

    /** Start a world serving the packages the given examples render, as the object driver does. */
    static async start(given: Given<ObjectPackage>): Promise<ClientDriver> {
        // serve the packages the examples render in a world of their own
        const simulation = testSimulation();
        const packages = given.examples.map((example) =>
            present(example.render?.(), `the package example ${example.name} renders`),
        );
        const store = await TestObjectStore.open({
            packages,
            simulation,
            ...ObjectDriver.scope(packages, simulation),
        });

        return new ClientDriver(simulation, store, packages);
    }

    /** Take a person's step through their client: a call, or a query of its replica. */
    async act(step: SetStep | ClientStep): Promise<void> {
        // refuse the set step, as a client world has no example properties
        if ("action" in step) {
            throw new TypeError("a client world takes no example properties");
        }
        const person = await this.#person(step.as);

        // read a type's objects from the client's replica, following them once
        if ("query" in step) {
            const type = this.#type(step.query);
            const live =
                person.queries.get(type.name) ??
                person.client.queryType(type).findMany().subscribe();
            person.queries.set(type.name, live);
            await live.ready;
            await live.read();
        }
        // call a method through the client, waiting for its prediction or confirmation
        else {
            const { type, method } = ObjectStep.target(step);
            const input = ObjectStep.input(step.input, this.#bound, (name) =>
                principal.user.reference(Scope.universe.id, this.#user(name)),
            );
            const mutation = person.client.call(this.#type(type), method, input);
            const output = await mutation.predicted;
            if (step.until === "confirmed") {
                await mutation.confirmed;
            }

            // bind the object it returns
            if (step.bind !== undefined) {
                const bound = this.#bound.get(step.bind) ?? [];
                this.#bound.set(step.bind, [...bound, CallOutput.parse(output).id]);
            }
        }
    }

    /** Count the objects of a type the home holds. */
    async observe(observation: ObjectObservation): Promise<JsonValue> {
        const type = this.#type(observation.count);

        return (await this.#store.database.select().from(type.table)).length;
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#simulation.fingerprint;
    }

    /** Close the clients, then the store serving the packages. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposables.disposeAsync();
        await this.#store[Symbol.asyncDispose]();
    }

    /** Read a person's user identifier, creating it on first use. */
    #user(name: string): Identifier<"user"> {
        const user = this.#users.get(name) ?? Identifier.create("user", this.#store.environment);
        this.#users.set(name, user);

        return user;
    }

    /** Open a person's client on their first step, over its own database, pushing and following. */
    #person(name: string): Promise<PersonClient> {
        // reuse the person's client, else open it once
        const known = this.#people.get(name);
        if (known !== undefined) {
            return known;
        }
        const opened = this.#open(name);
        this.#people.set(name, opened);

        return opened;
    }

    /** Open a person's client over a database of its own, running its loops until disposal. */
    async #open(name: string): Promise<PersonClient> {
        // store the client's replica in a database of its own
        const { environment } = this.#store;
        const objects = ObjectClient.keyed([...this.#types.values()]);
        const storage = await TestDatabase.create(ObjectClient.tables(objects), {
            environment,
            dialect: "sqlite",
        });
        this.#disposables.defer(() => storage.close());

        // open the client toward the store as the person, the first package serving the scope
        const user = this.#user(name);
        const served = present(this.#packages[0], "a served package");
        const client = await ObjectClient.open({
            database: storage.database,
            objects,
            scope: this.#store.scope,
            caller: principal.user.reference(Scope.universe.id, user),
            remote: ProcedureRemote.at({
                scope: this.#store.scope,
                endpoint: this.#store.as(user).endpoint(served),
                environment,
            }),
            environment,
        });
        this.#disposables.defer(() => client.close());

        // push and follow until disposal
        const stopping = new AbortController();
        const running = client.run(stopping.signal);
        this.#disposables.defer(async () => {
            stopping.abort();
            await running;
        });

        return { client, queries: new Map() };
    }

    /** Find a served type by name. */
    #type(name: string): ObjectType {
        return present(this.#types.get(name), `a served object type ${name}`);
    }
}
