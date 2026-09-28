import {
    accessRelationship,
    accessRole,
    anyone,
    GLOBAL_SCOPE,
    principal,
    Relationship,
    Role,
} from "@destack/access";
import { copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import type { AuditRecorder } from "@destack/audit";
import { and, type DatabaseConnection, type Dialect, eq, isNull } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { relayHub, TestDatabase } from "@destack/db/test";
import type { ObjectType } from "@destack/object";
import { subscription } from "@destack/notification";
import { Dispatcher } from "@destack/notification/server";
import { EphemeralStorage, ObjectServer, SystemCall } from "@destack/object/server";
import { Package } from "@destack/package";
import { type Identifier, identifier, schema } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { defineJournal, Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineSetting } from "@destack/setting/declare";
import { space } from "@destack/space/object";
import type { TrackerMessage } from "@destack/sync";
import { afterAll, onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import {
    comment,
    favourite,
    mention,
    presence,
    reaction,
    receipt,
    reply,
    thread,
} from "../../src/index.ts";
import { article } from "./article.ts";

/** Replayable method requests. */
const journal = defineJournal("journal");

/** The principals the scenarios act as: people, and an agent installation. */
export const actors = {
    alice: principal.user.reference(GLOBAL_SCOPE, "alice"),
    bob: principal.user.reference(GLOBAL_SCOPE, "bob"),
    carol: principal.user.reference(GLOBAL_SCOPE, "carol"),
    dave: principal.user.reference(GLOBAL_SCOPE, "dave"),
    agent: principal.installation.reference(GLOBAL_SCOPE, "agent"),
};

/** A principal the scenarios act as. */
export type Actor = keyof typeof actors;

/** The durable database of each dialect, shared by the scenarios of one test file. */
const databases = new Map<Dialect, Promise<TestDatabase>>();
afterAll(async () => {
    for (const storage of databases.values()) {
        await (await storage).close();
    }
});

/** Refuse delivering, which the scenarios leave to notification. */
function refuseDelivery(): never {
    throw new TypeError("the scenarios expand announcements and deliver nothing");
}

/** The dispatcher expanding the comments' announcements into notifications. */
const dispatcher = new Dispatcher({
    notifications: [mention, thread, reply],
    recipients: {
        settings: refuseDelivery,
        addresses: refuseDelivery,
        devices: refuseDelivery,
        timeZone: refuseDelivery,
        forget: refuseDelivery,
    },
    push: { send: refuseDelivery },
    mail: { send: refuseDelivery },
});

/** Every social type, served beside the articles taking them and the notification types comments write. */
const objects = {
    article,
    comment,
    reaction,
    receipt,
    favourite,
    presence,
    subscription,
    ...dispatcher.objects,
};

/** Serve articles and their attachments over a new database to one actor at a time, alice until another acts. */
export async function serveArticles(dialect: Dialect) {
    // hold the articles in a space of their own in the dialect's shared database, and presence in memory
    const storage = await durable(dialect);
    const spaceId = await openSpace(storage.database);
    const memory = await TestDatabase.create("sqlite", presence.tables, { isMigrated: true });
    const store = new EphemeralStorage(memory.database, [presence], relayHub<TrackerMessage>()(), {
        report: (error) => {
            throw error;
        },
    });
    onTestFinished(async () => {
        store.close();
        await memory.close();
    });

    // decide every call as the current actor
    let current: Actor = "alice";
    const server = new ObjectServer({
        objects,
        database: storage.database,
        ephemeral: store,
        context: (context) => ({
            subjects: [actors[context.requireCaller().id as Actor]],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(journal),
        audit: () => ({ record: async () => {} }) as unknown as AuditRecorder<DatabaseConnection>,
    });

    return {
        server,
        spaceId,
        /** Call a method as the current actor, replayable by request for durable objects and by client for presence. */
        call: async (object: ObjectType, name: string, input: object) =>
            (await server.call(
                object,
                name,
                {
                    spaceId,
                    ...(object.storage === "durable"
                        ? { requestId: RequestId.create() }
                        : { client: `client-${current}` }),
                    ...input,
                },
                context(spaceId, current).context,
            )) as Record<string, unknown> & { readonly id: string },
        /** List the objects of a type the current actor may list, oldest first. */
        list: async (object: ObjectType, input: object = {}) =>
            (
                (await server.call(
                    object,
                    "list",
                    { spaceId, ...input },
                    context(spaceId, current).context,
                )) as { readonly items: readonly (Record<string, unknown> & { id: string })[] }
            ).items.toSorted(
                (left, right) => (left.createdAt as number) - (right.createdAt as number),
            ),
        /** Name a host as its attachments' parent. */
        host: (object: ObjectType, id: string) => ({
            parent: { packageId: object.policy.definition.packageId, type: object.name, id },
        }),
        /** Act as another actor. */
        as: (actor: Actor) => {
            current = actor;
        },
        /** Expand the space's announcements batch by batch as the dispatcher does, until none waits. */
        expand: async () => {
            const table = dispatcher.objects.announcement.table;
            const waiting = () =>
                storage.database
                    .select()
                    .from(table)
                    .where(and(eq(table.scope, spaceId), isNull(table.expandedAt)));
            for (let rows = await waiting(); rows.length > 0; rows = await waiting()) {
                await server.executeAsSystem(
                    dispatcher.objects.announcement,
                    "expand",
                    rows.map(SystemCall.of),
                    Date.now(),
                );
            }
        },
        /** Follow the space's presence as the current actor's client, reading every page as a transport does. */
        follow: () => follow(server, spaceId, current),
    };
}

/** Open a new space below a new account, readable by anyone through a member role, returning its identifier. */
async function openSpace(database: DatabaseConnection): Promise<Identifier<"space">> {
    // record the account and the space as copies of their home's access hold them
    const accountId = identifier("account").parse(`account-${v7()}`);
    const spaceId = identifier("space").parse(`space-${v7()}`);
    const reference = space.reference(accountId, spaceId);
    await copyScope(database, account.reference(GLOBAL_SCOPE, accountId));
    await copyScope(database, reference);

    // define a role reading the space and bind it to anyone
    const now = Date.now();
    const role = identifier("role").parse(`role-${v7()}`);
    await database.insert(accessRole).values({
        id: role,
        createdAt: now,
        updatedAt: now,
        scope: spaceId,
        name: "member",
        description: "Reads the space",
    });
    const read = space.permission("read");
    await Role.permit(database, role, spaceId, [
        { packageId: read.packageId, type: read.type, name: read.name },
    ]);
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: identifier("relationship").parse(`relationship-${v7()}`),
                object: reference,
                role,
                subject: anyone.reference("*", "*"),
                createdAt: now,
                expiresAt: null,
            },
            spaceId,
        ),
    );

    return spaceId;
}

/** Hold the durable tables once per dialect for the scenarios of a file, each of which opens a space of its own. */
function durable(dialect: Dialect): Promise<TestDatabase> {
    // migrate the dialect's database on first use, and close it after the file's scenarios
    let storage = databases.get(dialect);
    if (storage === undefined) {
        storage = TestDatabase.create(
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
        databases.set(dialect, storage);
    }

    return storage;
}

/** Follow the space's presence as an actor's client, returning the rows held after each page. */
function follow(server: ObjectServer<typeof objects>, spaceId: string, actor: Actor) {
    // read the pages in the background into the rows held, waking whoever waits for the next
    const { context: followed, controller } = context(spaceId, actor);
    const pages = server.sync(spaceId, followed, {
        client: `client-${actor}`,
        queries: { presences: { object: "presence" } },
    });
    const held = new Map<string, Record<string, unknown>>();
    let arrived = 0;
    let wake = () => {};
    const reading = (async () => {
        for await (const page of pages) {
            // start over from a snapshot that resets, then apply the page's changes
            if (page.reset) {
                held.clear();
            }
            for (const change of page.changes) {
                if (change.operation === "delete") {
                    held.delete(change.row.id as string);
                } else {
                    held.set(change.row.id as string, change.row);
                }
            }
            arrived += 1;
            wake();
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
            read += 1;
            while (arrived < read) {
                await new Promise<void>((resolve) => (wake = resolve));
            }

            return [...held.values()];
        },
    };
}

/** Build an actor's request context in a space with its own cancellation. */
function context(spaceId: string, actor: Actor) {
    const controller = new AbortController();
    onTestFinished(() => controller.abort());

    return {
        controller,
        context: {
            scope: spaceId,
            caller: { id: actor },
            requireCaller: () => ({ id: actor }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
            signal: controller.signal,
        } as unknown as ServiceContext,
    };
}
