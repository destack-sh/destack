import { testHost } from "@destack/test/host";
import { principal } from "@destack/access";
import * as account from "@destack/account/object";
import type { Simulation, SimulatedClock } from "@destack/environment/simulation";
import { notification } from "@destack/notification";
import { TestNotification } from "@destack/notification/test";
import {
    ObjectType,
    PACKAGE_DECLARATION,
    PACKAGE_EXPORT,
    type PackageHandle,
} from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import { ObjectClient } from "@destack/object/client";
import type { TestCaller, TestObjectStore } from "@destack/object/test";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { nth, schema } from "@destack/schema";
import { ids, TestPackage, TestSpace } from "@destack/stack/test";
import { type Call, Scope } from "@destack/sync";
import { Table } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { Mention, mention, presence, reply, thread } from "../object/index.ts";
import socialPackage from "../package.ts";
import socialServer from "../server.ts";
import { articlesPackage, articlesServer } from "./article.ts";

/** When the kit's simulation starts: 16 October 2026, UTC. */
export const START = Date.UTC(2026, 9, 16);

/** The packages the space installs, in dependency order. */
export const PACKAGES: readonly TestPackage[] = [
    ...(await TestNotification.packages(testHost().crypto)),
    {
        handle: articlesPackage,
        build: await packageBuild(articlesPackage, []),
        server: articlesServer,
        connectors: [],
        servers: [],
        bindables: [],
        providers: [],
    },
    {
        handle: socialPackage,
        build: await packageBuild(
            socialPackage,
            [mention, thread, reply].map((declared) => ({
                kind: "notification",
                kindPackageId: notification.package.id,
                name: declared.name,
                description: declared.describe(),
            })),
        ),
        server: socialServer,
        connectors: [],
        servers: [],
        bindables: [],
        providers: [],
    },
];

/** The agent package, whose installation comments and reacts beside the people. */
const AGENT = Package.parse({
    id: "package-019f5530-8000-7000-8000-0000000000e3",
    name: "@example/agent",
    version: "2026.10.0",
});

/** The people reading the space, who share articles with each other, by name. */
const PEOPLE = ["alice", "bob", "carol", "dave"] as const;

/** A page of a list call, its rows read for their identifier and creation time. */
const ListPage = schema.looseObject({
    items: schema.array(schema.looseObject({ id: schema.string(), createdAt: schema.number() })),
});

/** The world the kit writes on its space once: the agent's installation. */
const SocialWorld = schema.object({
    /** The agent package's installation. */
    agent: schema.string(),
});

/** The world the kit writes on its space once. */
type SocialWorld = schema.Infer<typeof SocialWorld>;

/** The space the kit's tests copy: its packages, shipped servers and start. */
const SPACE_OPTIONS = { ...TestPackage.options(PACKAGES), start: START };

/** The template of the kit's space, which the run writes once and every test copies. */
export const SOCIAL_TEMPLATE = TestSpace.template(SPACE_OPTIONS, {
    name: "social",
    value: SocialWorld,
    write: (space) => TestSocial.write(space),
});

/** A principal the tests act as: a person reading the space, or the agent. */
export type Actor = (typeof PEOPLE)[number] | "agent";

/** A principal as comments mention it. */
export type Mentioned = Mention["principal"];

/** A space installing articles with social attachments, used by people and an agent. */
export class TestSocial implements AsyncDisposable {
    /** The space serving social. */
    readonly space: TestSpace;
    /** The served space. */
    readonly served: TestObjectStore;
    /** The simulation the space runs on. */
    readonly simulation: Simulation;
    /** The principals the tests act as: people reading the space, and the agent installation. */
    readonly actors: Readonly<Record<Actor, Mentioned>>;
    /** Each actor's caller: people with their clients' tokens, the agent as its installation. */
    readonly #callers: Readonly<Record<Actor, TestCaller>>;
    /** The space's selection, which every call spreads. */
    readonly scope = { spaceId: ids.space };
    /** The followers reading presence, closed once the kit closes. */
    readonly #followers: TestFollower[] = [];

    /** Keep a served space with its actors. */
    private constructor(fields: {
        readonly space: TestSpace;
        readonly served: TestObjectStore;
        readonly simulation: Simulation;
        readonly actors: Readonly<Record<Actor, Mentioned>>;
        readonly callers: Readonly<Record<Actor, TestCaller>>;
    }) {
        // keep the space, how it is served, and its actors with their callers
        this.space = fields.space;
        this.served = fields.served;
        this.simulation = fields.simulation;
        this.actors = fields.actors;
        this.#callers = fields.callers;
    }

    /** Serve a space with articles, social and an agent, read by four people, in a test's world. */
    static async open(): Promise<TestSocial> {
        // copy the space with its installations, people and roles into a fork of its world
        const { space, world } = await TestSpace.copy(SOCIAL_TEMPLATE, SPACE_OPTIONS);

        // serve it, driving the work through the fork
        return space.simulation.drive(TestSocial.#open(space, world));
    }

    /** Serve a copied space to its people and the agent. */
    static async #open(space: TestSpace, world: SocialWorld): Promise<TestSocial> {
        // serve the space with notifications, read by its people and the agent
        const { simulation } = space;
        const served = await space.serve();
        const people = PEOPLE.map((name) => space.universe.person(TestSocial.#email(name)));
        const actors = {
            alice: TestSocial.#mentioned(principal.user, nth(people, 0).id),
            bob: TestSocial.#mentioned(principal.user, nth(people, 1).id),
            carol: TestSocial.#mentioned(principal.user, nth(people, 2).id),
            dave: TestSocial.#mentioned(principal.user, nth(people, 3).id),
            agent: TestSocial.#mentioned(principal.installation, world.agent),
        };

        // call as the people and the agent's installation, driven through the simulation
        const drive = <Value>(work: Promise<Value>) => simulation.drive(work);
        const callers = {
            alice: space.person(served, nth(people, 0)).driven(drive),
            bob: space.person(served, nth(people, 1)).driven(drive),
            carol: space.person(served, nth(people, 2)).driven(drive),
            dave: space.person(served, nth(people, 3)).driven(drive),
            agent: served
                .as(principal.installation.reference(ids.space, world.agent))
                .driven(drive),
        };

        return new TestSocial({ space, served, simulation, actors, callers });
    }

    /** Install articles, social and the agent, sign the people up and let them read the space. */
    static async write(space: TestSpace): Promise<SocialWorld> {
        // install each package in dependency order, as the space's stack does, then the agent
        const installing = await space.serve();
        for (const installed of PACKAGES) {
            await space.install(installing, installed.build);
        }
        const agent = await space.install(
            installing,
            await MemoryBuild.of(AGENT, [], testHost().crypto),
        );
        await installing[Symbol.asyncDispose]();

        // serve again, as the machine does once installations change, and sign the people up
        await using served = await space.serve();
        const people = await Promise.all(
            PEOPLE.map((name) => space.universe.signIn(TestSocial.#email(name))),
        );

        // let the people and the agent read the space through a role its stack declares
        const readers = [...people.map((person) => person.id), agent];
        await space.apply(served, [], {
            roles: {
                reader: {
                    description: "Read the space",
                    permissions: [account.space.permission("read")],
                },
            },
            relationships: Object.fromEntries(
                readers.map((id, index) => [
                    `reader-${index}`,
                    { role: "reader", subject: { id } },
                ]),
            ),
        });

        return { agent };
    }

    /** Address a person reading the space by name. */
    static #email(name: (typeof PEOPLE)[number]): string {
        return `${name}@example.test`;
    }

    /** Refer to a person or installation as comments mention it. */
    static #mentioned(
        kind: typeof principal.user | typeof principal.installation,
        id: string,
    ): Mentioned {
        const scope = kind === principal.user ? Scope.universe.id : ids.space;

        return Mention.shape.principal.parse(kind.reference(scope, id));
    }

    /** Refer to a host as its attachments' parent. */
    static host(object: ObjectType, id: string) {
        return {
            parent: { packageId: object.policy.definition.packageId, type: object.name, id },
        };
    }

    /** Call as an actor through its client, then step the world a millisecond on. */
    async call<Result>(
        actor: Actor,
        called: Call<Result>,
        writerId = `writer-${actor}`,
    ): Promise<Result> {
        // call as the actor, a query, else a mutation, presence as its writer
        const result = await this.#callers[actor].call(called, { writerId });

        // step the world past the call's millisecond, so the next write orders after it by time
        await this.simulation.run({ to: this.clock.now() + 1 });

        return result;
    }

    /** The clock the space's servers read. */
    get clock(): SimulatedClock {
        return this.simulation.clock;
    }

    /** List the objects of a type an actor may list in the space, oldest first. */
    async list<Type extends ObjectType>(actor: Actor, object: Type, input: object = {}) {
        // list them as the actor
        const listed: ObjectType = object;
        const page = await this.call(actor, listed.call("list", { ...this.scope, ...input }));

        return ListPage.parse(page)
            .items.toSorted((left, right) => left.createdAt - right.createdAt)
            .map((row) => Table.selectSchema<Type["table"]>(object.table).parse(row));
    }

    /** Settle the space's controllers under the simulation. */
    settle(): Promise<void> {
        return this.simulation.drive(this.space.settle(this.served));
    }

    /** Follow the space's presence as an actor's client does, replicating it over HTTP. */
    async follow(actor: Actor): Promise<TestFollower> {
        // replicate presence into a local database through the actor's endpoint of social
        const caller = this.#callers[actor];
        const environment = this.simulation.environment(`follower-${actor}`);
        const database = await connectBunSqlite(":memory:", ObjectClient.tables({ presence }), {
            environment,
        });
        const resources = new AsyncDisposableStack();
        resources.defer(() => database.close());
        const client = await ObjectClient.open({
            environment,
            database,
            objects: { presence },
            scope: ids.space,
            caller: caller.subject,
            endpoint: caller.endpoint(socialPackage),
            reconnect: (machine) => {
                throw new TypeError(`no scope moves in this test, yet one moved to ${machine}`);
            },
        });

        // keep the follower until the kit closes, reading its rows under the simulation
        const follower = new TestFollower(client, resources, (work) => this.simulation.drive(work));
        this.#followers.push(follower);

        return follower;
    }

    /** End the follows, stop serving the space, then close it. */
    async [Symbol.asyncDispose](): Promise<void> {
        // end the follows before the databases close
        await this.simulation.drive(
            Promise.all(this.#followers.map((follower) => follower.close())),
        );

        // stop serving, then close the space, under the simulation
        await this.simulation.drive(this.served[Symbol.asyncDispose]());
        await this.simulation.drive(this.space[Symbol.asyncDispose]());
    }
}

/** A client following presence, reading its rows each time they change. */
export class TestFollower {
    /** The client replicating presence. */
    readonly #client: ObjectClient<{ readonly presence: typeof presence }>;
    /** The resources the client runs on, such as its local database. */
    readonly #resources: AsyncDisposableStack;
    /** The cancellation of the client's run and its watch. */
    readonly #stopping = new AbortController();
    /** The client's run, settled once stopped. */
    readonly #running: Promise<void>;
    /** The failures the client reported while running. */
    readonly #failures: unknown[] = [];
    /** The presence rows after each local commit. */
    readonly #values: AsyncGenerator<readonly Record<string, unknown>[]>;
    /** The rows read last, as JSON. */
    #last: string | undefined;
    /** Run work under the kit's simulation. */
    readonly #drive: <Value>(work: Promise<Value>) => Promise<Value>;

    /** Run a client and watch its presence rows. */
    constructor(
        client: ObjectClient<{ readonly presence: typeof presence }>,
        resources: AsyncDisposableStack,
        drive: <Value>(work: Promise<Value>) => Promise<Value>,
    ) {
        // keep the client, its resources and the drive, then run it and watch its presence rows
        this.#client = client;
        this.#resources = resources;
        this.#drive = drive;
        this.#running = client.run(this.#stopping.signal, (error) => this.#failures.push(error));
        this.#values = client.query.presence.findMany().subscribe().watch(this.#stopping.signal);
    }

    /** Read the rows once they differ from the rows read last, under the simulation. */
    rows(): Promise<Record<string, unknown>[]> {
        return this.#drive(this.#changed());
    }

    /** Read the rows once they differ from the rows read last. */
    async #changed(): Promise<Record<string, unknown>[]> {
        // read values until one differs from the last read, keeping the watch open for the next read
        for (
            let next = await this.#values.next();
            next.done !== true;
            next = await this.#values.next()
        ) {
            const text = JSON.stringify(next.value);
            if (text !== this.#last) {
                this.#last = text;

                return [...next.value];
            }
        }
        throw new TypeError("the presence follow ended before its rows changed");
    }

    /** Stop the client, refusing any failure it reported, and close its database. */
    async close(): Promise<void> {
        // stop the run, then close the client and its database
        this.#stopping.abort();
        await this.#running;
        await this.#client.close();
        await this.#resources.disposeAsync();

        // refuse a failure the client reported
        if (this.#failures.length > 0) {
            throw new AggregateError(this.#failures, "the presence follow failed");
        }
    }
}

/** Write a package's build describing it, with further declarations. */
function packageBuild(
    handle: PackageHandle,
    declared: Parameters<typeof MemoryBuild.of>[1],
): ReturnType<typeof MemoryBuild.of> {
    return MemoryBuild.of(
        Package.of(handle),
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
                name: PACKAGE_EXPORT,
                description: describePackage(handle),
            },
            ...declared,
        ],
        testHost().crypto,
    );
}
