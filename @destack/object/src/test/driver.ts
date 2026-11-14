import type { Given, SetStep } from "@destack/package/declare";
import type { Clock } from "@destack/environment";
import type { SimulatedTelemetry, Simulation } from "@destack/environment/simulation";
import { Identifier, type JsonValue, present, schema } from "@destack/schema";
import type { Driver } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { DynamicObjectClient } from "../client/index.ts";
import type { ObjectPackage } from "../server/index.ts";
import { ObjectType } from "../object/index.ts";
import { ObjectStep, objectInteraction, type ObjectObservation } from "../scenario/interaction.ts";
import { TestObjectStore } from "./store.ts";

/** The output of a call that creates or changes an object: its identifier. */
const CallOutput = schema.looseObject({ id: schema.string() });

/** A driver that plays object scenarios: people calling the packages a store serves, as given. */
export class ObjectDriver implements Driver<ObjectStep, ObjectObservation> {
    /** The interaction the driver plays. */
    static readonly interaction = objectInteraction;
    /** The settings its worlds run with: the test database's. */
    static readonly configuration = {};

    /** The world. */
    readonly #simulation: Simulation;
    /** The store serving the given packages. */
    readonly #store: TestObjectStore;
    /** The served packages. */
    readonly #packages: readonly ObjectPackage[];
    /** Each person's user identifier, by name, created on their first call. */
    readonly #people = new Map<string, Identifier<"user">>();
    /** The identifiers each call bound, by name, in call order. */
    readonly #bound = new Map<string, string[]>();
    /** The client calling each type as each person, by type and person. */
    readonly #clients = new Map<string, DynamicObjectClient>();

    /** Take a started world and the store serving the given packages. */
    constructor(
        simulation: Simulation,
        store: TestObjectStore,
        packages: readonly ObjectPackage[],
    ) {
        this.#simulation = simulation;
        this.#store = store;
        this.#packages = packages;
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#simulation.clock;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#simulation.telemetry;
    }

    /** Start a world serving the packages the given examples render. */
    static async start(given: Given<ObjectPackage>): Promise<ObjectDriver> {
        // serve the packages the examples render in a world of their own
        const simulation = testSimulation();
        const packages = given.examples.map((example) =>
            present(example.render?.(), `the package example ${example.name} renders`),
        );
        const store = await TestObjectStore.open({
            packages,
            simulation,
            ...ObjectDriver.#scope(packages, simulation),
        });

        return new ObjectDriver(simulation, store, packages);
    }

    /** Call a method as a person, binding the object it returns, until the work it woke settles. */
    async act(step: SetStep | ObjectStep): Promise<void> {
        // refuse the set step, as an object world has no example properties
        if ("action" in step) {
            throw new TypeError("an object world takes no example properties");
        }

        // call the method as the person on the resolved input
        const { type, method } = ObjectStep.target(step);
        const client = this.#client(type, step.as);
        const input = ObjectStep.input(step.input, this.#bound, (person) =>
            principal.user.reference(Scope.universe.id, this.#person(person)),
        );
        const output = await client.call(type, method, input);

        // bind the object it returns, and settle the work it woke
        if (step.bind !== undefined) {
            const bound = this.#bound.get(step.bind) ?? [];
            this.#bound.set(step.bind, [...bound, CallOutput.parse(output).id]);
        }
        for (const served of this.#packages) {
            await this.#store.server(served).idle();
        }
    }

    /** Count the objects of a type the store holds. */
    async observe(observation: ObjectObservation): Promise<JsonValue> {
        const type = this.#type(observation.count);

        return (await this.#store.database.select().from(type.table)).length;
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#simulation.fingerprint;
    }

    /** Close the store serving the packages. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#store[Symbol.asyncDispose]();
    }

    /** Serve a new scope of the type the packages' objects live in, none when they declare none. */
    static #scope(packages: readonly ObjectPackage[], simulation: Simulation) {
        // find the first scope type the served types declare
        const scopeType = packages
            .flatMap((served) => Object.values(served.objects))
            .flatMap((type) => (ObjectType.is(type) ? [type.scope].flat() : []))
            .find((scope): scope is ObjectType => ObjectType.is(scope));

        return scopeType === undefined
            ? {}
            : {
                  scope: scopeType.reference(
                      Scope.universe.id,
                      Identifier.create(scopeType.name, simulation.environment("store")),
                  ),
              };
    }

    /** Read a person's user identifier, creating it on their first call. */
    #person(name: string): Identifier<"user"> {
        // reuse the person's identifier, else create one
        const known = this.#people.get(name);
        if (known !== undefined) {
            return known;
        }
        const created = Identifier.create("user", this.#store.environment);
        this.#people.set(name, created);

        return created;
    }

    /** Find a served type by name. */
    #type(name: string): ObjectType {
        const types = this.#packages.flatMap((served) => Object.values(served.objects));

        return present(
            types.find((type): type is ObjectType => ObjectType.is(type) && type.name === name),
            `a served object type ${name}`,
        );
    }

    /** Read the client calling a type as a person, one per type and person as an app has one. */
    #client(name: string, person: string): DynamicObjectClient {
        // reuse the client, else open one at the package serving the type as the person
        const key = `${name} ${person}`;
        const known = this.#clients.get(key);
        if (known !== undefined) {
            return known;
        }
        const type = this.#type(name);
        const served = present(
            this.#packages.find((each) => Object.values(each.objects).includes(type)),
            `the package serving ${name}`,
        );
        const client = new DynamicObjectClient({
            objects: [type],
            scope: this.#store.scope,
            endpoint: this.#store.as(this.#person(person)).endpoint(served),
            environment: this.#store.environment,
        });
        this.#clients.set(key, client);

        return client;
    }
}
