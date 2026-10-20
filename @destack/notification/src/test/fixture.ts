import type { Call } from "@destack/sync";
import { STACKS } from "@destack/account/test";
import { readFile } from "node:fs/promises";
import {
    anyone,
    CONTAINED,
    type PermissionReference,
    PermissionSet,
    principal,
    universe,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import {
    account,
    claim,
    client,
    key,
    relationship,
    user,
    space,
    Account,
} from "@destack/account/object";
import { call, journal } from "@destack/object";
import {
    type Database,
    type DatabaseConnection,
    defineDatabase,
    type Dialect,
    eq,
    Snapshot,
    Table,
} from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { LocaleTag } from "@destack/locale";
import { Derivation } from "@destack/identity";
import { emailProvider } from "@destack/message/email";
import { MailFixture } from "@destack/message/test";
import { type PushKeys, pushProvider, Vapid } from "@destack/message/push";
import { message } from "@destack/message";
import { implementEndpoint, implementMessage, type MessageOptions } from "@destack/message/server";
import messagePackage from "@destack/message/package";
import { messageTables } from "@destack/message/stack";
import { ObjectType } from "@destack/object";
import { defineServer, ObjectStore } from "@destack/object/server";
import { serversOf, testEvents } from "@destack/object/test";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Package } from "@destack/package";
import { Identifier, present, schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { setting } from "@destack/setting/object";
import { settingServer, type SettingOptions } from "@destack/setting/server";
import { PackageHandle } from "@destack/runtime";
import { ControlLoop } from "@destack/runtime/control";
import { RequestId } from "@destack/runtime/request";
import { subjectContext, testCallKey, testServerContext } from "@destack/runtime/test";
import type { ServerContext } from "@destack/runtime/workload";
import {
    type Mutation,
    type Uplink,
    replicaPosition,
    replicaTables,
    Scope,
    ObjectReference,
    Subject,
} from "@destack/sync";
import { onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import {
    activity,
    announcement,
    deliver,
    Notification,
    notification,
    scheduled,
    subscription,
    summarize,
} from "../object/index.ts";
import { build, installation, installationRevision } from "@destack/stack/object";
import { buildIndexServers, installBuild } from "@destack/stack/test";
import { implementUser } from "@destack/account/server";
import { NotificationServer, type NotificationOptions } from "../server/index.ts";
import notificationPackage from "../package.ts";
import { PUSH_CAPABILITY } from "../message/recipient.ts";
import { Inbox } from "../notification/inbox.ts";
import { VirtualClock } from "@destack/environment/simulation";

/** The inbox's build, shipping its own German catalog. */
export const INBOX_BUILD: BuildReader = await MemoryBuild.write(
    new Map([
        [
            "locale/de.json",
            new Uint8Array(await readFile(`${import.meta.dirname}/../../locale/de.json`)),
        ],
    ]),
    { package: Package.of(notificationPackage) },
);

/** The time zone and locale each person of a scenario signs up with. */
const SIGN_UP = { timeZone: "UTC", locale: "en" };

/** The installation of the notification package in every space and home. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-019f5530-8000-7000-8000-0000000000f1");

/** The objects living in the person's home rather than the app's space. */
const HOME_OBJECTS = new Set(["notification", "message"]);

/** The most rounds a dispatch takes. */
const DISPATCH_ROUNDS = 20;

/** The longest a dispatch waits for a home's replicas of the space, in milliseconds. */
const REPLICA_TIMEOUT = 5000;

/** The status a push service answers a push it accepted with (RFC 8030 5). */
const CREATED = 201;

/** The status a push service answers a push to an endpoint it forgot with (RFC 8030 7.3). */
const GONE = 410;

/** How refused messages retry in a scenario: at once, twice. */
const RETRY = { initialInterval: 0, maximumInterval: 0, maximumAttempts: 2, backoffCoefficient: 1 };

/** A person's home: its own database, with its notifications, messages and replicas. */
export const homeDatabase = defineDatabase({
    name: "home",
    tables: [
        journal,
        ...call.tables,
        ...replicaTables,
        ...[setting, activity, announcement, subscription, notification].flatMap(
            (object) => object.tables,
        ),
        ...messageTables,
        ...[deliver, summarize, scheduled].flatMap((workflow) =>
            Object.values(workflow.objects).flatMap((object) => object.tables),
        ),
        installation.table,
        installationRevision.table,
        build.table,
    ],
    replicated: [user.table, client.table, key.table, space.table, account.table],
});

/** The setting package as a space serves it, its values written directly by the fixture. */
const SETTINGS: SettingOptions = {
    declared: () => Promise.reject(new TypeError("the fixture writes setting values directly")),
    after: [],
};

/** The universe's stand-in: the spaces and people a home follows on its person's behalf. */
export const universeDatabase = defineDatabase({
    name: "universe",
    tables: [
        journal,
        ...call.tables,
        ...replicaTables,
        ...[account, space, user, client, key, claim, relationship].flatMap(
            (object) => object.tables,
        ),
    ],
});

/** What a NotificationFixture serves: an app's objects in a space, and the people it notifies. */
export interface NotificationFixtureOptions<Name extends string> {
    /** The dialect of the databases, SQLite by default. */
    readonly dialect?: Dialect;
    /** The app's package recording the calls. */
    readonly origin: ObjectStore["origin"];
    /** The app's objects in the space, with the activities it serves. */
    readonly objects: readonly ObjectType[];
    /** The space's database with the app's objects' tables and the fixture's tables. */
    readonly database: Database;
    /** The people the scenario acts as and notifies, by name. */
    readonly people: Readonly<Record<Name, Subject>>;
    /** The person calls act as first. */
    readonly actor: NoInfer<Name>;
    /** The app's build, an empty one of its package by default. */
    readonly build?: BuildReader;
    /** The recipients of one announcement batch, 100 by default. */
    readonly batch?: number;
    /** The scenario's start in Unix milliseconds, now by default. */
    readonly at?: number;
    /** The permissions the app's workloads request on its own objects. */
    readonly requested?: readonly PermissionReference[];
}

/** A person's home: its space, its database and its store. */
interface Home {
    /** The home space. */
    readonly id: Identifier<"space">;
    /** The home's database. */
    readonly test: TestDatabase;
    /** The home's store, planning the person's activities and advancing their deliveries. */
    readonly store: ObjectStore;
}

/** An app's space and its people's homes, each in a database of its own. */
export class NotificationFixture<Name extends string> implements AsyncDisposable {
    /** The people the scenario acts as and notifies, by name. */
    readonly people: Readonly<Record<Name, Subject>>;
    /** The store of the app's objects in the space. */
    readonly store: ObjectStore;
    /** The app's space. */
    readonly spaceId: Identifier<"space">;
    /** The push service the recipients' browsers subscribe at. */
    readonly pushes: PushService;
    /** The mail transport. */
    readonly mails: MailFixture;
    /** The browser clients whose push subscriptions a home forgot at the account package. */
    readonly forgotten: string[] = [];
    /** The scenario's clock, which the stores and the fixture's calls read. */
    readonly clock: VirtualClock;
    /** The people's homes, by name, opened as a scenario first needs each. */
    readonly #homes = new Map<Name, Home>();
    /** The builds each space installs: the app's and the inbox's. */
    readonly #builds: readonly BuildReader[];
    /** The space's database. */
    readonly #test: TestDatabase;
    /** The dialect of the space's and the homes' databases. */
    readonly #dialect: Dialect;
    /** What every store serves notifications with, beside a home. */
    readonly #options: NotificationOptions;
    /** The message providers the homes send through. */
    readonly #providers: MessageOptions["providers"];
    /** The root secret every home's message key derives from. */
    readonly #root: Awaited<ReturnType<typeof Derivation.of>>;
    /** End the replicas and the inboxes. */
    readonly #following = new AbortController();
    /** The work closed before the databases: the replicas and the inboxes. */
    readonly #followers: (() => Promise<void>)[] = [];
    /** The control loops of each store's dispatched controllers, started at the first dispatch. */
    readonly #dispatchers = new Map<
        ObjectStore,
        {
            readonly stop: AbortController;
            readonly running: Promise<void>;
            readonly loop: ControlLoop;
        }
    >();
    /** The failures the dispatched controllers reported. */
    readonly #failures: unknown[] = [];
    /** The stand-in of the universe, opened with the first home. */
    #universe:
        | Promise<{
              readonly test: TestDatabase;
              readonly store: ObjectStore;
              readonly accountId: Identifier<"account">;
          }>
        | undefined;
    /** The closing of the databases, absent while open. */
    #closed: Promise<void> | undefined;
    /** The person calls act as. */
    #actor: Name;

    /** The tables a space keeps beside the app's. */
    static readonly tables: readonly Table[] = [
        journal,
        ...call.tables,
        ...replicaTables,
        installation.table,
        installationRevision.table,
        build.table,
    ];

    /** Keep the served space and the means to open homes. */
    private constructor(
        test: TestDatabase,
        store: ObjectStore,
        spaceId: Identifier<"space">,
        served: {
            readonly options: NotificationOptions;
            readonly providers: MessageOptions["providers"];
            readonly root: Awaited<ReturnType<typeof Derivation.of>>;
            readonly pushes: PushService;
            readonly mails: MailFixture;
            readonly builds: readonly BuildReader[];
        },
        options: NotificationFixtureOptions<Name> & { readonly clock: VirtualClock },
    ) {
        // keep the database, the store and the transports
        this.#test = test;
        this.#dialect = options.dialect ?? "sqlite";
        this.store = store;
        this.spaceId = spaceId;
        this.#options = served.options;
        this.#providers = served.providers;
        this.#root = served.root;
        this.pushes = served.pushes;
        this.mails = served.mails;
        this.#builds = served.builds;

        // act as the first person at the scenario's start
        this.people = options.people;
        this.#actor = options.actor;
        this.clock = options.clock;
    }

    /** The space's database. */
    get database(): DatabaseConnection {
        return this.#test.database;
    }

    /** Serve an app's objects in a new space, opening its people's homes as they are needed. */
    static async open<Name extends string>(
        options: NotificationFixtureOptions<Name>,
    ): Promise<NotificationFixture<Name>> {
        // send mail and pushes through the fixture's transports, sealed to one root's keys
        const { dialect = "sqlite", at = Date.now() } = options;
        const clock = new VirtualClock(at);
        const pushes = new PushService();
        const mails = new MailFixture();
        const vapid = await crypto.subtle.generateKey(
            { name: "ECDSA", namedCurve: "P-256" },
            true,
            ["sign", "verify"],
        );
        const root = Derivation.of(
            await Derivation.root(crypto.getRandomValues(new Uint8Array(32))),
        );
        const providers = {
            email: emailProvider(mails),
            push: pushProvider(async () => new Vapid(vapid, "https://destack.app"), pushes.fetch),
        };

        // render each notification in its declaring build's catalogs and its recipient's language
        const declaring =
            options.build ??
            (await MemoryBuild.write(new Map(), { package: options.origin.package }));
        const builds = [declaring, INBOX_BUILD];
        const served: NotificationOptions =
            options.batch === undefined ? {} : { batch: options.batch };

        // keep the space, granting the installation in it
        const test = await TestDatabase.create(dialect, options.database, { isMigrated: true });
        const spaceId = await NotificationFixture.openSpace(test.database, at);
        const installer = principal.installation.reference(spaceId, INSTALLATION);
        const requested = options.requested ?? [];
        await NotificationFixture.grant(test.database, spaceId, installer, requested, at);
        await NotificationFixture.install(test.database, spaceId, builds, at);

        // keep each person's profile in the space, in their sign-up locale
        for (const [name, person] of Object.entries<Subject>(options.people)) {
            await NotificationFixture.person(test.database, person, { name, ...SIGN_UP }, at);
        }

        // serve the app's objects and the notifications as the installation
        const app = PackageHandle.of(options.origin.package, {
            objects: Object.fromEntries(options.objects.map((each) => [each.name, each])),
        });
        const store = new ObjectStore({
            servers: [
                defineServer(app, { objects: options.objects }),
                defineServer(notificationPackage, new NotificationServer(served).definition()).at(
                    await NotificationFixture.installation(spaceId),
                ),
                settingServer(SETTINGS),
                // read the installed builds' declarations from their index
                ...(await buildIndexServers(test.database, builds)),
            ],
            chain: async () => undefined,
            principal: installer,
            database: test.database,
            // know the space, as the stack serving it follows it
            policies: [space],
            clock,
            callKey: testCallKey,
            events: await testEvents(test.database),
            origin: options.origin,
        });
        const fixture: NotificationFixture<Name> = new NotificationFixture(
            test,
            store,
            spaceId,
            { options: served, providers, root, pushes, mails, builds },
            { ...options, clock },
        );

        // close the homes and the space once the test ends
        onTestFinished(() => fixture.close());

        return fixture;
    }

    /** Keep a person's public profile in a database once, as their sign-up writes it. */
    static async person(
        database: DatabaseConnection,
        subject: Subject,
        profile: { readonly name: string; readonly locale: LocaleTag; readonly timeZone: string },
        at = Date.now(),
    ): Promise<void> {
        await database
            .insert(user.table)
            .values({
                id: user.identifier(subject.id),
                scope: Scope.universe.id,
                homeId: Account.home(Account.personal(subject.id)),
                ...profile,
                createdAt: at,
                updatedAt: at,
            })
            .onConflictDoNothing();
    }

    /** Install each build's package in a space, as applying its stack does. */
    static async install(
        database: DatabaseConnection,
        scope: Identifier<"space">,
        builds: readonly BuildReader[],
        at = Date.now(),
    ): Promise<void> {
        for (const reader of builds) {
            await installBuild(database, scope, reader, at);
        }
    }

    /** Open the home space of an account, a new one unless given, and return its identifier. */
    static async openSpace(
        database: DatabaseConnection,
        at: number,
        accountId: Identifier<"account"> = schema.identifier("account").parse(`account-${v7()}`),
    ): Promise<Identifier<"space">> {
        // replicate the account and the space from their home's access
        const spaceId = Account.home(accountId);
        const reference = space.reference(accountId, spaceId);
        const access = new AccessFixture(database);
        await access.replicateScope(account.reference(Scope.universe.id, accountId));
        await access.replicateScope(reference);

        // keep the space's row where the database replicates spaces
        if (database.tables.includes(space.table)) {
            const row = {
                id: spaceId,
                scope: accountId,
                name: "space",
                stack: STACKS.space,
                createdAt: at,
                updatedAt: at,
            };
            await database.insert(space.table).values(Table.complete(space.table, row));
        }

        // define a role reading the space and bind it to anyone
        await access.replicateRole(
            reference,
            {
                name: "visitor",
                description: "Reads the space",
                permissions: [space.permission("read")],
            },
            anyone.reference("*", "*"),
            at,
        );

        return spaceId;
    }

    /** Describe the notification installation of a space, which may push. */
    static async installation(spaceId: string): Promise<ServerContext> {
        return {
            ...(await testServerContext(Package.of(notificationPackage), spaceId)),
            installation: INSTALLATION,
            capabilities: [PUSH_CAPABILITY],
        };
    }

    /** Grant an installation in a space what its package and its push capability request. */
    static async grant(
        database: DatabaseConnection,
        spaceId: string,
        installer: Subject,
        requested: readonly PermissionReference[],
        at: number,
    ): Promise<void> {
        // bind the role with the space permissions the package requests and the app's ones
        const access = new AccessFixture(database);
        const lent = notificationPackage.permissions.installation ?? {};
        await access.replicateRole(
            await Scope.object(Snapshot.live(database), spaceId),
            {
                name: `installation/${installer.id}`,
                description: "What the installation requests in this space",
                permissions: [...(lent.space ?? []), ...requested],
            },
            installer,
            at,
        );

        // grant the installation what its space lends beyond the space, the allowed push capability's too
        const delegator = principal.space.reference(Scope.universe.id, spaceId);
        const scopes = { space: spaceId, home: [], account: undefined };
        const push =
            notificationPackage.capabilities[PUSH_CAPABILITY]?.permissions?.installation ?? {};
        const beyond = [lent, push]
            .flatMap((set) => PermissionSet.grant(set, scopes))
            .filter((permission) => permission.scope !== spaceId);
        await access.replicateGrant(delegator, installer, beyond, at);
    }

    /** Close the replicas and the inboxes, then the databases. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Open the universe's stand-in once: the space's row and access, served as the universe. */
    #openUniverse(at: number): Promise<{
        readonly test: TestDatabase;
        readonly store: ObjectStore;
        readonly accountId: Identifier<"account">;
    }> {
        this.#universe ??= (async () => {
            // keep the space's row and access in the universe's database
            const test = await TestDatabase.create(this.#dialect, universeDatabase, {
                isMigrated: true,
            });
            const accountId = schema
                .identifier("account")
                .parse(`account-${Identifier.uuid(this.spaceId)}`);
            await NotificationFixture.openSpace(test.database, at, accountId);
            const store = new ObjectStore({
                chain: async () => undefined,
                clock: this.clock,
                principal: universe.reference(Scope.universe.id, Scope.universe.id),
                servers: serversOf([account, space, implementUser(), relationship]),
                claims: { object: claim },
                database: test.database,
                callKey: testCallKey,
                events: await testEvents(test.database),
                origin: { package: notificationPackage },
            });

            return { test, store, accountId };
        })();

        return this.#universe;
    }

    /** Close the replicas and the inboxes, then the databases, once. */
    close(): Promise<void> {
        this.#closed ??= (async () => {
            // end the replicas, the inboxes and the dispatched controllers
            this.#following.abort();
            for (const follower of this.#followers) {
                await follower();
            }
            for (const { running } of this.#dispatchers.values()) {
                await running;
            }

            // close the homes' databases, then the space's
            for (const home of this.#homes.values()) {
                await home.test.close();
            }
            await (await this.#universe)?.test.close();
            await this.#test.close();
        })();

        return this.#closed;
    }

    /** Move the clock by some minutes, letting the homes' follows confirm their replicas. */
    async wait(minutes: number): Promise<void> {
        // stop the dispatched controllers as an evicted worker, which the next dispatch starts again
        for (const { stop, running } of this.#dispatchers.values()) {
            stop.abort();
            await running;
        }
        this.#dispatchers.clear();

        // move the clock past the feeds' heartbeats, which repeat positions to caught-up follows
        this.clock.advance(minutes * 60_000);
        const now = this.clock.now();

        // wait until each home's followed replicas are confirmed at the new time
        for (const home of this.#homes.values()) {
            const { database } = home.test;
            const isConfirmed = async () => {
                const rows = await database
                    .select({ sequence: replicaPosition.sequence, at: replicaPosition.confirmedAt })
                    .from(replicaPosition);

                return rows.every((row) => row.sequence === null || row.at >= now);
            };
            await database.log.until(isConfirmed, AbortSignal.timeout(REPLICA_TIMEOUT));
        }
    }

    /** Act as another person. */
    as(person: Name): void {
        this.#actor = person;
    }

    /** The space's selection, which the calls of the app's types spread. */
    get scope(): { readonly spaceId: Identifier<"space"> } {
        return { spaceId: this.spaceId };
    }

    /** Read the current person's home's selection, which the calls of the home's types spread. */
    async homeScope(): Promise<{ readonly spaceId: Identifier<"space"> }> {
        return { spaceId: (await this.home(this.#actor)).id };
    }

    /** Make a call as the current person, in their home for the home's types, else in the space. */
    async call<Result>(called: Call<Result>): Promise<Result> {
        // read through a query, or write through a one-call mutation, in the call's store
        const object = present(
            ObjectType.find([...this.store.objects, notification, message], called),
            "the called type",
        );
        const { store, id: scope } = HOME_OBJECTS.has(object.name)
            ? await this.home(this.#actor)
            : { store: this.store, id: this.spaceId };
        const caller = this.#context(scope).access();
        const id = RequestId.create(this.clock.now());

        return object.method(called.method).mutates
            ? store.mutate(caller, { id, calls: [called] }, { scope }).then(([result]) => result)
            : store.query(caller, called);
    }

    /** Open a person's home once, with their profile and replicas of the space. */
    async home(person: Name): Promise<Home> {
        // keep an opened home
        const opened = this.#homes.get(person);
        if (opened !== undefined) {
            return opened;
        }

        // keep the home in a database of its own, owned by the person, with their profile and the installation's grant
        const subject = this.people[person];
        const at = this.clock.now();
        const test = await TestDatabase.create(this.#dialect, homeDatabase, { isMigrated: true });
        const id = await NotificationFixture.openSpace(
            test.database,
            at,
            Account.personal(subject.id),
        );
        const owned = await Scope.object(Snapshot.live(test.database), id);
        await new AccessFixture(test.database).bindOwner(owned, subject, at);

        // keep the person's personal account, which makes the space their home, as its chain does
        await test.database.insert(account.table).values({
            id: Account.personal(subject.id),
            scope: subject.id,
            handle: person,
            name: person,
            residencyId: "eu",
            kind: "personal",
            createdAt: at,
            updatedAt: at,
        });
        const installer = principal.installation.reference(id, INSTALLATION);
        await NotificationFixture.grant(test.database, id, installer, [], at);
        await NotificationFixture.install(test.database, id, this.#builds, at);
        // keep the person's profile in their home, the space and the universe, whose follow relation admits the home
        const { store: universal, test: universeTest, accountId } = await this.#openUniverse(at);
        for (const database of [test.database, this.#test.database, universeTest.database]) {
            await NotificationFixture.person(database, subject, { name: person, ...SIGN_UP }, at);
        }

        // record the person, their personal account and its home at the universe, as sign-up does
        const universeAccess = new AccessFixture(universeTest.database);
        const personal = Account.personal(subject.id);
        await universeAccess.replicateScope(subject);
        await universeAccess.replicateScope(account.reference(Scope.universe.id, personal));
        await universeAccess.replicateScope(space.reference(personal, id));

        // relate the person to the space at the universe, as its membership does
        await universeAccess.replicateRelationship(
            accountId,
            space.reference(accountId, this.spaceId),
            "guest",
            subject,
            at,
        );

        // serve the notifications and the messages as the installation, following the space's activities as the home
        const homeSpace = principal.space.reference(Scope.universe.id, id);
        const homeSets = [
            { ...account.reference(Scope.universe.id, personal), relation: CONTAINED },
            { ...subject, relation: "home" },
        ];
        const uplink: Uplink = {
            confirm: async () => {},
            stream: idle,
            receive: (mutation: Mutation) => this.#receive(universeTest, mutation),
            // answer the person's queries at the universe's stand-in
            call: (asked, scope, signer) => universal.uplink.call(asked, scope, signer),
        };
        const store = new ObjectStore({
            servers: [
                defineServer(
                    notificationPackage,
                    new NotificationServer(this.#options).definition(),
                ).at(await NotificationFixture.installation(id)),
                defineServer(messagePackage, {
                    objects: [
                        implementMessage({
                            providers: this.#providers,
                            retry: RETRY,
                            keys: this.#root,
                        }),
                        implementEndpoint(this.#root),
                    ],
                }),
                settingServer(SETTINGS),
                // read the installed builds' declarations from their index
                ...(await buildIndexServers(test.database, this.#builds)),
            ],
            principal: installer,
            database: test.database,
            clock: this.clock,
            callKey: testCallKey,
            events: await testEvents(test.database),
            origin: { package: notificationPackage },
            // know the home's space and the person's clients
            policies: [client, space],
            uplink,
            // keep the home's chain above it, as its machine does where the home is placed
            chain: async () => ({ kind: "home", parent: Account.personal(subject.id) }),
            // follow the app's space at its store and every other scope at the universe's stand-in, as the home
            publisher: (scope) => ({
                stream: (request, signal) =>
                    (scope === this.spaceId ? this.store : universal).source.relayed(
                        request,
                        subjectContext(homeSpace, scope, {
                            signal,
                            clock: this.clock,
                            groups: homeSets,
                        }),
                    ),
                confirm: (request, watermark) =>
                    (scope === this.spaceId ? this.store : universal).source.confirm(
                        request,
                        watermark,
                    ),
            }),
        });
        const home: Home = { id, test, store };
        this.#homes.set(person, home);

        // follow the replicas in the background until the fixture closes
        this.#replicate(home);

        return home;
    }

    /** Make a person a member of the space, reading it. */
    async join(person: Name): Promise<void> {
        await new AccessFixture(this.database).replicateRole(
            await Scope.object(Snapshot.live(this.database), this.spaceId),
            {
                name: `member/${person}`,
                description: "Reads the space as its member",
                permissions: [space.permission("read")],
            },
            this.people[person],
            this.clock.now(),
        );
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
        // keep the verified email, the time zone and the locale on the person's user in every database
        await this.home(person);
        const fields = {
            ...(profile.email === undefined ? {} : { email: profile.email, emailVerified: true }),
            ...(profile.timeZone === undefined ? {} : { timeZone: profile.timeZone }),
            ...(profile.locale === undefined ? {} : { locale: profile.locale }),
        };
        const id = schema.identifier("user").parse(this.people[person].id);
        const universal = await this.#universe;
        const databases = [
            this.#test.database,
            ...(universal === undefined ? [] : [universal.test.database]),
            ...[...this.#homes.values()].map((home) => home.test.database),
        ];
        for (const database of databases) {
            await database
                .update(user.table)
                .set({ ...fields, updatedAt: this.clock.now() })
                .where(eq(user.table.id, id));
        }
    }

    /** Set a person's value of a setting in their scope, as their home replicates it. */
    async set<Value extends schema.Schema>(
        person: Name,
        declared: Setting<Value>,
        value: schema.Infer<Value>,
    ): Promise<void> {
        // keep the value in the person's home, as its replica of their scope does
        const { test } = await this.home(person);
        const subject = this.people[person];
        await test.database.insert(setting.table).values({
            id: schema.identifier("setting").parse(`setting-${v7()}`),
            createdAt: this.clock.now(),
            ...ObjectReference.columns("createdBy", subject),
            updatedAt: this.clock.now(),
            ...ObjectReference.columns("updatedBy", subject),
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
        // keep the client at the universe, which owns the person's clients, beside their home
        await this.home(person);
        const { test } = await this.#openUniverse(this.clock.now());
        const scope = schema.identifier("user").parse(this.people[person].id);
        const revokedAt = isRevoked ? this.clock.now() : null;
        await test.database
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
        // keep the client at the universe, which owns the person's clients, beside their home
        await this.home(person);
        const { test } = await this.#openUniverse(this.clock.now());
        await test.database.insert(client.table).values({
            id: browser.id,
            scope: schema.identifier("user").parse(this.people[person].id),
            kind: "browser",
            name: "Browser",
            push: { url: browser.url, keys: browser.keys },
            createdAt: this.clock.now(),
            updatedAt: this.clock.now(),
        });
    }

    /** Let the homes replicate the space's activities and the controllers settle. */
    async dispatch(): Promise<void> {
        for (let round = 0; round < DISPATCH_ROUNDS; round++) {
            // wait for each home's replica of the space to catch up with its latest change
            const position = await this.database.log.position();
            for (const home of this.#homes.values()) {
                const isReplicated = await home.store.wait(
                    { scope: this.spaceId, ...position },
                    AbortSignal.timeout(REPLICA_TIMEOUT),
                );
                if (!isReplicated) {
                    throw new TypeError("a home's replica never caught up with the space");
                }
            }

            // let the space's and the homes' controllers settle, failing on their errors
            const before = await this.#positions();
            const homes = [...this.#homes.values()];
            const stores = [this.store, ...homes.map((home) => home.store)];
            await Promise.all(stores.map((store) => this.#dispatcher(store).idle()));
            if (this.#failures.length > 0) {
                throw this.#failures[0];
            }

            // stop once a round changes nothing
            if ((await this.#positions()) === before) {
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
                left.recipientId.localeCompare(right.recipientId) ||
                left.createdAt - right.createdAt ||
                left.id.localeCompare(right.id),
        );
    }

    /** Read a person's inbox in their home, as the home keeps it, latest occurrence first. */
    async inbox(person: Name): Promise<Inbox> {
        // join the home's notifications with its activity replicas
        const { test } = await this.home(person);
        const notifications = await test.database.select().from(notification.table);
        const activities = await test.database.select().from(activity.table);

        return new Inbox(notifications, activities, this.clock.now());
    }

    /** Read a person's messages of a notification, by channel. */
    async messages(person: Name, id: string) {
        const home = await this.home(person);
        const rows = await home.test.database.select().from(message.table);

        return rows
            .filter((row) => Notification.read(row) === id)
            .map((row) => ({ to: row.to, status: row.status, reason: row.reason }))
            .toSorted((left, right) => left.to.channel.localeCompare(right.to.channel));
    }

    /** Authenticate the current person in a scope at the scenario's time. */
    #context(scope: string) {
        return subjectContext(this.people[this.#actor], scope, {
            signal: this.#following.signal,
            clock: this.clock,
        });
    }

    /** Read the log positions of the space and the homes, as one comparable text. */
    async #positions(): Promise<string> {
        const databases = [this.#test, ...[...this.#homes.values()].map((home) => home.test)];
        const positions = await Promise.all(
            databases.map(async (test) => (await test.database.log.position()).sequence),
        );

        return positions.join(" ");
    }

    /** Read the control loop of a store's dispatched controllers, started on first use. */
    #dispatcher(store: ObjectStore): ControlLoop {
        // reuse the store's running loop
        const known = this.#dispatchers.get(store);
        if (known !== undefined) {
            return known.loop;
        }

        // run the dispatched controllers until the fixture closes, collecting their failures
        const dispatched = store.controllers().filter((each) => each.mode !== "follow");
        const loop = new ControlLoop(store.database, dispatched, {
            clock: this.clock,
            report: (_controller, _key, error) => this.#failures.push(error),
        });
        const stop = new AbortController();
        const running = loop
            .run(AbortSignal.any([this.#following.signal, stop.signal]))
            .catch((error: unknown) => {
                this.#failures.push(error);
            });
        this.#dispatchers.set(store, { stop, running, loop });

        return loop;
    }

    /** Follow a home's replicas of the space in the background until the fixture closes. */
    #replicate(home: Home): void {
        const replicas = home.store.controllers().filter((each) => each.mode === "follow");
        const running = new ControlLoop(home.test.database, replicas, {
            clock: this.clock,
            report: (_controller, _key, error) => {
                throw error;
            },
        }).run(this.#following.signal);
        this.#followers.push(() => running);
    }

    /** Receive a home's changes of replicated clients as the account package that keeps them. */
    async #receive(test: TestDatabase, mutation: Mutation): Promise<readonly unknown[]> {
        for (const received of mutation.calls) {
            // refuse any other change than forgetting a push subscription
            if (
                ObjectType.find([client], received) === undefined ||
                received.method !== "unsubscribe"
            ) {
                throw new TypeError(
                    `the fixture receives no ${received.type.type}.${received.method}`,
                );
            }

            // forget it at the universe
            const id = schema.identifier("client").parse(received.input["id"]);
            await test.database
                .update(client.table)
                .set({ push: null })
                .where(eq(client.table.id, id));
            this.forgotten.push(id);
        }

        return mutation.calls.map(() => ({}));
    }
}

/** Stream no replicas, as a home sends its clients' changes and replicates nothing upward. */
async function* idle() {}

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
