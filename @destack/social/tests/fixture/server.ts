import { LocaleTag } from "@destack/locale";
import { MemoryBuild } from "@destack/package/test";
import { anyone, principal, Caller } from "@destack/access";
import { Scope, type TrackerMessage, type Uplink } from "@destack/sync";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
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
import { subscription } from "@destack/notification";
import { serveActivities } from "@destack/notification/server";
import { EphemeralStorage, ObjectStore } from "@destack/object/server";
import { aligned, type Identifier, schema, Clock } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { subjectContext, testCallKey } from "@destack/service/test";

import { onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import { favourite, mention, presence, reaction, receipt, reply, thread } from "../../src/index.ts";
import { comment } from "../../src/server/index.ts";
import { article } from "./article.ts";

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

/** An uplink streaming no copies and refusing changes. */
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

/** The activity objects answering actions and expanding the comments' announcements into activities. */
const notifications = serveActivities({ notifications: [mention, thread, reply] });

/** Every social type, served beside the articles taking them and the activity types comments write. */
const objects = {
    article,
    comment,
    reaction,
    receipt,
    favourite,
    presence,
    subscription,
    ...notifications,
};

/** Serve articles and their attachments over a new database to one actor at a time, alice until another acts. */
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
        /** Call a method as the current actor, replayable by request for durable objects and by writer for presence. */
        call: async <Type extends ObjectType, Name extends CallableName<Type>>(
            object: Type,
            name: Name,
            input: Readonly<Record<string, unknown>>,
        ) => {
            // read through a query, or write through a one-call mutation, presence as the given writer or the actor's
            const { writerId = `writer-${current}`, ...fields } = input;
            const call = Call.record(object, name, { spaceId, ...fields });
            const caller = context(spaceId, current).context.caller();
            const options =
                object.storage === "durable" ? {} : { writerId: schema.string().parse(writerId) };

            const result = object.method(name).mutates
                ? store
                      .mutate(caller, { id: RequestId.create(Date.now()), calls: [call] }, options)
                      .then(([written]) => written)
                : store.query(caller, call);

            return result as Promise<CallOutput<Type, Name>>;
        },
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
        /** Expand the space's announcements batch by batch as their controller does, until none waits. */
        expand: () => expand(store, storage.database, spaceId),
        /** Follow the space's presence as the current actor's client and read every page like a transport. */
        follow: () => follow(store, spaceId, current),
    };
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

/** Serve the objects as the articles' installation, rendering notifications in no recipient's own locale. */
async function serveInstallation(
    database: DatabaseConnection,
    ephemeral: EphemeralStorage,
    spaceId: Identifier<"space">,
): Promise<ObjectStore> {
    return new ObjectStore({
        clock: Clock.system,
        principal: principal.installation.reference(spaceId, INSTALLATION),
        objects,
        database,
        ephemeral,
        callKey: testCallKey,
        origin: { package: space.package, service: "social" },
        installation: {
            id: INSTALLATION,
            scope: spaceId,
            build: await MemoryBuild.write(new Map(), { package: article.package }),
            publisher: silent,
            settings: {},
            publisherAt: () => silent,
            at: unfetched,
            directory: {
                isHome: async () => false,
                locale: async () => LocaleTag.parse("en"),
                address: async () => {},
                messageKey: () => Promise.reject(new TypeError("the fixture seals no messages")),
            },
        },
    });
}

/** Answer an installation's calls to an address with a refusal, as the scenarios call no other service. */
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
    const table = notifications.announcement.table;
    const waiting = () =>
        database
            .select()
            .from(table)
            .where(and(eq(table.scope, spaceId), isNull(table.expandedAt)));

    // expand them until none waits
    for (let rows = await waiting(); rows.length > 0; rows = await waiting()) {
        await store.mutate(Caller.of(store.principal, Date.now()), {
            id: RequestId.create(Date.now()),
            calls: rows.map((row) =>
                notifications.announcement.call("expand", notifications.announcement.target(row)),
            ),
        });
    }
}

/** Open a new space below a new account, readable by anyone through a member role, returning its identifier. */
async function openSpace(database: DatabaseConnection): Promise<Identifier<"space">> {
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
        { name: "member", description: "Reads the space", permissions: [space.permission("read")] },
        anyone.reference("*", "*"),
    );

    // grant the articles' installation what its workloads request, as approving it does
    await accessCopies.copyRole(
        reference,
        {
            name: `installation/${INSTALLATION}`,
            description: "What the articles' installation requests in this space",
            permissions: [
                notifications.announcement.permission("expand"),
                article.permission("read"),
            ],
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
                journal,
                ...Object.values(objects)
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
