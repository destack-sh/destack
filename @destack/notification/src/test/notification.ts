import type { Crypto, Environment } from "@destack/environment";
import * as account from "@destack/account/object";
import accountPackage from "@destack/account/package";
import type { TestPerson } from "@destack/account/test";
import { asc, eq } from "@destack/db";
import type { Simulation, SimulatedClock } from "@destack/environment/simulation";
import type { LocaleTag } from "@destack/locale";
import {
    MAIL_PROVIDER,
    subscribeBrowser,
    TestMailTransport,
    TestMessage,
    type Browser,
} from "@destack/message/test";
import { mailConnector } from "@destack/message/email";
import { message } from "@destack/message/object";
import type { Outcome } from "@destack/message/provider";
import { MethodContext, PACKAGE_DECLARATION, PACKAGE_EXPORT } from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import { implementPackage } from "@destack/object/server";
import type { TestCaller, TestObjectStore } from "@destack/object/test";
import { Package } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { type Identifier, present, schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { describeSetting } from "@destack/setting/inspect";
import settingPackage from "@destack/setting/package";
import { ids, TestPackage, TestSpace } from "@destack/stack/test";
import { type Call, RequestId, type Subject } from "@destack/sync";
import { testHost } from "@destack/test/host";
import germanCatalog from "../../locale/de.json" with { type: "json" };
import { PUSH_CAPABILITY } from "../message/recipient.ts";
import { Inbox } from "../notification/inbox.ts";
import { activity, Notification, notification, summary } from "../object/index.ts";
import notificationPackage from "../package.ts";
import { NotificationServer } from "../server/server.ts";
import { focus, preferences, summarySchedule } from "../setting/setting.ts";
import { notesPackage, notesServer, openNotesBuild } from "./notes.ts";

/** When the kit's world starts: Monday 28 September 2026, 10:00 UTC, noon in Vienna. */
const START = Date.UTC(2026, 8, 28, 10, 0);

/** The origin of the push service the people's browsers subscribe at. */
export const PUSH_ORIGIN = "https://push.example";

/** The port HTTPS origins listen at (RFC 9110 4.2.2). */
const HTTPS_PORT = 443;

/** The status a push service answers an accepted push with (RFC 8030 5). */
const CREATED = 201;

/** The status a push service answers a forgotten subscription with (RFC 8030 7.3). */
const GONE = 410;

/** The most settle rounds a dispatch runs, as a notification crosses a few stores. */
const DISPATCH_ROUNDS = 10;

/** The time a jump runs before its target: two 10 s feed heartbeats, inside access's 30 s lag. */
const CONFIRM_MILLISECONDS = 20_000;

/** The types a person's home serves, which calls address in the home. */
const HOME_TYPES: ReadonlySet<string> = new Set([notification.name, summary.name, message.name]);

/** The people the tests act as and notify, each signed in under their name. */
export const PEOPLE = ["alice", "bob", "carol", "dave", "erin"] as const;

/** A person the tests act as and notify. */
export type Person = (typeof PEOPLE)[number];

/** A person's personal home, served beside the space. */
export interface TestHome {
    /** The hosted home. */
    readonly space: TestSpace;
    /** The served home. */
    readonly served: TestObjectStore;
    /** The person's caller in the home, with their client's tokens. */
    readonly caller: TestCaller;
}

/** A push a browser's push service received. */
export interface TestPush {
    /** The subscription URL it went to. */
    readonly url: string;
    /** The urgency header. */
    readonly urgency: string;
    /** The lifetime header, in seconds. */
    readonly ttl: number;
    /** The topic header. */
    readonly topic: string;
    /** The encrypted body. */
    readonly body: Uint8Array<ArrayBuffer>;
}

/** A push service on a simulated network, storing each push and forgetting some subscriptions. */
export class TestPushService {
    /** The pushes received, in order. */
    readonly requests: TestPush[] = [];
    /** The subscription URLs the service forgot. */
    readonly gone = new Set<string>();

    /** Listen at the push origin of a simulation's network, as its own node. */
    static listen(simulation: Simulation): TestPushService {
        // answer each push at the origin's host as a node of that name
        const service = new TestPushService();
        const { hostname } = new URL(PUSH_ORIGIN);
        simulation.network.server(hostname).http({
            hostname,
            port: HTTPS_PORT,
            fetch: (request) => service.#receive(request),
        });

        return service;
    }

    /** Store a push with its headers, answering a forgotten subscription as gone. */
    async #receive(request: Request): Promise<Response> {
        const { url, headers } = request;
        this.requests.push({
            url,
            urgency: present(headers.get("Urgency"), "the urgency"),
            ttl: Number(present(headers.get("TTL"), "the lifetime")),
            topic: present(headers.get("Topic"), "the topic"),
            body: await request.bytes(),
        });

        return new Response(null, { status: this.gone.has(url) ? GONE : CREATED });
    }
}

/** A space with the documents application and notifications, and five people with their homes. */
export class TestNotification implements AsyncDisposable {
    /** The space serving the documents and their notifications. */
    readonly space: TestSpace;
    /** The served space. */
    readonly served: TestObjectStore;
    /** The clock the servers read. */
    readonly clock: SimulatedClock;
    /** The people the tests act as, signed in at the universe. */
    readonly people: Readonly<Record<Person, TestPerson>>;
    /** The people's hosted homes, each served on first use. */
    readonly #hosted: ReadonlyMap<Person, TestSpace>;
    /** The homes served so far, by person. */
    readonly #homes = new Map<Person, Promise<TestHome>>();
    /** Every space the world hosts, by identifier. */
    readonly #spaces: ReadonlyMap<Identifier<"space">, TestSpace>;
    /** The other hosted spaces served so far, such as the account's home. */
    readonly #others: TestHome[] = [];
    /** The mail transport the homes' messages send through. */
    readonly mails: TestMailTransport;
    /** The push service the people's browsers subscribe at. */
    readonly pushes: TestPushService;
    /** The space's selection, which every call of its types spreads. */
    readonly scope = { spaceId: ids.space };
    /** Each person's caller in the space, with their client's tokens. */
    readonly #callers: Readonly<Record<Person, TestCaller>>;

    /** The world the space's servers run in, its store's. */
    get environment(): Environment {
        return this.served.environment;
    }

    /** Open a kit over a served space, its people and their homes. */
    private constructor(fields: {
        readonly space: TestSpace;
        readonly served: TestObjectStore;
        readonly people: Readonly<Record<Person, TestPerson>>;
        readonly hosted: ReadonlyMap<Person, TestSpace>;
        readonly spaces: ReadonlyMap<Identifier<"space">, TestSpace>;
        readonly mails: TestMailTransport;
        readonly pushes: TestPushService;
    }) {
        // store the space, its people, their homes and the outside systems
        this.space = fields.space;
        this.served = fields.served;
        this.clock = fields.space.simulation.clock;
        this.people = fields.people;
        this.#hosted = fields.hosted;
        this.#spaces = fields.spaces;
        this.mails = fields.mails;
        this.pushes = fields.pushes;
        this.#callers = {
            alice: fields.space.person(fields.served, fields.people.alice),
            bob: fields.space.person(fields.served, fields.people.bob),
            carol: fields.space.person(fields.served, fields.people.carol),
            dave: fields.space.person(fields.served, fields.people.dave),
            erin: fields.space.person(fields.served, fields.people.erin),
        };
    }

    /** Serve the space with its people's homes, mails answering in turn. */
    static async open(
        options: {
            /** The recipients of one announcement batch, the server's default when absent. */
            readonly batch?: number;
            /** The outcomes the mail transport answers in turn, every email sent when absent. */
            readonly mails?: readonly Outcome[];
        } = {},
    ): Promise<TestNotification> {
        // copy the template's world with the mail bound to the transport and the notifications batched as asked
        const { batch } = options;
        const mails = new TestMailTransport(options.mails);
        const servers = SPACE_OPTIONS.servers.map((server) =>
            batch === undefined || server.package.id !== notificationPackage.id
                ? server
                : implementPackage(
                      notificationPackage,
                      new NotificationServer({ batch }).definition(),
                  ),
        );
        const { space, spaces } = await TestSpace.copy(TestNotification.template(), {
            ...SPACE_OPTIONS,
            servers,
            connectors: [mailConnector(MAIL_PROVIDER, mails)],
        });

        // serve the space beside the push service, finding each person's hosted home
        const pushes = TestPushService.listen(space.simulation);
        const served = await space.serve();
        const person = (name: Person) => space.universe.person(TestNotification.#email(name));
        const people = {
            alice: person("alice"),
            bob: person("bob"),
            carol: person("carol"),
            dave: person("dave"),
            erin: person("erin"),
        };
        const hosted = new Map(
            PEOPLE.map((name) => [
                name,
                present(
                    [...spaces.values()].find((each) => each.owner.id === people[name].id),
                    `the home of ${name}`,
                ),
            ]),
        );

        return new TestNotification({ space, served, people, hosted, spaces, mails, pushes });
    }

    /** Read notification as a package a space installs, with message beside it. */
    static async packages(crypto: Crypto): Promise<TestPackage[]> {
        return [
            await TestMessage.package(crypto),
            {
                handle: notificationPackage,
                build: await TestNotification.build(crypto),
                server: implementPackage(
                    notificationPackage,
                    new NotificationServer({}).definition(),
                ),
                connectors: [],
                servers: [],
                bindables: [],
                providers: [],
            },
        ];
    }

    /** Write the notification build: its package, its settings and German catalog. */
    static build(crypto: Crypto): Promise<BuildReader> {
        return MemoryBuild.of(
            Package.of(notificationPackage),
            [
                {
                    kind: PACKAGE_DECLARATION,
                    kindPackageId: objectPackage.id,
                    name: PACKAGE_EXPORT,
                    description: describePackage(notificationPackage),
                },
                ...[focus, preferences, summarySchedule].map((declared) => ({
                    kind: "setting",
                    kindPackageId: settingPackage.id,
                    name: declared.name,
                    description: describeSetting(declared),
                })),
            ],
            crypto,
            {
                files: new Map([
                    ["locale/de.json", new TextEncoder().encode(JSON.stringify(germanCatalog))],
                ]),
            },
        );
    }

    /** Define the template of the kit's world, which every test copies. */
    static template() {
        return TestSpace.template(SPACE_OPTIONS, {
            name: "notification",
            value: schema.null(),
            spaces: [
                { id: ids.space, account: ids.account, name: "space" },
                { id: account.Account.home(ids.account), account: ids.account, name: "home" },
                ...PEOPLE.map((name) => ({ home: TestNotification.#email(name) })),
            ],
            write: (space, _directory, spaces) => TestNotification.#write(space, spaces),
        });
    }

    /** Install the packages in the space and each home, and relate the people as guests. */
    static async #write(
        space: TestSpace,
        spaces: ReadonlyMap<Identifier<"space">, TestSpace>,
    ): Promise<null> {
        // install each package in dependency order, as the space's stack does
        const installing = await space.serve();
        for (const installed of PACKAGES) {
            await space.install(installing, installed.build, TestNotification.#install(installed));
        }
        await installing[Symbol.asyncDispose]();

        // install the messages and notifications in each person's home
        for (const home of spaces.values()) {
            if (home.id !== space.id) {
                const serving = await home.serve();
                for (const installed of HOME_PACKAGES) {
                    await home.install(
                        serving,
                        installed.build,
                        TestNotification.#install(installed),
                    );
                }
                await serving[Symbol.asyncDispose]();
            }
        }

        // serve again, as the machine does once installations change, and name the people their homes signed in and relate them as guests
        await using served = await space.serve();
        for (const name of PEOPLE) {
            const person = space.universe.person(TestNotification.#email(name));
            await person.client(accountPackage).user.update({
                id: person.id,
                requestId: RequestId.create(space.universe.environment),
                name,
            });
            await TestNotification.#relate(space, person, "guest");
        }

        // follow the people's profiles in the space before the tests notify them
        await space.settle(served);

        return null;
    }

    /** Give the notifications their push capability, and every other package none. */
    static #install(installed: TestPackage) {
        return { capabilities: installed.handle === notificationPackage ? [PUSH_CAPABILITY] : [] };
    }

    /** Address a person by name. */
    static #email(name: Person): string {
        return `${name}@example.test`;
    }

    /** Read a person as a principal. */
    subject(person: Person): Subject {
        return this.people[person].subject;
    }

    /** Serve a person's home, which must run before anything it does falls due. */
    home(person: Person): Promise<TestHome> {
        // serve each home once
        const known = this.#homes.get(person);
        if (known !== undefined) {
            return known;
        }
        const space = present(this.#hosted.get(person), `the home of ${person}`);
        const serving = (async () => {
            const served = await space.serve();

            return { space, served, caller: space.person(served, this.people[person]) };
        })();
        this.#homes.set(person, serving);

        return serving;
    }

    /** Serve another space the world hosts, such as the account's home, until the kit closes. */
    async serve(id: Identifier<"space">): Promise<void> {
        const space = present(this.#spaces.get(id), `the hosted space ${id}`);
        const served = await space.serve();
        this.#others.push({ space, served, caller: space.person(served, space.owner) });
    }

    /** Read a person's home's selection, which the calls of the home's types spread. */
    homeScope(person: Person): { readonly spaceId: Identifier<"space"> } {
        return { spaceId: present(this.#hosted.get(person), `the home of ${person}`).id };
    }

    /** Call as a person through their client's tokens, in their home for its types. */
    async call<Result>(person: Person, called: Call<Result>): Promise<Result> {
        // find the called procedure on the person's client of the type's package, in the store serving it
        const home = HOME_TYPES.has(called.type.type) ? await this.home(person) : undefined;
        const served = home?.served ?? this.served;
        const caller = home?.caller ?? this.#callers[person];
        const { type, handle } = present(served.type(called.type.type), "the called type");
        const client: unknown = Reflect.get(caller.client(handle), type.name);
        const procedure: unknown = Reflect.get(Object(client), called.method);
        if (typeof procedure !== "function") {
            throw new TypeError(`${type.name} has no method ${called.method}`);
        }

        // name a mutation's request
        const isMutation = type.method(called.method).mutates;
        const named = isMutation ? { requestId: RequestId.create(this.environment) } : {};
        const answered: unknown = await Reflect.apply(procedure, undefined, [
            { ...called.input, ...named },
        ]);

        // read the answer as the method's result, parsed by its output schema
        const [result] = MethodContext.results([type], [called], [answered]);

        return result;
    }

    /** Make a person a member reading the space: its owner relates them as a viewer. */
    async join(person: Person): Promise<void> {
        await TestNotification.#relate(this.space, this.people[person], "viewer");
        await this.settle();
    }

    /** Relate a person to the space at the universe as its owner, as a guest or a viewer. */
    static async #relate(
        space: TestSpace,
        person: TestPerson,
        relation: "guest" | "viewer",
    ): Promise<void> {
        await space.owner.client(objectPackage).relationship.create({
            scope: space.account,
            requestId: RequestId.create(space.universe.environment),
            object: account.space.reference(space.account, space.id),
            relation,
            subject: person.subject,
        });
    }

    /** Set a person's time zone or language on their profile, as they edit it. */
    async contact(
        person: Person,
        profile: { readonly timeZone?: string; readonly locale?: LocaleTag },
    ): Promise<void> {
        const signedIn = this.people[person];
        await this.space.simulation.drive(
            signedIn.client(accountPackage).user.update({
                id: signedIn.id,
                requestId: RequestId.create(this.environment),
                ...profile,
            }),
        );
    }

    /** Set a person's value of a setting in their own scope, as they choose it. */
    async set<Value extends schema.Schema>(
        person: Person,
        declared: Setting<Value>,
        value: schema.Infer<Value>,
    ): Promise<void> {
        const signedIn = this.people[person];
        await this.space.simulation.drive(
            signedIn.client(settingPackage).setting.create({
                scope: signedIn.id,
                packageId: declared.reference.packageId,
                name: declared.reference.name,
                mode: "set",
                value: schema.json().parse(value),
                release: declared.package.version,
                requestId: RequestId.create(this.environment),
            }),
        );
    }

    /** Read the native client a person signed in with, which shows their desktop notifications. */
    client(person: Person): Identifier<"client"> {
        return this.people[person].clientId;
    }

    /** Sign another desktop client of a person in at their account, answering the client. */
    async desktop(person: Person): Promise<Identifier<"client">> {
        const signedIn = this.people[person];
        const created = await this.space.simulation.drive(
            signedIn.client(accountPackage).client.create({
                userId: signedIn.id,
                requestId: RequestId.create(this.environment),
                name: "Desktop",
                kind: "native",
            }),
        );

        return created.id;
    }

    /** Subscribe a person's browser to pushes at a path of the push service. */
    async browser(
        person: Person,
        path: string,
    ): Promise<{
        readonly id: Identifier<"client">;
        readonly url: string;
        readonly browser: Browser;
    }> {
        // create the person's browser client at their account, and store its subscription
        const signedIn = this.people[person];
        const clients = signedIn.client(accountPackage).client;
        const browser = await subscribeBrowser(this.environment);
        const created = await this.space.simulation.drive(
            clients.create({
                userId: signedIn.id,
                requestId: RequestId.create(this.environment),
                name: "Browser",
                kind: "browser",
            }),
        );
        const url = `${PUSH_ORIGIN}${path}`;
        await this.space.simulation.drive(
            clients.subscribe({
                userId: signedIn.id,
                id: created.id,
                requestId: RequestId.create(this.environment),
                push: { url, keys: browser.keys },
            }),
        );

        return { id: created.id, url, browser };
    }

    /** Revoke one of a person's clients at their account, as they sign it out. */
    async revoke(person: Person, id: Identifier<"client">): Promise<void> {
        const signedIn = this.people[person];
        await this.space.simulation.drive(
            signedIn.client(accountPackage).client.revoke({
                userId: signedIn.id,
                id,
                requestId: RequestId.create(this.environment),
            }),
        );
    }

    /** Read the space's activities, by recipient and creation order. */
    async activities() {
        const { table } = activity;

        return await this.served.database
            .select()
            .from(table)
            .where(eq(table.scope, this.scope.spaceId))
            .orderBy(asc(table.recipientId), asc(table.createdAt), asc(table.id));
    }

    /** Read a person's inbox: their home's notifications joined with its activity replicas. */
    async inbox(person: Person): Promise<Inbox> {
        // read the home's notifications and activity replicas
        const { database } = (await this.home(person)).served;
        const notifications = await database.select().from(notification.table);
        const activities = await database.select().from(activity.table);

        return new Inbox(notifications, activities, this.clock.now());
    }

    /** Read a person's messages of a notification, by channel. */
    async messages(person: Person, id: string) {
        const { database } = (await this.home(person)).served;
        const rows = await database.select().from(message.table);

        return rows
            .filter((row) => Notification.from(row) === id)
            .map((row) => ({ to: row.to, status: row.status, reason: row.reason }))
            .toSorted((left, right) => left.to.channel.localeCompare(right.to.channel));
    }

    /** Settle the space's controllers, such as expanding its announcements. */
    settle(): Promise<void> {
        return this.space.settle(this.served);
    }

    /** Settle the space and the homes until none of them commits. */
    async dispatch(): Promise<void> {
        for (let round = 0; round < DISPATCH_ROUNDS; round += 1) {
            // settle each, and stop once a round moved no log
            const before = await this.#positions();
            await this.settle();
            for (const home of await this.#served()) {
                await home.space.settle(home.served);
            }
            if ((await this.#positions()) === before) {
                return;
            }
        }
        throw new Error(`the space and homes kept committing after ${DISPATCH_ROUNDS} rounds`);
    }

    /** Move the world on by some minutes, then dispatch what fell due. */
    wait(minutes: number): Promise<void> {
        return this.reach(this.clock.now() + minutes * 60_000);
    }

    /** Jump the world to a time, running its last moments so the follows confirm. */
    async reach(at: number): Promise<void> {
        // jump to a moment before the time, run the rest, then dispatch what fell due
        const confirming = Math.max(this.clock.now(), at - CONFIRM_MILLISECONDS);
        await this.space.simulation.jump(confirming);
        await this.space.simulation.run({ to: at });
        await this.dispatch();
    }

    /** Read the log positions of the space and the homes, as one comparable text. */
    async #positions(): Promise<string> {
        const stores = [this.served, ...(await this.#served()).map((home) => home.served)];
        const positions = await Promise.all(
            stores.map(async (store) => (await store.database.log.position()).sequence),
        );

        return positions.join(" ");
    }

    /** List the homes and other spaces served so far. */
    async #served(): Promise<TestHome[]> {
        return [...(await Promise.all(this.#homes.values())), ...this.#others];
    }

    /** Stop serving the homes and the space, then close it. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const home of await this.#served()) {
            await home.served[Symbol.asyncDispose]();
        }
        await this.served[Symbol.asyncDispose]();
        await this.space[Symbol.asyncDispose]();
    }
}

/** The packages the space installs in dependency order: messages, notifications, documents. */
export const PACKAGES: readonly TestPackage[] = [
    ...(await TestNotification.packages(testHost().crypto)),
    {
        handle: notesPackage,
        build: await openNotesBuild(testHost().crypto),
        server: notesServer,
        connectors: [],
        servers: [],
        bindables: [],
        providers: [],
    },
];

/** The packages each person's home installs: messages and notifications. */
const HOME_PACKAGES = PACKAGES.filter((each) => each.handle !== notesPackage);

/** The world the kit's tests copy: its packages, shipped servers and builds, and start. */
const SPACE_OPTIONS = { ...TestPackage.options(PACKAGES), start: START };
