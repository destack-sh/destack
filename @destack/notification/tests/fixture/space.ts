import type { notification } from "../../src/object/notification.ts";
import type { InstanceOf } from "@destack/object";
import { reconciliation, testCallKey } from "@destack/service/test";
import { accessRelationship, accessRole, anyone, Relationship, Role } from "@destack/access";
import type { PackageId } from "@destack/package";
import { Scope, Subject } from "@destack/sync";
import { copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import { journal } from "@destack/audit";
import { type DatabaseConnection, type Dialect, eq, Key, TABLE, type Table } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TestDatabase } from "@destack/db/test";
import type { ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import { type Identifier, identifier, type schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { subjectContext } from "@destack/service/test";
import type { Setting } from "@destack/setting";
import type { SettingValue } from "@destack/setting/object";
import { user } from "@destack/account/object";
import { space } from "@destack/space/object";
import { replicaRow, replicaTables } from "@destack/sync";
import type * as sync from "@destack/sync";
import { afterAll, onTestFinished } from "@destack/test";
import { v7 } from "uuid";
import { type Badge, type Outcome, subscription, UNREAD } from "../../src/index.ts";
import {
    type Contact,
    type Mail,
    type PushRequest,
    NotificationServer,
} from "../../src/server/index.ts";
import { type Actor, actors } from "./actor.ts";
import { document, notifications } from "./document.ts";

export { type Actor, actors } from "./actor.ts";

/** The start of every scenario: Monday 28 September 2026, 10:00 UTC, noon in Vienna. */
export const MONDAY = Date.UTC(2026, 8, 28, 10, 0);

/** The durable database of each dialect, shared by the scenarios of one test file. */
const databases = new Map<Dialect, Promise<TestDatabase>>();
afterAll(async () => {
    for (const storage of databases.values()) {
        await (await storage).close();
    }
});

/** The recipients' personal scopes: settings, push endpoints, email addresses and active devices, kept in memory. */
export class MemoryDirectory {
    /** The setting values each recipient set, by subject key. */
    readonly values = new Map<string, SettingValue[]>();
    /** The devices whose desktops show each recipient's banners, by subject key. */
    readonly desktops = new Map<string, readonly string[]>();
    /** The push endpoints of each recipient, by subject key. */
    readonly endpoints = new Map<string, Contact["endpoints"][number][]>();
    /** The verified email address of each recipient, by subject key. */
    readonly emails = new Map<string, string>();
    /** The time zone each recipient's profile names, by subject key, UTC where absent. */
    readonly zones = new Map<string, string>();
    /** The devices each recipient is active on, by subject key. */
    readonly active = new Map<string, readonly string[]>();
    /** The endpoints forgotten, by identifier. */
    readonly forgotten: string[] = [];

    /** Read the time zone a recipient's profile names, UTC for one naming none. */
    async timeZone(recipient: Subject): Promise<string> {
        return this.zones.get(Subject.key(recipient)) ?? "UTC";
    }

    /** Set a recipient's value of a setting, in their scope or refined to a space. */
    set<Value extends schema.Schema>(
        actor: Actor,
        setting: Setting<Value>,
        value: schema.Infer<Value>,
        refinement: { readonly space?: string } = {},
    ): void {
        const values = this.values.get(Subject.key(actors[actor])) ?? [];
        values.push({
            id: identifier("setting").parse(`setting-${v7()}`),
            createdAt: 0,
            createdBy: Subject.key(actors[actor]),
            updatedAt: 0,
            updatedBy: Subject.key(actors[actor]),
            revision: 1,
            tags: {},
            scope: actors[actor].id,
            managerInstallationId: null,
            managerPackageId: null,
            managerName: null,
            detachedAt: null,
            packageId: setting.reference.packageId,
            name: setting.reference.name,
            package: null,
            space: (refinement.space ?? null) as never,
            installation: null,
            device: null,
            mode: "set",
            value: value as never,
            release: setting.package.version,
        });
        this.values.set(Subject.key(actors[actor]), values);
    }

    /** Read the values a recipient placed. */
    async settings(recipient: Subject): Promise<readonly SettingValue[]> {
        return this.values.get(Subject.key(recipient)) ?? [];
    }

    /** Read a recipient's desktops, push endpoints and email address. */
    async contact(recipient: Subject): Promise<Contact> {
        const email = this.emails.get(Subject.key(recipient));

        return {
            desktops: this.desktops.get(Subject.key(recipient)) ?? [],
            endpoints: this.endpoints.get(Subject.key(recipient)) ?? [],
            ...(email === undefined ? {} : { email }),
        };
    }

    /** Read the devices a recipient is active on. */
    async devices(recipient: Subject): Promise<readonly string[]> {
        return this.active.get(Subject.key(recipient)) ?? [];
    }

    /** Forget a push endpoint. */
    async forget(recipient: Subject, endpoint: string): Promise<void> {
        const known = this.endpoints.get(Subject.key(recipient)) ?? [];
        this.endpoints.set(
            Subject.key(recipient),
            known.filter((entry) => entry.id !== endpoint),
        );
        this.forgotten.push(endpoint);
    }
}

/** A push transport keeping the requests it was handed, answering each with the next scripted outcome or as sent. */
export class MemoryPush {
    /** The requests handed over, in order. */
    readonly requests: PushRequest[] = [];
    /** The outcomes the next requests get, in order. */
    readonly outcomes: Outcome[] = [];

    /** Keep a request and answer it. */
    async send(request: PushRequest): Promise<Outcome> {
        this.requests.push(request);

        return this.outcomes.shift() ?? { outcome: "sent" };
    }
}

/** A mail transport keeping the emails it was handed. */
export class MemoryMail {
    /** The emails handed over, in order. */
    readonly sent: Mail[] = [];

    /** Keep an email as sent. */
    async send(mail: Mail): Promise<Outcome> {
        this.sent.push(mail);

        return { outcome: "sent" };
    }
}

/** The notification objects the dispatch controllers run, by name. */
const DISPATCHED = new Set(["notification", "delivery", "announcement"]);

/** Serve documents and their notifications in a new space, acting as one actor at a time, at a clock the scenario moves. */
export async function serveSpace(dialect: Dialect, options: { readonly batch?: number } = {}) {
    // keep the space in the dialect's shared database, and start the clock on Monday
    const storage = await durable(dialect);
    let now = MONDAY;
    const spaceId = await openSpace(storage.database, now);
    const clock = () => now;

    // deliver over in-memory homes and transports
    const homes = new MemoryDirectory();
    const pushes = new MemoryPush();
    const mails = new MemoryMail();
    const objects = new NotificationServer({
        notifications,
        recipients: homes,
        push: pushes,
        mail: mails,
        ...(options.batch === undefined ? {} : { batch: options.batch }),
    }).objects();

    // decide every call as the current actor, at the clock's time
    let current: Actor = "alice";
    const server = new ObjectServer({
        objects: { document, subscription, user, ...objects },
        database: storage.database,
        clock,
        context: (context) => ({
            subjects: [context.requireAuthentication().claims.subject],
            now,
            attributes: {},
        }),
        callKey: testCallKey,
        origin: { package: document.package, service: "notification" },
    });
    const controllers = server.controllers().filter((each) => DISPATCHED.has(each.name));

    return {
        server,
        spaceId,
        homes,
        pushes,
        mails,
        objects,
        /** Call a method as the current actor, replayable by request. */
        call: async (name: string, input: object, object: ObjectType = document) =>
            (await server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context(spaceId, current).context,
            )) as Record<string, unknown> & { readonly id: string },
        /** Execute one mutation of several calls as the current actor. */
        mutate: (calls: readonly sync.Call[]) =>
            server.mutate(
                { id: RequestId.create(), calls: [...calls] },
                context(spaceId, current).context,
            ),
        /** Act as another actor. */
        as: (actor: Actor) => {
            current = actor;
        },
        /** Move the clock by some minutes. */
        wait: (minutes: number) => {
            now += minutes * 60_000;
        },
        /** Read the clock. */
        now: clock,
        /** Follow an actor's inbox as its home copies the space's notifications naming the actor. */
        inbox: (actor: Actor) => new SpaceInbox(server, spaceId, actor),
        /** Copy a user who joined the space, as its cell keeps the users it follows. */
        join: async (actor: Actor) => {
            const id = actors[actor].id;
            await storage.database.insert(user.table).values({
                id,
                scope: Scope.universe.id,
                name: actor,
                createdAt: now,
                updatedAt: now,
            } as never);
            await storage.database.insert(replicaRow).values({
                name: spaceId,
                scope: Scope.universe.id,
                table: user.table[TABLE].sqlName,
                key: Key.name(user.table as Table, { id }),
                concealed: [],
            });
        },
        /** Reconcile every key the notification controllers list until a round changes nothing. */
        dispatch: async () => {
            for (let round = 0; round < DISPATCH_ROUNDS; round++) {
                const before = (await storage.database.log.position()).sequence;
                for (const controller of controllers) {
                    for (const key of await controller.list()) {
                        await controller.reconcile(
                            key,
                            reconciliation(new AbortController().signal),
                        );
                    }
                }
                if ((await storage.database.log.position()).sequence === before) {
                    return;
                }
            }
            throw new TypeError("the notification controllers keep changing the space");
        },
        /** Read the space's notifications as the system stores them, by recipient and creation order. */
        notifications: async () => {
            const table = objects.notification.table;
            const rows = await storage.database
                .select()
                .from(table)
                .where(eq(table.scope, spaceId));

            return rows.toSorted(
                (left, right) =>
                    left.recipient.localeCompare(right.recipient) ||
                    left.createdAt - right.createdAt ||
                    left.id.localeCompare(right.id),
            );
        },
        /** Refer to a document as its attachments' parent. */
        host: (id: string) => ({
            parent: { packageId: document.policy.definition.packageId, type: document.name, id },
        }),
        /** Read the deliveries of a notification, by channel and endpoint. */
        deliveries: async (id: string) => {
            const table = objects.delivery.table;
            const rows = await storage.database
                .select()
                .from(table)
                .where(eq(table.parentId, id as never));

            return rows
                .map((row) => ({
                    channel: row.channel,
                    endpoint: row.endpoint,
                    device: row.device,
                    banner: row.banner,
                    state: row.state,
                    dueAt: row.dueAt,
                    isSummarized: row.isSummarized,
                    reason: row.reason,
                }))
                .toSorted((left, right) => left.channel.localeCompare(right.channel));
        },
    };
}

/** The rounds a dispatch takes at most: planning, sending, retries and batches settle well within them. */
const DISPATCH_ROUNDS = 20;

/** An actor's inbox over one space: its notifications and their live unread counts. */
export class SpaceInbox {
    /** The rows kept, by identifier. */
    readonly rows = new Map<string, InstanceOf<typeof notification>>();
    /** The unread counts kept, by app. */
    readonly counts = new Map<string, Badge>();
    /** The pages read so far. */
    #pages = 0;
    /** Wake whoever waits for the next page. */
    #wake: () => void = () => {};
    /** The space followed. */
    readonly #space: string;

    /** Follow a space's notifications as an actor, until the scenario ends. */
    constructor(server: Pick<ObjectServer, "source">, spaceId: string, actor: Actor) {
        // read every page in the background into the rows and counts kept
        this.#space = spaceId;
        const { context: followed, controller } = context(spaceId, actor);
        const pages = server.source.sync(spaceId, followed, {
            queries: {
                notifications: { object: "notification" },
                unread: { object: "notification", ...UNREAD },
            },
        });
        const reading = (async () => {
            for await (const page of pages) {
                this.#apply(page);
                this.#pages += 1;
                this.#wake();
            }
        })();
        onTestFinished(async () => {
            // end the stream before its database closes
            controller.abort();
            await reading;
        });
    }

    /** Follow the notifications kept, the latest occurrence first. */
    async *notifications(
        signal: AbortSignal,
    ): AsyncIterable<readonly InstanceOf<typeof notification>[]> {
        while (!signal.aborted) {
            yield [...this.rows.values()].toSorted(
                (left, right) => right.occurredAt - left.occurredAt,
            );
            await this.#next(this.#pages + 1);
        }
    }

    /** Follow the unread counts kept. */
    async *badges(signal: AbortSignal): AsyncIterable<readonly Badge[]> {
        while (!signal.aborted) {
            yield [...this.counts.values()];
            await this.#next(this.#pages + 1);
        }
    }

    /** Wait until the kept rows and counts meet a condition and read each arriving page. */
    async until(condition: (inbox: SpaceInbox) => boolean): Promise<void> {
        while (!condition(this)) {
            await this.#next(this.#pages + 1);
        }
    }

    /** Read the unread count across apps, as a dock badge shows it. */
    get unread(): number {
        return [...this.counts.values()].reduce((total, badge) => total + badge.unread, 0);
    }

    /** Wait for a page. */
    async #next(page: number): Promise<void> {
        while (this.#pages < page) {
            await new Promise<void>((resolve) => (this.#wake = resolve));
        }
    }

    /** Apply a page's rows and aggregate groups. */
    #apply(page: sync.QueryPage): void {
        // start over from a snapshot that resets
        if (page.reset) {
            this.rows.clear();
            this.counts.clear();
        }

        // keep the notification rows entering and changing, and drop those leaving
        for (const change of page.changes) {
            if (change.operation === "delete") {
                this.rows.delete(change.row.id as string);
            } else {
                this.rows.set(
                    change.row.id as string,
                    change.row as unknown as InstanceOf<typeof notification>,
                );
            }
        }

        // keep each app's count, dropping groups that empty
        for (const result of page.results ?? []) {
            if (result.group === null) {
                this.counts.clear();
            } else if (result.values === null) {
                this.counts.delete(String(result.group.parentPackageId));
            } else {
                this.counts.set(String(result.group.parentPackageId), {
                    space: (result.group.origin as string | null) ?? this.#space,
                    packageId: result.group.parentPackageId as PackageId,
                    unread: Number(result.values.unread),
                    latestAt: Number(result.values.latestAt),
                });
            }
        }
    }
}

/** Open a new space below a new account, readable by anyone through a member role, returning its identifier. */
async function openSpace(database: DatabaseConnection, at: number): Promise<Identifier<"space">> {
    // record the account and the space as copies of their home's access has them
    const accountId = identifier("account").parse(`account-${v7()}`);
    const spaceId = identifier("space").parse(`space-${v7()}`);
    const reference = space.reference(accountId, spaceId);
    await copyScope(database, account.reference(Scope.universe.id, accountId));
    await copyScope(database, reference);

    // define a role reading the space and bind it to anyone
    const role = identifier("role").parse(`role-${v7()}`);
    await database.insert(accessRole).values({
        id: role,
        createdAt: at,
        updatedAt: at,
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
                createdAt: at,
                expiresAt: null,
            },
            spaceId,
        ),
    );

    return spaceId;
}

/** Keep the durable tables once per dialect for the scenarios of a file. */
function durable(dialect: Dialect): Promise<TestDatabase> {
    // migrate the dialect's database on first use
    let storage = databases.get(dialect);
    if (storage === undefined) {
        const objects = {
            document,
            subscription,
            user,
            ...new NotificationServer({
                notifications,
                recipients: new MemoryDirectory(),
                push: new MemoryPush(),
                mail: new MemoryMail(),
            }).objects(),
        };
        storage = TestDatabase.create(
            dialect,
            defineDatabase({
                name: "main",
                tables: [
                    journal,
                    ...replicaTables,
                    ...Object.values(objects).flatMap((object) => object.tables),
                ],
            }),
            { isMigrated: true },
        );
        databases.set(dialect, storage);
    }

    return storage;
}

/** Build an actor's request context in a space with its own cancellation. */
function context(spaceId: string, actor: Actor) {
    const controller = new AbortController();
    onTestFinished(() => controller.abort());

    return {
        controller,
        context: subjectContext(actors[actor], spaceId, controller.signal),
    };
}
