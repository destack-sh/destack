import { readFile } from "node:fs/promises";
import { anyone, type PermissionReference, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, client, key, user, space, Account } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
import {
    type Database,
    type DatabaseConnection,
    type Dialect,
    eq,
    Key,
    Snapshot,
    TABLE,
    type Table,
} from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Catalog, type LocaleTag } from "@destack/locale";
import { Derivation } from "@destack/identity";
import { emailProvider } from "@destack/message/email";
import { MailFixture } from "@destack/message/test";
import { type PushKeys, pushProvider, Vapid } from "@destack/message/push";
import { message, messageAttempt, MessageKey } from "@destack/message";
import { serveMessages } from "@destack/message/server";
import {
    Call,
    type CallableName,
    type CallInput,
    type CallOutput,
    type ObjectCall,
    type ObjectType,
} from "@destack/object";
import { ObjectStore, ScopeProjector } from "@destack/object/server";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { PackageId } from "@destack/package";
import { Clock, type Identifier, present, schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { setting } from "@destack/setting/object";
import type { Controller } from "@destack/service/control";
import { RequestId } from "@destack/service/request";
import { reconciliation, subjectContext, testCallKey } from "@destack/service/test";

import { type Mutation, Replica, replicaRow, replicaTables, Scope, Subject } from "@destack/sync";
import { onTestFinished, TestClock } from "@destack/test";
import { v7 } from "uuid";
import { activity, announcement, delivery, notification } from "../object/index.ts";
import { serveInbox } from "../server/index.ts";
import { inboxService } from "../service/index.ts";
import { HomeInbox } from "./inbox.ts";

/** The inbox's build, shipping its own German catalog. */
export const INBOX_BUILD: BuildReader = await MemoryBuild.write(
    new Map([
        [
            "locale/de.json",
            new Uint8Array(await readFile(`${import.meta.dirname}/../../locale/de.json`)),
        ],
    ]),
    { package: inboxService.package },
);

/** The inbox's own catalogs, read from its build as the machine reads them. */
export const INBOX_CATALOGS: readonly Catalog[] = await Catalog.read(INBOX_BUILD);

/** The time zone and locale each person of a scenario signs up with. */
const SIGN_UP = { timeZone: "UTC", locale: "en" };

/** The installation of the app keeping the activities. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-019f5530-8000-7000-8000-0000000000f1");

/** The permissions the fixture's installation has in each space and home. */
const INSTALLATION_PERMISSIONS = [
    notification.permission("read"),
    notification.permission("deliver"),
    delivery.permission("deliver"),
    announcement.permission("expand"),
    message.permission("send"),
    message.permission("read"),
    message.permission("control"),
    message.permission("erase"),
    messageAttempt.permission("record"),
];

/** The controllers a dispatch runs, by name. */
const DISPATCHED = new Set(["notification", "delivery", "announcement", "message", "sends"]);

/** The objects living in the people's home rather than the app's space. */
const HOME_OBJECTS = new Set(["notification", "delivery"]);

/** The most rounds a dispatch takes. */
const DISPATCH_ROUNDS = 20;

/** The longest a dispatch waits for the home's projection, in milliseconds. */
const PROJECTION_TIMEOUT = 5000;

/** The status a push service answers a push it accepted with (RFC 8030 5). */
const CREATED = 201;

/** The status a push service answers a push to an endpoint it forgot with (RFC 8030 7.3). */
const GONE = 410;

/** How refused messages retry in a scenario: at once, twice. */
const RETRY = { initialInterval: 0, maximumInterval: 0, maximumAttempts: 2, backoffCoefficient: 1 };

/** Stream no copies, as a machine with nothing to copy. */
async function* idle() {}

/** Answer an installation's calls to an address with a refusal. */
export function unfetched(address: string) {
    return {
        url: address,
        fetch: () => Promise.reject(new Error("the test installation fetches no address")),
    };
}

/** What a NotificationFixture serves: an app's objects in a space, and the people it notifies. */
export interface NotificationFixtureOptions<Name extends string> {
    /** The dialect of the database, SQLite by default. */
    readonly dialect?: Dialect;
    /** The app's package and service recording the calls. */
    readonly origin: ObjectStore["origin"];
    /** The app's objects in the space, with the activities it serves. */
    readonly objects: readonly ObjectType[];
    /** The database with the app's objects' tables and the fixture's tables. */
    readonly database: Database;
    /** The people the scenario acts as and notifies, by name. */
    readonly people: Readonly<Record<Name, Subject>>;
    /** The person calls act as first. */
    readonly actor: NoInfer<Name>;
    /** The app's build, an empty one of its package by default. */
    readonly build?: BuildReader;
    /** The scenario's start in Unix milliseconds, now by default. */
    readonly at?: number;
    /** The permissions the app's workloads request on its own objects. */
    readonly requested?: readonly PermissionReference[];
}

/** An app's space and its people's home in one database. */
export class NotificationFixture<Name extends string> implements AsyncDisposable {
    /** The people the scenario acts as and notifies, by name. */
    readonly people: Readonly<Record<Name, Subject>>;
    /** The store of the app's objects in the space and the inbox's in the home. */
    readonly store: ObjectStore;
    /** The app's space. */
    readonly spaceId: Identifier<"space">;
    /** The people's home. */
    readonly homeId: Identifier<"space">;
    /** The push service the recipients' browsers subscribe at. */
    readonly pushes: PushService;
    /** The mail transport. */
    readonly mails: MailFixture;
    /** The browser clients whose push subscriptions the inbox forgot at the account service. */
    readonly forgotten: string[] = [];
    /** The scenario's clock, which the stores and the fixture's calls read. */
    readonly clock: TestClock;
    /** The languages people read, by subject key. */
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
    /** The person calls act as. */
    #actor: Name;

    /** The tables the fixture keeps beside the app's. */
    static readonly tables: readonly Table[] = [
        journal,
        ...replicaTables,
        ...[user, setting, notification, delivery, message, messageAttempt].flatMap(
            (object) => object.tables,
        ),
    ];

    /** The tables the fixture keeps as copies, receiving their changes as their home. */
    static readonly copies: readonly Table[] = [client.table, key.table];

    /** Keep the served space and home. */
    private constructor(
        test: TestDatabase,
        store: ObjectStore,
        spaces: { readonly spaceId: Identifier<"space">; readonly homeId: Identifier<"space"> },
        transports: { readonly pushes: PushService; readonly mails: MailFixture },
        options: NotificationFixtureOptions<Name> & { readonly clock: TestClock },
    ) {
        // keep the database, the store and the transports
        this.#test = test;
        this.store = store;
        this.spaceId = spaces.spaceId;
        this.homeId = spaces.homeId;
        this.pushes = transports.pushes;
        this.mails = transports.mails;

        // act as the first person at the scenario's start
        this.people = options.people;
        this.#actor = options.actor;
        this.clock = options.clock;
        this.#controllers = store.controllers().filter((each) => DISPATCHED.has(each.name));

        // read each person in the locale they signed up with
        for (const person of Object.values<Subject>(options.people)) {
            this.#locales.set(Subject.key(person), SIGN_UP.locale);
        }
    }

    /** The database with the space and the home. */
    get database(): DatabaseConnection {
        return this.#test.database;
    }

    /** Serve an app's objects in a new space and the inbox in its people's home. */
    static async open<Name extends string>(
        options: NotificationFixtureOptions<Name>,
    ): Promise<NotificationFixture<Name>> {
        // migrate the database of the app's objects and the inbox
        const { dialect = "sqlite", at = Date.now() } = options;
        const clock = new TestClock(at);
        const transports = { pushes: new PushService(), mails: new MailFixture() };
        const objects = await NotificationFixture.#objects(options.objects, transports);
        const test = await TestDatabase.create(dialect, options.database, { isMigrated: true });

        // keep the space and the people's home, granting the installation in both
        const { database } = test;
        const spaceId = await NotificationFixture.openSpace(database, at);
        const homeId = await NotificationFixture.openSpace(database, at);
        const installation = principal.installation.reference(spaceId, INSTALLATION);
        const requested = [...INSTALLATION_PERMISSIONS, ...(options.requested ?? [])];
        for (const granted of [spaceId, homeId]) {
            await NotificationFixture.grant(database, granted, installation, requested, at);
        }

        // copy each person's user
        for (const [name, person] of Object.entries<Subject>(options.people)) {
            await database.insert(user.table).values(NotificationFixture.#user(person, name, at));
        }

        // serve them as the app's installation, sending changes of copied clients to their home
        const uplink = {
            stream: idle,
            receive: (mutation: Mutation) => fixture.#receive(mutation),
        };
        const fixture: NotificationFixture<Name> = new NotificationFixture(
            test,
            new ObjectStore({
                principal: installation,
                objects,
                policies: [client],
                database,
                clock,
                callKey: testCallKey,
                origin: options.origin,
                subscriber: { uplink, subscriptions: async () => [] },
                installation: {
                    id: INSTALLATION,
                    scope: spaceId,
                    build:
                        options.build ??
                        (await MemoryBuild.write(new Map(), { package: options.origin.package })),
                    publisher: uplink,
                    settings: {},
                    publisherAt: () => uplink,
                    at: unfetched,
                    directory: {
                        isHome: async (_subject, home) => home === homeId,
                        locale: async (subject) =>
                            present(fixture.#locales.get(subject), `the locale of ${subject}`),
                        address: async () => {},
                        messageKey: async (home) => {
                            throw new Error(`the source space sends no messages, not to ${home}`);
                        },
                    },
                },
            }),
            { spaceId, homeId },
            transports,
            { ...options, clock },
        );
        onTestFinished(() => fixture.close());

        // project the space's activities into the home
        fixture.#follow();

        return fixture;
    }

    /** Describe a person's user row as they sign up. */
    static #user(person: Subject, name: string, at: number) {
        return {
            id: schema.identifier("user").parse(person.id),
            scope: Scope.universe.id,
            name,
            ...SIGN_UP,
            createdAt: at,
            updatedAt: at,
        };
    }

    /** Serve the app's objects beside the users, the inbox and the messages. */
    static async #objects(
        served: readonly ObjectType[],
        transports: { readonly pushes: PushService; readonly mails: MailFixture },
    ) {
        // send mail and pushes through the fixtures' transports
        const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
            "sign",
            "verify",
        ]);
        const root = Derivation.of(
            await Derivation.root(crypto.getRandomValues(new Uint8Array(32))),
        );
        const providers = {
            email: emailProvider(transports.mails),
            push: pushProvider(
                async () => new Vapid(keys, "https://destack.app"),
                transports.pushes.fetch,
            ),
        };

        return [
            ...served,
            user,
            ...serveInbox({
                catalogs: INBOX_CATALOGS,
                key: async () => (await MessageKey.derive(root)).key,
            }),
            ...serveMessages({ providers, retry: RETRY, deriver: async () => root }),
        ];
    }

    /** Open a new space below a new account, and return its identifier. */
    static async openSpace(database: DatabaseConnection, at: number): Promise<Identifier<"space">> {
        // record the account and the space as copies of their home's access has them
        const accountId = schema.identifier("account").parse(`account-${v7()}`);
        const spaceId = Account.home(accountId);
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

    /** Grant an installation in a space what its workloads request there, as approving it does. */
    static async grant(
        database: DatabaseConnection,
        spaceId: string,
        installation: Subject,
        permissions: readonly PermissionReference[],
        at: number,
    ): Promise<void> {
        await new AccessFixture(database).copyRole(
            await Scope.object(Snapshot.live(database), spaceId),
            {
                name: `installation/${installation.id}`,
                description: "What the installation requests in this space",
                permissions: [...permissions],
            },
            installation,
            at,
        );
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

    /** Move the clock by some minutes. */
    wait(minutes: number): void {
        this.clock.advance(minutes * 60_000);
    }

    /** Act as another person. */
    as(person: Name): void {
        this.#actor = person;
    }

    /** Call a method of an object type as the current person in its space. */
    call<Type extends ObjectType, Method extends CallableName<Type>>(
        object: Type,
        name: Method,
        input: NoInfer<CallInput<Type, Method>>,
    ): Promise<CallOutput<Type, Method>>;
    /**
     * Call a method of an object type as the current person in its space.
     *
     * @construct the result is the method's output, which CallOutput reads from the same method.
     */
    call(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): Promise<unknown> {
        // read through a query, or write through a one-call mutation, in the object's space
        const scope = HOME_OBJECTS.has(object.name) ? this.homeId : this.spaceId;
        const call: ObjectCall = object.call(name, { spaceId: scope, ...input });
        const caller = this.#context(scope).caller();

        return object.method(name).mutates
            ? this.store
                  .mutate(caller, { id: RequestId.create(this.clock.now()), calls: [call] })
                  .then(([result]) => result)
            : this.store.query(caller, call);
    }

    /** Follow a person's inbox in the home until the fixture closes. */
    inbox(person: Name): HomeInbox {
        const inbox = new HomeInbox(this.store, this.homeId, this.people[person]);
        this.#followers.push(() => inbox.close());

        return inbox;
    }

    /** Copy a person who joined the space, as the space keeps the users it follows. */
    async join(person: Name): Promise<void> {
        // copy the user
        const row = NotificationFixture.#user(this.people[person], person, this.clock.now());
        await this.database.insert(user.table).values(row).onConflictDoNothing();

        // record it as a copy of the space's replica
        await this.database.insert(replicaRow).values({
            name: this.spaceId,
            scope: Scope.universe.id,
            table: user.table[TABLE].sqlName,
            key: Key.name(user.table, { id: row.id }),
            concealed: [],
        });
    }

    /** Give a person a verified email, a time zone or a language. */
    async contact(
        person: Name,
        profile: {
            readonly email?: string;
            readonly timeZone?: string;
            readonly locale?: LocaleTag;
        },
    ): Promise<void> {
        // keep the verified email, the time zone and the locale on the person's user
        const row = NotificationFixture.#user(this.people[person], person, this.clock.now());
        const fields = {
            ...(profile.email === undefined ? {} : { email: profile.email, emailVerified: true }),
            ...(profile.timeZone === undefined ? {} : { timeZone: profile.timeZone }),
            ...(profile.locale === undefined ? {} : { locale: profile.locale }),
        };
        await this.database
            .insert(user.table)
            .values({ ...row, ...fields })
            .onConflictDoUpdate({
                target: user.table.id,
                set: { ...fields, updatedAt: this.clock.now() },
            });

        // keep the language where the directory reads it too
        if (profile.locale !== undefined) {
            this.#locales.set(Subject.key(this.people[person]), profile.locale);
        }
    }

    /** Set a person's value of a setting in their scope. */
    async set<Value extends schema.Schema>(
        person: Name,
        declared: Setting<Value>,
        value: schema.Infer<Value>,
    ): Promise<void> {
        const subject = this.people[person];
        const caller = Subject.key(subject);
        await this.database.insert(setting.table).values({
            id: schema.identifier("setting").parse(`setting-${v7()}`),
            createdAt: this.clock.now(),
            createdBy: caller,
            updatedAt: this.clock.now(),
            updatedBy: caller,
            revision: 1,
            tags: {},
            scope: subject.id,
            packageId: declared.reference.packageId,
            name: declared.reference.name,
            space: null,
            mode: "set",
            value: schema.json().parse(value),
            release: declared.package.version,
        });
    }

    /** Sign a person's desktop client in, or revoke it. */
    async desktop(person: Name, id: Identifier<"client">, isRevoked = false): Promise<void> {
        const scope = schema.identifier("user").parse(this.people[person].id);
        const revokedAt = isRevoked ? this.clock.now() : null;
        await this.database
            .insert(client.table)
            .values({
                id,
                scope,
                kind: "native",
                name: "Desktop",
                createdAt: this.clock.now(),
                updatedAt: this.clock.now(),
                revokedAt,
            })
            .onConflictDoUpdate({ target: client.table.id, set: { revokedAt } });
    }

    /** Subscribe a person's browser client to pushes at a push service URL. */
    async subscribe(
        person: Name,
        browser: {
            readonly id: Identifier<"client">;
            readonly url: string;
            readonly keys: PushKeys;
        },
    ): Promise<void> {
        await this.database.insert(client.table).values({
            id: browser.id,
            scope: schema.identifier("user").parse(this.people[person].id),
            kind: "browser",
            name: "Browser",
            push: { url: browser.url, keys: browser.keys },
            createdAt: this.clock.now(),
            updatedAt: this.clock.now(),
        });
    }

    /** Project the space's activities and reconcile every key the controllers list. */
    async dispatch(): Promise<void> {
        const { database } = this;
        for (let round = 0; round < DISPATCH_ROUNDS; round++) {
            // wait for the home's projection to reach the space's latest change it did not replicate
            const position = await database.log.position(await database.log.epoch());
            if (
                !(await Replica.wait(
                    database,
                    this.spaceId,
                    position,
                    AbortSignal.timeout(PROJECTION_TIMEOUT),
                ))
            ) {
                throw new TypeError("the home's projection never caught up with the space");
            }

            // reconcile every listed key
            const before = (await database.log.position()).sequence;
            for (const controller of this.#controllers) {
                for (const listed of await controller.list()) {
                    await controller.reconcile(
                        listed,
                        reconciliation(new AbortController().signal),
                    );
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

        return rows.toSorted((left, right) => NotificationFixture.#compare(left, right));
    }

    /** Read the home's notifications as the system stores them, by recipient and creation order. */
    async notifications() {
        const table = notification.table;
        const rows = await this.database.select().from(table).where(eq(table.scope, this.homeId));

        return rows.toSorted((left, right) => NotificationFixture.#compare(left, right));
    }

    /** Order rows by recipient, then creation, then identifier. */
    static #compare(
        left: { readonly recipient: string; readonly createdAt: number; readonly id: string },
        right: { readonly recipient: string; readonly createdAt: number; readonly id: string },
    ): number {
        return (
            left.recipient.localeCompare(right.recipient) ||
            left.createdAt - right.createdAt ||
            left.id.localeCompare(right.id)
        );
    }

    /** Read the deliveries of a notification, by channel and client. */
    async deliveries(id: string) {
        const table = delivery.table;
        const rows = await this.database
            .select()
            .from(table)
            .where(eq(table.parentId, schema.identifier("notification").parse(id)));

        return rows
            .map((row) => ({
                channel: row.channel,
                client: row.client,
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
            clock: this.clock,
        });
    }

    /** Receive the inbox's changes of copied clients as the account service that keeps them. */
    async #receive(mutation: Mutation): Promise<void> {
        for (const call of mutation.calls) {
            // refuse any other change than forgetting a push subscription
            if (Call.typeOf([client], call) === undefined || call.method !== "unsubscribe") {
                throw new TypeError(`the fixture receives no ${call.type.type}.${call.method}`);
            }

            // forget it in its copy
            const id = schema.identifier("client").parse(call.input["id"]);
            await this.database
                .update(client.table)
                .set({ push: null })
                .where(eq(client.table.id, id));
            this.forgotten.push(id);
        }
    }

    /** Project the space's activities into the home as the people's notifications. */
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
                    Clock.system,
                ),
            ],
        });
        const subscribed = this.store.source.projectionShape.subscription({
            name: this.homeId,
            scope: this.spaceId,
            below: this.homeId,
            parameters: {
                installation: INSTALLATION,
                service: PackageId.parse(this.store.origin.package.id),
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
            (from, signal) => this.store.source.project({ ...subscribed, ...from }, signal),
            this.#following.signal,
            { clock: this.clock },
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

/** A push service that keeps the pushes posted to it. */
export class PushService {
    /** The pushes received, in order. */
    readonly requests: ReceivedPush[] = [];
    /** The endpoint URLs the service forgot. */
    readonly gone = new Set<string>();

    /** Receive one push as fetch posts it. */
    readonly fetch = async (request: Request): Promise<Response> => {
        // keep the push with its headers
        const { url, headers } = request;
        this.requests.push({
            url,
            urgency: present(headers.get("Urgency"), "the urgency"),
            ttl: Number(present(headers.get("TTL"), "the lifetime")),
            topic: present(headers.get("Topic"), "the topic"),
            body: await request.bytes(),
        });

        return new Response(null, { status: this.gone.has(url) ? GONE : CREATED });
    };
}
