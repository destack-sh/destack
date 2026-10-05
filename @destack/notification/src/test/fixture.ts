import { readFile } from "node:fs/promises";
import { anyone } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, device, pushEndpoint, user } from "@destack/account/object";
import { locale } from "@destack/account/setting";
import { journal } from "@destack/audit/stack";
import {
    type Database,
    type DatabaseConnection,
    type Dialect,
    eq,
    Key,
    TABLE,
    type Table,
} from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { memoryBuild } from "@destack/host/test";
import { Catalog, type LocaleTag } from "@destack/locale";
import { mailProvider } from "@destack/mail/provider";
import { MailFixture } from "@destack/mail/test";
import { type PushKeys, pushProvider, Vapid } from "@destack/message/push";
import { message, messageAttempt } from "@destack/message";
import { serveMessages } from "@destack/message/server";
import type { CallableName, CallOutput, ObjectType } from "@destack/object";
import { ObjectServer, ScopeProjector } from "@destack/object/server";
import type { BuildReader } from "@destack/package/manifest";
import { type Identifier, present, schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { setting } from "@destack/setting/object";
import type { Controller } from "@destack/service/control";
import { RequestId } from "@destack/service/request";
import { emptyBuild, reconciliation, subjectContext, testCallKey } from "@destack/service/test";
import { space } from "@destack/space/object";
import {
    type Call,
    type Mutation,
    Replica,
    replicaRow,
    replicaTables,
    Scope,
    Subject,
} from "@destack/sync";
import { onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import { activity, delivery, notification } from "../object/index.ts";
import { serveInbox } from "../server/index.ts";
import { inboxService } from "../service/index.ts";
import { HomeInbox } from "./inbox.ts";

/** The inbox's build, shipping its own German catalog. */
export const INBOX_BUILD: BuildReader = await memoryBuild(
    inboxService.package,
    {},
    new Map([
        [
            "locale/de.json",
            new Uint8Array(await readFile(`${import.meta.dirname}/../../locale/de.json`)),
        ],
    ]),
);

/** The inbox's own catalogs, read from its build as its workload reads them. */
export const INBOX_CATALOGS: readonly Catalog[] = await Catalog.read(INBOX_BUILD);

/** The installation of the app keeping the activities. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-019f5530-8000-7000-8000-0000000000f1");

/** The controllers a dispatch runs, by name: the objects', and the outbox sending changes of copied rows. */
const DISPATCHED = new Set(["notification", "delivery", "announcement", "message", "sends"]);

/** The objects living in the people's home rather than the app's space. */
const HOME_OBJECTS = new Set(["notification", "delivery"]);

/** The rounds a dispatch takes at most: planning, sending, settling and batches settle well within them. */
const DISPATCH_ROUNDS = 20;

/** The status a push service answers a push it accepted with (RFC 8030 5). */
const CREATED = 201;

/** The status a push service answers a push to an endpoint it forgot with (RFC 8030 7.3). */
const GONE = 410;

/** Stream no copies, as a cell with nothing to copy. */
async function* idle() {}

/** How refused messages retry in a scenario: at once, twice. */
const RETRY = { initialInterval: 0, maximumInterval: 0, maximumAttempts: 2, backoffCoefficient: 1 };

/** What a NotificationFixture serves: an app's objects in a space, and the people it notifies by name. */
export interface NotificationFixtureOptions<Name extends string> {
    /** The dialect of the database, SQLite by default. */
    readonly dialect?: Dialect;
    /** The app's package and service recording the calls. */
    readonly origin: ObjectServer["origin"];
    /** The app's objects in the space, with the activities it serves. */
    readonly objects: Readonly<Record<string, ObjectType>>;
    /** The database with the app's objects' tables and the fixture's own, copying the fixture's copies. */
    readonly database: Database;
    /** The people the scenario acts as and notifies, by name. */
    readonly people: Readonly<Record<Name, Subject>>;
    /** The person calls act as first. */
    readonly actor: NoInfer<Name>;
    /** The app's build, an empty one of its package by default. */
    readonly build?: BuildReader;
    /** The scenario's start in Unix milliseconds, now by default. */
    readonly at?: number;
}

/** An app's space and its people's home in one database, delivering notifications over in-memory transports at a clock the scenario moves. */
export class NotificationFixture<Name extends string> implements AsyncDisposable {
    /** The people the scenario acts as and notifies, by name. */
    readonly people: Readonly<Record<Name, Subject>>;
    /** The server of the app's objects in the space and the inbox's in the home. */
    readonly server: ObjectServer;
    /** The app's space. */
    readonly spaceId: Identifier<"space">;
    /** The people's home. */
    readonly homeId: Identifier<"space">;
    /** The push service the recipients' browsers subscribe at. */
    readonly pushes: PushService;
    /** The mail transport. */
    readonly mails: MailFixture;
    /** The push endpoints the inbox forgot through their kind. */
    readonly forgotten: string[] = [];
    /** The languages people set, by subject key, as the cell's directory reads them. */
    readonly #locales = new Map<string, LocaleTag>();
    /** The test database. */
    readonly #test: TestDatabase;
    /** The controllers a dispatch reconciles. */
    readonly #controllers: readonly Controller[];
    /** End the projection and the inboxes. */
    readonly #following = new AbortController();
    /** The work closed before the database: the projection and the inboxes. */
    readonly #followers: (() => Promise<void>)[] = [];
    /** The closing of the database, absent while open. */
    #closed: Promise<void> | undefined;
    /** The scenario's time, in Unix milliseconds. */
    #now: number;
    /** The person calls act as. */
    #actor: Name;

    /** The tables the fixture keeps beside the app's: the inbox's, the messages', the people's users, desktops and settings, the journal and the copies' bookkeeping. */
    static readonly tables: readonly Table[] = [
        journal,
        ...replicaTables,
        ...[user, device, setting, notification, delivery, message, messageAttempt].flatMap(
            (object) => object.tables,
        ),
    ];

    /** The tables the fixture keeps as copies, whose changes it receives as their home: the push endpoints. */
    static readonly copies: readonly Table[] = [pushEndpoint.table];

    /** Keep the served space and home. */
    private constructor(
        test: TestDatabase,
        server: ObjectServer,
        spaces: { readonly spaceId: Identifier<"space">; readonly homeId: Identifier<"space"> },
        transports: { readonly pushes: PushService; readonly mails: MailFixture },
        options: NotificationFixtureOptions<Name> & { readonly at: number },
    ) {
        // keep the database, the server and the transports
        this.#test = test;
        this.server = server;
        this.spaceId = spaces.spaceId;
        this.homeId = spaces.homeId;
        this.pushes = transports.pushes;
        this.mails = transports.mails;

        // act as the first person at the scenario's start
        this.people = options.people;
        this.#actor = options.actor;
        this.#now = options.at;
        this.#controllers = server.controllers().filter((each) => DISPATCHED.has(each.name));
    }

    /** The database with the space and the home. */
    get database(): DatabaseConnection {
        return this.#test.database;
    }

    /** Serve an app's objects in a new space and the inbox in its people's home, closed after the test. */
    static async open<Name extends string>(
        options: NotificationFixtureOptions<Name>,
    ): Promise<NotificationFixture<Name>> {
        // migrate the database of the app's objects and the inbox
        const { dialect = "sqlite", at = Date.now() } = options;
        const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
            "sign",
            "verify",
        ]);
        const transports = { pushes: new PushService(), mails: new MailFixture() };
        const providers = [
            mailProvider(transports.mails),
            pushProvider(new Vapid(keys, "https://destack.app"), transports.pushes.fetch),
        ];
        const objects = {
            ...options.objects,
            user,
            ...serveInbox({ catalogs: INBOX_CATALOGS }),
            ...serveMessages(providers, RETRY),
        };
        const test = await TestDatabase.create(dialect, options.database, { isMigrated: true });

        // keep the space and the people's home
        const { database } = test;
        const spaceId = await NotificationFixture.openSpace(database, at);
        const homeId = await NotificationFixture.openSpace(database, at);

        // serve them as the app's installation, sending changes of copied push endpoints to their home
        const fixture: NotificationFixture<Name> = new NotificationFixture(
            test,
            new ObjectServer({
                objects,
                policies: [pushEndpoint],
                database,
                clock: () => fixture.now(),
                callKey: testCallKey,
                origin: options.origin,
                subscriber: {
                    uplink: {
                        stream: idle,
                        receive: (mutation) => fixture.#receive(mutation),
                    },
                    subscriptions: async () => [],
                },
                installation: {
                    id: INSTALLATION,
                    scope: spaceId,
                    build: options.build ?? (await emptyBuild(options.origin.package)),
                    publisher: {
                        stream: idle,
                        receive: (mutation) => fixture.#receive(mutation),
                    },
                    publisherAt: () => ({
                        stream: idle,
                        receive: (mutation: Mutation) => fixture.#receive(mutation),
                    }),
                    directory: {
                        isHome: async (_subject, home) => home === homeId,
                        locale: async (subject) => fixture.#locales.get(subject),
                        address: async () => {},
                    },
                },
            }),
            { spaceId, homeId },
            transports,
            { ...options, at },
        );
        onTestFinished(() => fixture.close());

        // project the space's activities into the home
        fixture.#follow();

        return fixture;
    }

    /** Open a new space below a new account, readable by anyone through a member role, returning its identifier. */
    static async openSpace(database: DatabaseConnection, at: number): Promise<Identifier<"space">> {
        // record the account and the space as copies of their home's access has them
        const accountId = schema.identifier("account").parse(`account-${v7()}`);
        const spaceId = schema.identifier("space").parse(`space-${v7()}`);
        const reference = space.reference(accountId, spaceId);
        const accessCopies = new AccessFixture(database);
        await accessCopies.copyScope(account.reference(Scope.universe.id, accountId));
        await accessCopies.copyScope(reference);

        // define a role reading the space and bind it to anyone
        await accessCopies.copyRole(
            reference,
            {
                name: "member",
                description: "Reads the space",
                permissions: [space.permission("read")],
            },
            anyone.reference("*", "*"),
            at,
        );

        return spaceId;
    }

    /** Close the projection and the inboxes, then the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Close the projection and the inboxes, then the database, once. */
    close(): Promise<void> {
        this.#closed ??= (async () => {
            this.#following.abort();
            for (const follower of this.#followers) {
                await follower();
            }
            await this.#test.close();
        })();

        return this.#closed;
    }

    /** Read the scenario's time. */
    now(): number {
        return this.#now;
    }

    /** Move the clock by some minutes. */
    wait(minutes: number): void {
        this.#now += minutes * 60_000;
    }

    /** Act as another person. */
    as(person: Name): void {
        this.#actor = person;
    }

    /** Call a method of an object type as the current person in its space, replayable by request. */
    call<Type extends ObjectType, Method extends CallableName<Type>>(
        object: Type,
        name: Method,
        input: object,
    ): Promise<CallOutput<Type, Method>> {
        const scope = HOME_OBJECTS.has(object.name) ? this.homeId : this.spaceId;
        const request = { spaceId: scope, requestId: RequestId.create(), ...input };

        return this.server.call(object, name, request, this.#context(scope));
    }

    /** Execute one mutation of several calls as the current person. */
    mutate(calls: readonly Call[]) {
        return this.server.mutate(
            { id: RequestId.create(), calls: [...calls] },
            this.#context(this.spaceId),
        );
    }

    /** Follow a person's inbox in the home until the fixture closes. */
    inbox(person: Name): HomeInbox {
        const inbox = new HomeInbox(this.server, this.homeId, this.people[person]);
        this.#followers.push(() => inbox.close());

        return inbox;
    }

    /** Copy a person who joined the space, as its cell keeps the users it follows. */
    async join(person: Name): Promise<void> {
        // copy the user
        const id = schema.identifier("user").parse(this.people[person].id);
        await this.database
            .insert(user.table)
            .values({
                id,
                scope: Scope.universe.id,
                name: person,
                createdAt: this.#now,
                updatedAt: this.#now,
            })
            .onConflictDoNothing();

        // record it as a copy of the space's replica
        await this.database.insert(replicaRow).values({
            name: this.spaceId,
            scope: Scope.universe.id,
            table: user.table[TABLE].sqlName,
            key: Key.name(user.table, { id }),
            concealed: [],
        });
    }

    /** Give a person a verified email, a time zone or a language in their scope, as their user's copy and settings hold them. */
    async reach(
        person: Name,
        profile: {
            readonly email?: string;
            readonly timeZone?: string;
            readonly locale?: LocaleTag;
        },
    ): Promise<void> {
        // keep the verified email and the time zone on the person's user
        const id = schema.identifier("user").parse(this.people[person].id);
        const fields = {
            ...(profile.email === undefined ? {} : { email: profile.email, emailVerified: true }),
            ...(profile.timeZone === undefined ? {} : { timeZone: profile.timeZone }),
        };
        await this.database
            .insert(user.table)
            .values({
                id,
                scope: Scope.universe.id,
                name: person,
                createdAt: this.#now,
                updatedAt: this.#now,
                ...fields,
            })
            .onConflictDoUpdate({
                target: user.table.id,
                set: { ...fields, updatedAt: this.#now },
            });

        // set the language in the person's scope, which the directory reads too
        if (profile.locale !== undefined) {
            await this.set(person, locale, profile.locale);
            this.#locales.set(Subject.key(this.people[person]), profile.locale);
        }
    }

    /** Set a person's value of a setting, in their scope or refined to a space. */
    async set<Value extends schema.Schema>(
        person: Name,
        declared: Setting<Value>,
        value: schema.Infer<Value>,
        refinement: { readonly space?: string } = {},
    ): Promise<void> {
        const subject = this.people[person];
        const key = Subject.key(subject);
        await this.database.insert(setting.table).values({
            id: schema.identifier("setting").parse(`setting-${v7()}`),
            createdAt: this.#now,
            createdBy: key,
            updatedAt: this.#now,
            updatedBy: key,
            revision: 1,
            tags: {},
            scope: subject.id,
            packageId: declared.reference.packageId,
            name: declared.reference.name,
            space:
                refinement.space === undefined
                    ? null
                    : schema.identifier("space").parse(refinement.space),
            mode: "set",
            value: schema.json().parse(value),
            release: declared.package.version,
        });
    }

    /** Sign a person's desktop in, or revoke it. */
    async desktop(person: Name, id: Identifier<"device">, isRevoked = false): Promise<void> {
        const scope = schema.identifier("user").parse(this.people[person].id);
        const revokedAt = isRevoked ? this.#now : null;
        await this.database
            .insert(device.table)
            .values({
                id,
                scope,
                kind: "desktop",
                name: "Desktop",
                createdAt: this.#now,
                updatedAt: this.#now,
                revokedAt,
            })
            .onConflictDoUpdate({ target: device.table.id, set: { revokedAt } });
    }

    /** Subscribe a person's browser to pushes at an endpoint, as their scope's copy keeps it. */
    async subscribe(
        person: Name,
        endpoint: {
            readonly id: Identifier<"push-endpoint">;
            readonly url: string;
            readonly keys: PushKeys;
        },
    ): Promise<void> {
        await this.database.insert(pushEndpoint.table).values({
            ...endpoint,
            scope: schema.identifier("user").parse(this.people[person].id),
            name: "Browser",
            createdAt: this.#now,
            updatedAt: this.#now,
        });
    }

    /** Project the space's activities and reconcile every key the controllers list until a round changes nothing. */
    async dispatch(): Promise<void> {
        const { database } = this;
        for (let round = 0; round < DISPATCH_ROUNDS; round++) {
            // wait for the home's projection to catch up with the space
            const position = await database.log.position();
            if (
                !(await Replica.reach(database, this.spaceId, position, AbortSignal.timeout(5000)))
            ) {
                throw new TypeError("the home's projection never reached the space");
            }

            // reconcile every listed key
            const before = (await database.log.position()).sequence;
            for (const controller of this.#controllers) {
                for (const key of await controller.list()) {
                    await controller.reconcile(key, reconciliation(new AbortController().signal));
                }
            }

            // stop once a round changes nothing
            if ((await database.log.position()).sequence === before) {
                return;
            }
        }
        throw new TypeError("the notification controllers keep changing the space");
    }

    /** Read the space's activities as the system stores them, by recipient and creation order. */
    async activities() {
        const table = activity.table;
        const rows = await this.database.select().from(table).where(eq(table.scope, this.spaceId));

        return rows.toSorted(
            (left, right) =>
                left.recipient.localeCompare(right.recipient) ||
                left.createdAt - right.createdAt ||
                left.id.localeCompare(right.id),
        );
    }

    /** Read the home's notifications as the system stores them, by recipient and creation order. */
    async notifications() {
        const table = notification.table;
        const rows = await this.database.select().from(table).where(eq(table.scope, this.homeId));

        return rows.toSorted(
            (left, right) =>
                left.recipient.localeCompare(right.recipient) ||
                left.createdAt - right.createdAt ||
                left.id.localeCompare(right.id),
        );
    }

    /** Read the deliveries of a notification, by channel and endpoint. */
    async deliveries(id: string) {
        const table = delivery.table;
        const rows = await this.database
            .select()
            .from(table)
            .where(eq(table.parentId, schema.identifier("notification").parse(id)));

        return rows
            .map((row) => ({
                channel: row.channel,
                endpoint: row.endpoint,
                device: row.device,
                state: row.state,
                dueAt: row.dueAt,
                isSummarized: row.isSummarized,
                reason: row.reason,
            }))
            .toSorted((left, right) => left.channel.localeCompare(right.channel));
    }

    /** Authenticate the current person in a space at the scenario's time. */
    #context(scope: string) {
        return subjectContext(this.people[this.#actor], scope, {
            signal: this.#following.signal,
            clock: () => this.#now,
        });
    }

    /** Receive the inbox's changes of copied push endpoints as the account service keeping them: forget each deleted one. */
    async #receive(mutation: Mutation): Promise<void> {
        for (const call of mutation.calls) {
            // refuse any other change than a deletion of a push endpoint
            if (call.method !== `${pushEndpoint.name}.delete`) {
                throw new TypeError(`the fixture receives no ${call.method}`);
            }

            // forget it in its copy
            const id = schema.identifier("push-endpoint").parse(call.input["id"]);
            await this.database.delete(pushEndpoint.table).where(eq(pushEndpoint.table.id, id));
            this.forgotten.push(id);
        }
    }

    /** Project the space's activities into the home as the people's notifications until the fixture closes. */
    #follow(): void {
        // subscribe the home to every person's activities in the space
        const projection = new Replica({
            name: this.homeId,
            scope: this.spaceId,
            tables: [],
            isRelayed: false,
            projectors: [
                new ScopeProjector(
                    notification,
                    present(notification.projected, "the notifications' projection"),
                    this.homeId,
                ),
            ],
        });
        const subscribed = this.server.source.projectionShape.subscription({
            name: this.homeId,
            scope: this.spaceId,
            below: this.homeId,
            parameters: {
                installation: INSTALLATION,
                packageId: activity.policy.definition.packageId,
                type: activity.name,
                to: "recipient",
                recipients: Object.values<Subject>(this.people).map((person) =>
                    Subject.key(person),
                ),
            },
        });

        // follow the projection until the fixture closes
        const followed = projection.follow(
            this.database,
            ({ after }, signal) =>
                this.server.source.project(
                    { ...subscribed, ...(after === undefined ? {} : { after }) },
                    signal,
                ),
            this.#following.signal,
        );
        this.#followers.push(() => followed);
    }
}

/** One push a push service received, with the headers RFC 8030 reads and its encrypted body. */
export interface ReceivedPush {
    /** The endpoint's URL. */
    readonly url: string;
    /** The urgency header. */
    readonly urgency: string;
    /** The lifetime in seconds. */
    readonly ttl: number;
    /** The topic header. */
    readonly topic: string;
    /** The encrypted body. */
    readonly body: Uint8Array<ArrayBuffer>;
}

/** A push service keeping the pushes posted to it, answering each as created, or as gone for a forgotten endpoint. */
export class PushService {
    /** The pushes received, in order. */
    readonly requests: ReceivedPush[] = [];
    /** The endpoint URLs the service forgot. */
    readonly gone = new Set<string>();

    /** Receive one push as fetch posts it. */
    readonly fetch = async (url: string, initialize: RequestInit): Promise<Response> => {
        // keep the push with its headers
        const headers = new Headers(initialize.headers);
        if (!(initialize.body instanceof Uint8Array)) {
            throw new TypeError("a push posts bytes");
        }
        this.requests.push({
            url,
            urgency: present(headers.get("Urgency"), "the urgency"),
            ttl: Number(headers.get("TTL")),
            topic: present(headers.get("Topic"), "the topic"),
            body: new Uint8Array(initialize.body),
        });

        return new Response(null, { status: this.gone.has(url) ? GONE : CREATED });
    };
}
