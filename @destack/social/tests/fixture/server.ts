import { LocaleTag } from "@destack/locale";
import { MemoryBuild } from "@destack/package/test";
import { anyone, principal } from "@destack/access";
import { type Call, Scope, type TrackerMessage, type Uplink } from "@destack/sync";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { STACKS } from "@destack/account/test";
import {
    and,
    type DatabaseConnection,
    type Dialect,
    eq,
    isNull,
    defineDatabase,
    Table,
} from "@destack/db";
import { channelHub } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectType } from "@destack/object";
import { announcement, notification, scheduled } from "@destack/notification";
import notificationServer from "@destack/notification/server";
import { NotificationFixture } from "@destack/notification/test";
import { build, installation, installationRevision } from "@destack/stack/object";
import { buildIndexServers } from "@destack/stack/test";
import { user } from "@destack/account/object";
import { defineServer, EphemeralStorage, ObjectStore } from "@destack/object/server";
import { PackageHandle } from "@destack/runtime";
import { callTables, testEvents } from "@destack/object/test";
import { nth, type Identifier, present, schema } from "@destack/schema";

import { RequestId } from "@destack/runtime/request";
import { reconciliation, subjectContext, testCallKey, testKeys } from "@destack/runtime/test";

import { onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import { comment, Mention, mention, presence, reply, thread } from "../../src/index.ts";
import socialServer from "../../src/server.ts";
import { article } from "./article.ts";
import { ResourceContext } from "@destack/resource/context";
import { Clock } from "@destack/environment";

/** The principals the scenarios act as: people, and an agent installation. */
export const actors = {
    alice: mentioned(principal.user, "user-019f5530-8000-7000-8000-00000000a001"),
    bob: mentioned(principal.user, "user-019f5530-8000-7000-8000-00000000a002"),
    carol: mentioned(principal.user, "user-019f5530-8000-7000-8000-00000000a003"),
    dave: mentioned(principal.user, "user-019f5530-8000-7000-8000-00000000a004"),
    agent: mentioned(principal.installation, "agent"),
};

/** A principal the scenarios act as. */
export type Actor = keyof typeof actors;

/** The installation of the articles' package serving them. */
const INSTALLATION = "installation-019f5530-8000-7000-8000-0000000000e1";

/** An uplink streaming no replicas and refusing changes. */
const silent: Uplink = {
    stream: () => (async function* () {})(),
    receive: () => Promise.reject(new Error("the fixture's uplink receives no changes")),
    call: () => Promise.reject(new Error("the fixture's uplink answers no queries")),
};

/** A row a change carries, read for its identifier. */
const RowIdentity = schema.looseObject({ id: schema.string() });

/** A page of a list call, its rows read for their identifier and creation time. */
const ListPage = schema.looseObject({
    items: schema.array(schema.looseObject({ id: schema.string(), createdAt: schema.number() })),
});

/** The social package's build, declaring the comments' notifications. */
const SOCIAL_BUILD = await MemoryBuild.of(
    comment.package,
    [mention, thread, reply].map((declared) => ({
        kind: "notification",
        kindPackageId: notification.package.id,
        name: declared.name,
        description: declared.describe(),
    })),
);

/** The types the articles' package serves. */
const ARTICLE_TYPES = [article];

/** The package of the articles. */
const articles = PackageHandle.of(article.package, {
    objects: Object.fromEntries(ARTICLE_TYPES.map((type) => [type.name, type])),
});

/** The servers a space composes: the articles, social and notifications. */
const servers = [
    defineServer(articles, { objects: ARTICLE_TYPES }),
    socialServer,
    notificationServer,
];

/** Every type the servers' packages declare. */
const objects = servers.flatMap((server) => Object.values(server.package.objects));

/** Serve articles and their attachments over a new database to one actor at a time. */
export async function serveArticles(dialect: Dialect) {
    // store the articles in a space of their own in the dialect's shared database, and presence in memory
    const storage = await durable(dialect);
    const spaceId = await openSpace(storage.database);
    const store = await serveInstallation(storage.database, await openEphemeral(), spaceId);

    // decide every call as the current actor
    let current: Actor = "alice";

    return {
        store,
        spaceId,
        /** The space's selection, which every call of the scenarios spreads. */
        scope: { spaceId },
        /** Make a call as the current actor, presence as the actor's writer unless given. */
        call: <Result>(called: Call<Result>, writerId = `writer-${current}`) =>
            request(store, spaceId, current, called, writerId),
        /** List the objects of a type the current actor may list, oldest first. */
        list: <Type extends ObjectType>(object: Type, input: object = {}) =>
            list(store, spaceId, current, object, input),
        /** Refer to a host as its attachments' parent. */
        host: (object: ObjectType, id: string) => ({
            parent: { packageId: object.policy.definition.packageId, type: object.name, id },
        }),
        /** Act as another actor. */
        as: (actor: Actor) => {
            current = actor;
        },
        /** Expand the space's announcements batch by batch, as their controller does. */
        expand: () => expand(store, storage.database, spaceId),
        /** Run the calls the space's actions scheduled, as their workflow's controller does. */
        send: () => send(store),
        /** Follow the space's presence as the current actor's client, reading every page. */
        follow: () => follow(store, spaceId, current),
    };
}

/** Call a method as an actor: a query, else a one-call mutation, presence as a writer. */
async function request<Result>(
    store: ObjectStore,
    spaceId: string,
    actor: Actor,
    called: Call<Result>,
    writerId: string,
): Promise<Result> {
    // write presence as the given writer, in the space as the actor
    const object = present(ObjectType.find(store.objects, called), "the called type");
    const caller = context(spaceId, actor).context.access();
    const options = object.storage === "durable" ? {} : { writerId };

    return object.method(called.method).mutates
        ? store
              .mutate(caller, { id: RequestId.create(Date.now()), calls: [called] }, options)
              .then(([written]) => written)
        : store.query(caller, called);
}

/** Store presence in a memory database, closed after the test. */
async function openEphemeral(): Promise<EphemeralStorage> {
    // migrate the memory database and report every failure loudly
    const memory = await TestDatabase.create("sqlite", presence.tables, { isMigrated: true });
    const store = new EphemeralStorage(
        memory.database,
        [presence],
        channelHub<TrackerMessage>()(),
        {
            clock: Clock.system,
            report: (error) => {
                throw error;
            },
        },
    );

    // close it after the test
    onTestFinished(async () => {
        store.close();
        await memory.close();
    });

    return store;
}

/** Serve the objects as the articles' installation. */
async function serveInstallation(
    database: DatabaseConnection,
    ephemeral: EphemeralStorage,
    spaceId: Identifier<"space">,
): Promise<ObjectStore> {
    const store: ObjectStore = new ObjectStore({
        // serve the space's packages, and the index of the social build it installs
        servers: [...servers, ...(await buildIndexServers(database, [SOCIAL_BUILD]))],
        chain: async () => undefined,
        clock: Clock.system,
        database,
        ephemeral,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: space.package },
        server: {
            installation: INSTALLATION,
            name: "social",
            space: spaceId,
            build: await MemoryBuild.write(new Map(), { package: article.package }),
            publisher: silent,
            settings: {},
            capabilities: [],
            publisherAt: () => silent,
            at: unfetched,
            resources: new ResourceContext(),
            keys: await testKeys(),
            fetch: () => Promise.reject(new Error("the test installation fetches nothing")),
            url: "https://social.test",
        },
    });

    return store;
}

/** Refuse an installation's calls to an address, as the scenarios call no other package. */
function unfetched(address: string) {
    return {
        url: address,
        fetch: () => Promise.reject(new Error("the test installation fetches no address")),
    };
}

/** List the objects of a type an actor may list in the space, oldest first. */
async function list<Type extends ObjectType>(
    store: ObjectStore,
    spaceId: string,
    actor: Actor,
    object: Type,
    input: object,
) {
    // list them as the actor
    const listed: ObjectType = object;
    const page = await store.query(
        context(spaceId, actor).context.access(),
        listed.call("list", { spaceId, ...input }),
    );

    return ListPage.parse(page)
        .items.toSorted((left, right) => left.createdAt - right.createdAt)
        .map((row) => Table.selectSchema<Type["table"]>(object.table).partial().parse(row));
}

/** Expand the space's announcements batch by batch as their controller does, until none waits. */
async function expand(
    store: ObjectStore,
    database: DatabaseConnection,
    spaceId: string,
): Promise<void> {
    // read the announcements the controller has not expanded yet
    const table = announcement.table;
    const waiting = () =>
        database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.scope, schema.identifier("space").parse(spaceId)),
                    isNull(table.expandedAt),
                ),
            );

    // expand them until none waits
    for (let rows = await waiting(); rows.length > 0; rows = await waiting()) {
        await store.mutate(store.serving(announcement), {
            id: RequestId.create(Date.now()),
            calls: rows.map((row) => announcement.call("expand", announcement.target(row))),
        });
    }
}

/** Run the calls the store's actions scheduled, as their workflow's controller does. */
async function send(store: ObjectStore): Promise<void> {
    const runs = present(
        store.controllers().find((controller) => controller.name === scheduled.run.name),
        "the scheduled calls' runs",
    );
    for (const listed of await runs.list()) {
        await runs.reconcile(listed, reconciliation(new AbortController().signal));
    }
}

/** Open a new space below a new account, readable by anyone through a member role. */
async function openSpace(database: DatabaseConnection): Promise<Identifier<"space">> {
    // replicate the account and the space from their home's access
    const accountId = schema.identifier("account").parse(`account-${v7()}`);
    const spaceId = schema.identifier("space").parse(`space-${v7()}`);
    const reference = space.reference(accountId, spaceId);
    const access = new AccessFixture(database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(reference);

    // create the space's row, which its installations refer to
    const now = Date.now();
    const row = { id: spaceId, scope: accountId, name: "articles", stack: STACKS.space };
    await database
        .insert(space.table)
        .values(Table.complete(space.table, { ...row, createdAt: now, updatedAt: now }));

    // define a role reading the space and bind it to anyone
    await access.replicateRole(
        reference,
        { name: "member", description: "Reads the space", permissions: [space.permission("read")] },
        anyone.reference("*", "*"),
    );

    // grant the installation serving articles, social and notification what their packages request
    await access.replicateRole(
        reference,
        {
            name: `installation/${INSTALLATION}`,
            description: "What the installation's packages request in this space",
            permissions: [
                article.permission("read"),
                ...(notificationServer.package.permissions.installation?.space ?? []),
            ],
        },
        principal.installation.reference(spaceId, INSTALLATION),
    );

    // install the social package and store each person's profile, as the space replicates them
    await NotificationFixture.install(database, spaceId, [SOCIAL_BUILD]);
    for (const [name, person] of Object.entries(actors)) {
        if (principal.user.is(person)) {
            await NotificationFixture.person(database, person, {
                name,
                locale: LocaleTag.parse("en"),
                timeZone: "UTC",
            });
        }
    }

    return spaceId;
}

/** Claim a migrated database of the durable tables in a dialect, closed after the test. */
async function durable(dialect: Dialect): Promise<TestDatabase> {
    // claim a migrated database of the dialect, closed after the test
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [
                ...callTables,
                ...objects
                    .filter((object) => object.storage === "durable")
                    .flatMap((object) => object.tables),
                ...[user, account, space, installation, installationRevision, build].flatMap(
                    (type) => type.tables,
                ),
            ],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());

    return storage;
}

/** A client following the space's presence. */
export type Follower = ReturnType<typeof follow>;

/** Follow the space's presence as an actor's client, returning the rows kept after each page. */
function follow(store: ObjectStore, spaceId: string, actor: Actor) {
    // read the pages in the background into the rows kept, waking whoever waits for the next
    const { context: followed, controller } = context(spaceId, actor);
    const pages = store.source.relayed(
        store.source.ephemeralPublication.subscription({
            name: "ephemeral",
            scope: spaceId,
            target: spaceId,
            parameters: {
                writerId: `writer-${actor}`,
                queries: { presences: { object: "presence" } },
            },
        }),
        followed,
    );
    const kept = new Map<string, Record<string, unknown>>();
    const arrivals: PromiseWithResolvers<void>[] = [];
    let arrived = 0;
    const reading = (async () => {
        for await (const page of pages) {
            // start over from a snapshot that resets, then apply the page's changes
            if (page.reset) {
                kept.clear();
            }
            for (const change of page.changes) {
                const { id } = RowIdentity.parse(change.row);
                if (change.operation === "delete") {
                    kept.delete(id);
                } else {
                    kept.set(id, change.row);
                }
            }
            arrival(arrivals, arrived).resolve();
            arrived += 1;
        }
    })();
    onTestFinished(async () => {
        // end the stream before its databases close
        controller.abort();
        await reading;
    });
    let read = 0;

    return {
        /** Read the rows once the next page arrives. */
        rows: async () => {
            await arrival(arrivals, read).promise;
            read += 1;

            return [...kept.values()];
        },
    };
}

/** Read the arrival of a page by its position, waiting on it before it arrives. */
function arrival(
    arrivals: PromiseWithResolvers<void>[],
    position: number,
): PromiseWithResolvers<void> {
    // open every arrival up to the position
    while (arrivals.length <= position) {
        arrivals.push(Promise.withResolvers<void>());
    }

    return nth(arrivals, position);
}

/** Build an actor's request context in a space with its own cancellation. */
function context(spaceId: string, actor: Actor) {
    const controller = new AbortController();
    onTestFinished(() => controller.abort());

    return {
        controller,
        context: subjectContext(actors[actor], spaceId, { signal: controller.signal }),
    };
}

/** Refer to a principal of a kind as comments mention it. */
function mentioned(kind: typeof principal.user | typeof principal.installation, id: string) {
    return Mention.shape.principal.parse(kind.reference(Scope.universe.id, id));
}
