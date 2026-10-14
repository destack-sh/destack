import { LocaleTag } from "@destack/locale";
import { MemoryBuild } from "@destack/package/test";
import { anyone, principal, Caller } from "@destack/access";
import { Scope, type TrackerMessage, type Uplink } from "@destack/sync";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import {
    and,
    type DatabaseConnection,
    type Dialect,
    eq,
    isNull,
    defineDatabase,
} from "@destack/db";
import { channelHub } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Call, type CallableName, type CallOutput, type ObjectType } from "@destack/object";
import { announcement, notification } from "@destack/notification";
import { notificationExtension } from "@destack/notification/server";
import { EphemeralStorage, ObjectExtension, ObjectStore } from "@destack/object/server";
import { PackageHandle } from "@destack/runtime";
import { callTables, TestRunClient, testEvents } from "@destack/object/test";
import { reconciliation } from "@destack/runtime/test";
import { aligned, type Identifier, present, schema, Clock } from "@destack/schema";

import { RequestId } from "@destack/runtime/request";
import { subjectContext, testCallKey } from "@destack/runtime/test";

import { onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import { comment, mention, presence, reply, thread } from "../../src/index.ts";
import socialServer from "../../src/server/index.ts";
import { article } from "./article.ts";
import { ResourceContext } from "@destack/resource/context";

/** The principals the scenarios act as: people, and an agent installation. */
export const actors = {
    alice: principal.user.reference(Scope.universe.id, "alice"),
    bob: principal.user.reference(Scope.universe.id, "bob"),
    carol: principal.user.reference(Scope.universe.id, "carol"),
    dave: principal.user.reference(Scope.universe.id, "dave"),
    agent: principal.installation.reference(Scope.universe.id, "agent"),
};

/** A principal the scenarios act as. */
export type Actor = keyof typeof actors;

/** The installation of the articles' package serving them. */
const INSTALLATION = "installation-019f5530-8000-7000-8000-0000000000e1";

/** An uplink streaming no replicas and refusing changes. */
const silent: Uplink = {
    stream: () => (async function* () {})(),
    receive: () => Promise.reject(new Error("the fixture's uplink receives no changes")),
};

/** A row a change carries, read for its identifier. */
const RowIdentity = schema.looseObject({ id: schema.string() });

/** A page of a list call, its rows read for their identifier and creation time. */
const ListPage = schema.looseObject({
    items: schema.array(schema.looseObject({ id: schema.string(), createdAt: schema.number() })),
});

/** The social package's build, declaring the comments' notifications. */
const SOCIAL_BUILD = await MemoryBuild.declaring(
    comment.package,
    [mention, thread, reply].map((declared) => ({
        kind: "notification",
        package: notification.package.id,
        name: declared.name,
        description: declared.describe(),
    })),
);

/** The types the articles' package serves. */
const ARTICLE_TYPES = [article];

/** The extensions a space composes: the articles, social and notifications. */
const extensions = [
    {
        package: PackageHandle.of(article.package, {
            objects: Object.fromEntries(ARTICLE_TYPES.map((type) => [type.name, type])),
        }),
        objects: ARTICLE_TYPES,
    },
    socialServer,
    notificationExtension({
        declared: async () => SOCIAL_BUILD,
        locale: async () => LocaleTag.parse("en"),
        key: () => Promise.reject(new TypeError("the fixture seals no messages")),
    }),
];

/** Every type the extensions serve. */
const objects = extensions.flatMap((extension) => extension.objects);

/** Serve articles and their attachments over a new database to one actor at a time. */
export async function serveArticles(dialect: Dialect) {
    // keep the articles in a space of their own in the dialect's shared database, and presence in memory
    const storage = await durable(dialect);
    const spaceId = await openSpace(storage.database);
    const store = await serveInstallation(storage.database, await openEphemeral(), spaceId);

    // decide every call as the current actor
    let current: Actor = "alice";

    return {
        store,
        spaceId,
        /** Call a method as the current actor. */
        call: <Type extends ObjectType, Name extends CallableName<Type>>(
            object: Type,
            name: Name,
            input: Readonly<Record<string, unknown>>,
        ) => request(store, spaceId, current, object, name, input),
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
        /** Run the calls the space's mutations sent, as the machine runs them. */
        send: () => send(store),
        /** Follow the space's presence as the current actor's client, reading every page. */
        follow: () => follow(store, spaceId, current),
    };
}

/** Call a method as an actor: a query, else a one-call mutation, presence as a writer. */
async function request<Type extends ObjectType, Name extends CallableName<Type>>(
    store: ObjectStore,
    spaceId: string,
    actor: Actor,
    object: Type,
    name: Name,
    input: Readonly<Record<string, unknown>>,
): Promise<CallOutput<Type, Name>>;
/**
 * Call a method as an actor.
 *
 * @construct the result is the method's output, which CallOutput reads from the same method.
 */
async function request(
    store: ObjectStore,
    spaceId: string,
    actor: Actor,
    object: ObjectType,
    name: string,
    input: Readonly<Record<string, unknown>>,
): Promise<unknown> {
    // write presence as the given writer or the actor's
    const { writerId = `writer-${actor}`, ...fields } = input;
    const call = Call.record(object, name, { spaceId, ...fields });
    const caller = context(spaceId, actor).context.caller();
    const options =
        object.storage === "durable" ? {} : { writerId: schema.string().parse(writerId) };

    return object.method(name).mutates
        ? store
              .mutate(caller, { id: RequestId.create(Date.now()), calls: [call] }, options)
              .then(([written]) => written)
        : store.query(caller, call);
}

/** Keep presence in a memory database, closed after the test. */
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
    const composed: { readonly store: ObjectStore } = ObjectExtension.compose(extensions, {
        clock: Clock.system,
        database,
        ephemeral,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: space.package },
        runs: new TestRunClient(() => composed.store),
        installation: {
            id: INSTALLATION,
            scope: spaceId,
            build: await MemoryBuild.write(new Map(), { package: article.package }),
            publisher: silent,
            settings: {},
            publisherAt: () => silent,
            at: unfetched,
            resources: new ResourceContext(),
            fetch: () => Promise.reject(new Error("the test installation fetches nothing")),
        },
    });

    return composed.store;
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
        context(spaceId, actor).context.caller(),
        listed.call("list", { spaceId, ...input }),
    );

    return ListPage.parse(page)
        .items.toSorted((left, right) => left.createdAt - right.createdAt)
        .map((row) => object.rowSchema().parse(row));
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
        await store.mutate(Caller.of(store.principal, Date.now()), {
            id: RequestId.create(Date.now()),
            calls: rows.map((row) => announcement.call("expand", announcement.target(row))),
        });
    }
}

/** Run the calls the store's mutations sent, as the machine runs them. */
async function send(store: ObjectStore): Promise<void> {
    const sends = present(
        store.controllers().find((controller) => controller.name === "sends"),
        "the store's sends",
    );
    for (const listed of await sends.list()) {
        await sends.reconcile(listed, reconciliation(new AbortController().signal));
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

    // define a role reading the space and bind it to anyone
    await access.replicateRole(
        reference,
        { name: "member", description: "Reads the space", permissions: [space.permission("read")] },
        anyone.reference("*", "*"),
    );

    // grant the articles' installation what its workloads request, as approving it does
    await access.replicateRole(
        reference,
        {
            name: `installation/${INSTALLATION}`,
            description: "What the articles' installation requests in this space",
            permissions: [announcement.permission("expand"), article.permission("read")],
        },
        principal.installation.reference(spaceId, INSTALLATION),
    );

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
        store.source.ephemeralShape.subscription({
            name: "ephemeral",
            scope: spaceId,
            below: spaceId,
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

    return aligned(arrivals, position);
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
