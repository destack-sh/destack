import {
    keySubject,
    type ObjectReference,
    GLOBAL_SCOPE,
    principal,
    type Subject,
    subjectKey,
} from "@destack/access";
import {
    and,
    eq,
    inArray,
    isNotNull,
    isNull,
    or,
    TABLE,
    type DatabaseConnection,
    type Table,
} from "@destack/db";
import { Snapshot } from "@destack/db/log";
import { Condition } from "@destack/db/query";
import type { Call, ObjectType } from "@destack/object";
import { type ObjectServer, SystemCall } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import type { Controller } from "@destack/service/control";
import { RetryPolicy } from "@destack/service/timer";
import type { SettingContext } from "@destack/setting";
import { member, type Member } from "@destack/space/object";
import { type Decision, decide } from "../delivery/decide.ts";
import type { Outcome } from "../delivery/outcome.ts";
import type { MailTransport } from "../mail/transport.ts";
import type { Content, Notification } from "../notification/notification.ts";
import { announcement, type AnnouncementRow } from "../object/announcement.ts";
import {
    delivery,
    type DeliveryChannel,
    type DeliveryError,
    type DeliveryRow,
    type SkipReason,
} from "../object/delivery.ts";
import type { PushEndpointRow } from "../object/endpoint.ts";
import { notification, type NotificationRow } from "../object/notification.ts";
import { type Reason, Subscription } from "../object/subscription.ts";
import { focus, summary } from "../preference/preference.ts";
import type { InterruptionLevel } from "../preference/preference.ts";
import { PushEncryption } from "../push/encryption.ts";
import type { PushTransport, Urgency } from "../push/transport.ts";

/** The recipients one announcement batch notifies, as many as one notify call names. */
const BATCH = 100;

/** The keys reconciled at once: 16 in flight at 50 to 200 ms send 80 to 320 pushes a second. */
const CONCURRENCY = 16;

/** How long a push service keeps a message for an offline browser, in seconds: one day. */
const PUSH_TTL = 24 * 60 * 60;

/** How refused sends retry: 5 s doubling to 15 min, 8 attempts over about an hour. */
const RETRY = RetryPolicy.of({
    initialInterval: 5_000,
    maximumInterval: 15 * 60_000,
    maximumAttempts: 8,
    jitter: "full",
});

/** How urgently each level's push goes out, as RFC 8030's urgencies. */
const URGENCIES: Readonly<Record<InterruptionLevel, Urgency>> = {
    passive: "very-low",
    active: "normal",
    timeSensitive: "high",
    critical: "high",
};

/** The parts of an object server the dispatcher uses. */
type Serving = Pick<ObjectServer, "objects" | "database" | "executeAsSystem">;

/** Where a recipient receives notifications beyond the inbox. */
export interface Addresses {
    /** The push endpoints of the recipient's browsers. */
    readonly endpoints: readonly Pick<PushEndpointRow, "id" | "url" | "keys" | "device">[];
    /** The recipient's verified email address, absent without one. */
    readonly email?: string;
}

/** The recipients' settings, addresses and presence, as the host reaches them. */
export interface Recipients {
    /** Resolve a recipient's settings where a package's notification arrives in a space. */
    settings(
        recipient: Subject,
        selection: { readonly space: string; readonly package: PackageId },
    ): SettingContext;
    /** Read where a recipient receives notifications beyond the inbox. */
    addresses(recipient: Subject): Promise<Addresses>;
    /** Read the devices a recipient is active on now, as their presence shows. */
    devices(recipient: Subject): Promise<readonly string[]>;
    /** Read the recipient's IANA time zone. */
    timeZone(recipient: Subject): Promise<string>;
    /** Forget a push endpoint its push service no longer knows. */
    forget(recipient: Subject, endpoint: string): Promise<void>;
}

/** What one attempt makes of one delivery, as its send commits it. */
interface Attempt {
    /** The delivery. */
    readonly id: string;
    /** Where it stands after the attempt. */
    readonly state: DeliveryRow["state"];
    /** When it is due next, for deliveries left pending. */
    readonly dueAt?: number;
    /** Whether it waits for a summary, for deliveries left pending. */
    readonly isSummarized?: boolean;
    /** The refusals so far. */
    readonly attempts?: number;
    /** Why it was skipped. */
    readonly reason?: SkipReason;
    /** Why the channel refused it. */
    readonly error?: DeliveryError;
}

/** One pending delivery a send decides. */
interface Candidate {
    /** The delivery. */
    readonly row: DeliveryRow;
    /** Its notification. */
    readonly notification: NotificationRow;
    /** The notification's declaration. */
    readonly declaration: Notification;
    /** What the channel does with it now. */
    readonly decision: Decision;
}

/** Deliver a space's notifications over push and email. */
export class Dispatcher {
    /** The declarations served, which render and decide the notifications. */
    readonly notifications: readonly Notification[];
    /** The recipients' personal scopes. */
    readonly recipients: Recipients;
    /** The push transport: direct with the platform's keys, or through the regional service. */
    readonly push: PushTransport;
    /** The mail transport. */
    readonly mail: MailTransport;
    /** The recipients one announcement batch notifies. */
    readonly batch: number;
    /** The notification types with their system methods. */
    readonly objects: {
        readonly notification: typeof notification;
        readonly announcement: typeof announcement;
        readonly delivery: typeof delivery;
    };

    /** Deliver the declared notifications over the given transports. */
    constructor(options: {
        /** The declarations served. */
        readonly notifications: readonly Notification[];
        /** The recipients' personal scopes. */
        readonly recipients: Recipients;
        /** The push transport. */
        readonly push: PushTransport;
        /** The mail transport. */
        readonly mail: MailTransport;
        /** The recipients one announcement batch notifies, 100 by default. */
        readonly batch?: number;
    }) {
        // retain the declarations and the host's transports
        this.notifications = options.notifications;
        this.recipients = options.recipients;
        this.push = options.push;
        this.mail = options.mail;
        this.batch = options.batch ?? BATCH;

        // implement the system methods
        this.objects = {
            notification: notification.handle({
                deliver: {
                    prepare: (call) => this.#addresses(call),
                    effect: (call) => this.#plan(call),
                },
            }),
            delivery: delivery.handle({
                send: {
                    prepare: (call) => this.#attempt(call),
                    // key each attempt by the delivery and its refusal count
                    key: (call) => `${String(call.target!.id)}/${String(call.target!.attempts)}`,
                    effect: (call) => this.#record(call),
                },
            }),
            announcement: announcement.handle({ expand: (call) => this.#expand(call) }),
        };
    }

    /** Run the space's deliveries as one controller. */
    controller(server: Serving, options: { readonly now?: () => number } = {}): Controller {
        const now = options.now ?? Date.now;

        return {
            name: "notification",
            watches: [notification.table, delivery.table, announcement.table] as Table[],
            concurrency: CONCURRENCY,
            keys: (change) => {
                // reconcile source rows, never home copies
                const row = change.after as Record<string, unknown> | undefined;

                return row === undefined || (row.origin ?? null) !== null
                    ? []
                    : [key(change.table, row)];
            },
            list: () => this.#list(server.database),
            reconcile: (named) => this.#reconcile(server, named, now()),
        };
    }

    /** Find the declaration of a notification or announcement row. */
    declaration(row: Pick<NotificationRow, "packageId" | "name">): Notification {
        const found = this.notifications.find((declared) => declared.is(row));
        if (found === undefined) {
            throw new TypeError(`notification ${row.packageId}/${row.name} is not served`);
        }

        return found;
    }

    /** List every key with work waiting. */
    async #list(database: DatabaseConnection): Promise<readonly string[]> {
        // read the unread notifications left to plan, and the snoozed ones
        const notifications = await database
            .select()
            .from(notification.table)
            .where(
                and(
                    isNull(notification.table.readAt),
                    isNull(notification.table.origin),
                    or(
                        isNull(notification.table.plannedAt),
                        isNotNull(notification.table.snoozedUntil),
                    ),
                ),
            );

        // read the pending deliveries and the announcements with members left
        const deliveries = await database
            .select()
            .from(delivery.table)
            .where(eq(delivery.table.state, "pending"));
        const announcements = await database
            .select()
            .from(announcement.table)
            .where(isNull(announcement.table.expandedAt));

        return [
            ...new Set([
                ...notifications.map((row) => key(notification.table, row)),
                ...deliveries.map((row) => key(delivery.table, row)),
                ...announcements.map((row) => key(announcement.table, row)),
            ]),
        ];
    }

    /** Reconcile one key. */
    async #reconcile(server: Serving, named: string, now: number): Promise<number | undefined> {
        const [kind, ...parts] = named.split(" ");

        // plan an unread notification's deliveries, again once its snooze ends
        if (kind === "notification") {
            const [row] = await server.database
                .select()
                .from(notification.table)
                .where(eq(notification.table.id, parts[0]! as never));
            if (row === undefined || row.readAt !== null || row.origin !== null) {
                return undefined;
            } else if (row.snoozedUntil !== null && row.snoozedUntil > now) {
                return row.snoozedUntil - now;
            } else if (row.plannedAt !== null && row.snoozedUntil === null) {
                return undefined;
            }
            await server.executeAsSystem(
                served(server, notification),
                "deliver",
                [SystemCall.of(row)],
                now,
            );

            return undefined;
        }
        // send a due delivery
        else if (kind === "delivery") {
            const [row] = await server.database
                .select()
                .from(delivery.table)
                .where(eq(delivery.table.id, parts[0]! as never));
            if (row === undefined || row.state !== "pending" || row.isSummarized) {
                return undefined;
            } else if (row.dueAt > now) {
                return row.dueAt - now;
            }
            await server.executeAsSystem(
                served(server, delivery),
                "send",
                [SystemCall.of(row)],
                now,
            );

            return undefined;
        }
        // send a due summary once per recipient and address
        else if (kind === "summary") {
            const [scope, channel, due] = parts as [string, DeliveryChannel, string];
            const dueAt = Number(due);
            if (dueAt > now) {
                return dueAt - now;
            }
            const rows = await server.database
                .select({ row: delivery.table, recipient: notification.table.recipient })
                .from(delivery.table)
                .innerJoin(notification.table, eq(notification.table.id, delivery.table.parentId))
                .where(
                    and(
                        eq(delivery.table.scope, scope as never),
                        eq(delivery.table.channel, channel),
                        eq(delivery.table.state, "pending"),
                        eq(delivery.table.isSummarized, true),
                        eq(delivery.table.dueAt, dueAt),
                    ),
                )
                .orderBy(delivery.table.id);
            const leads = new Map(
                rows.map((entry) => [`${entry.recipient} ${entry.row.endpoint}`, entry.row]),
            );
            for (const lead of leads.values()) {
                await server.executeAsSystem(
                    served(server, delivery),
                    "send",
                    [SystemCall.of(lead)],
                    now,
                );
            }

            return undefined;
        }
        // expand an announcement's next batch
        else if (kind === "announcement") {
            const [row] = await server.database
                .select()
                .from(announcement.table)
                .where(eq(announcement.table.id, parts[0]! as never));
            if (row === undefined || row.expandedAt !== null) {
                return undefined;
            }
            await server.executeAsSystem(
                served(server, announcement),
                "expand",
                [SystemCall.of(row)],
                now,
            );

            return undefined;
        }
        // refuse a key no change names
        else {
            throw new TypeError(`notification dispatcher has no key ${named}`);
        }
    }

    /** Read the recipient's push endpoints before planning. */
    async #addresses(call: Call): Promise<readonly string[]> {
        const row = call.target as NotificationRow;
        const addresses = await this.recipients.addresses(keySubject(row.recipient));

        return addresses.endpoints.map((endpoint) => endpoint.id);
    }

    /** Plan a notification's deliveries: one per push endpoint and one email. */
    async #plan(call: Call): Promise<unknown> {
        // discard the deliveries of an earlier occurrence
        const row = call.target as NotificationRow;
        const endpoints = call.prepared as readonly string[];
        const earlier = await call.database
            .select({ id: delivery.table.id })
            .from(delivery.table)
            .where(eq(delivery.table.parentId, row.id));
        for (const each of earlier) {
            await call.invoke(delivery, "discard", { id: each.id });
        }

        // schedule every endpoint and email, each deciding when it is due
        const targets = [
            ...endpoints.map((endpoint) => ({ channel: "push", endpoint })),
            { channel: "email" },
        ];
        for (const target of targets) {
            await call.invoke(delivery, "schedule", {
                parentId: row.id,
                ...target,
                state: "pending",
                dueAt: call.now,
            });
        }

        return call.revise({ plannedAt: call.now, snoozedUntil: null });
    }

    /** Decide and send a due delivery or summary before recording it. */
    async #attempt(call: Call): Promise<readonly Attempt[]> {
        // read the delivery and its summary's siblings
        const lead = call.target as DeliveryRow;
        const [first] = await call.database
            .select()
            .from(notification.table)
            .where(eq(notification.table.id, lead.parentId));
        const recipient = keySubject(first!.recipient);
        const rows = lead.isSummarized
            ? await this.#siblings(call, lead, first!.recipient)
            : [lead];
        const notifications = await call.database
            .select()
            .from(notification.table)
            .where(
                inArray(
                    notification.table.id,
                    rows.map((row) => row.parentId),
                ),
            );

        // decide each delivery
        const addresses = await this.recipients.addresses(recipient);
        const endpoint = addresses.endpoints.find((known) => known.id === lead.endpoint);
        const devices = lead.channel === "push" ? await this.recipients.devices(recipient) : [];
        const timeZone = await this.recipients.timeZone(recipient);
        const candidates: Candidate[] = [];
        for (const row of rows) {
            const held = notifications.find((known) => known.id === row.parentId)!;
            const declaration = this.declaration(held);
            const settings = await this.recipients
                .settings(recipient, { space: row.scope, package: declaration.package.id })
                .resolve({
                    preference: declaration.preference,
                    focus,
                    summary,
                });
            const decision = decide({
                channel: row.channel,
                notification: held,
                interruption: declaration.definition.interruption,
                preference: settings.preference.value,
                focus: settings.focus.value,
                summary: settings.summary.value,
                timeZone,
                isReadable: await isReadable(call, held, recipient),
                isAddressed:
                    row.channel === "push" ? endpoint !== undefined : addresses.email !== undefined,
                isPresent: devices.some((device) => device !== endpoint?.device),
                isSummaryDue: row.isSummarized && row.dueAt <= call.now,
                now: call.now,
            });
            candidates.push({ row, notification: held, declaration, decision });
        }

        // record what waits or never goes out, and send the rest as one message
        const attempts: Attempt[] = [];
        for (const { row, decision } of candidates) {
            if (decision.action === "skip") {
                attempts.push({ id: row.id, state: "skipped", reason: decision.reason });
            } else if (decision.action === "defer") {
                const { until: dueAt, isSummarized } = decision;
                attempts.push({ id: row.id, state: "pending", dueAt, isSummarized });
            }
        }
        const sending = candidates.filter((candidate) => candidate.decision.action === "send");
        if (sending.length === 0) {
            return attempts;
        }
        const outcome = await this.#send(call, lead, sending, addresses, endpoint);

        // forget an endpoint its push service forgot
        if (outcome.outcome === "gone" && lead.endpoint !== null) {
            await this.recipients.forget(recipient, lead.endpoint);
        }

        return [...attempts, ...sending.map(({ row }) => settled(row, outcome, call.now))];
    }

    /** Record an attempt on the deliveries still pending. */
    async #record(call: Call): Promise<unknown> {
        // read which deliveries are still pending
        const attempts = call.prepared as readonly Attempt[];
        const pending = await call.database
            .select({ id: delivery.table.id })
            .from(delivery.table)
            .where(
                and(
                    inArray(
                        delivery.table.id,
                        attempts.map((attempt) => attempt.id as never),
                    ),
                    eq(delivery.table.state, "pending"),
                ),
            );

        // record each pending one's attempt
        for (const attempt of attempts.filter((each) =>
            pending.some((row) => row.id === each.id),
        )) {
            await call.invoke(delivery, "record", {
                ...attempt,
                ...(attempt.state === "sent" ? { sentAt: call.now } : {}),
            });
        }

        return {};
    }

    /** Read the pending deliveries sent in one summary. */
    async #siblings(
        call: Call,
        lead: DeliveryRow,
        recipient: string,
    ): Promise<readonly DeliveryRow[]> {
        const rows = await call.database
            .select({ row: delivery.table })
            .from(delivery.table)
            .innerJoin(notification.table, eq(notification.table.id, delivery.table.parentId))
            .where(
                and(
                    eq(delivery.table.scope, lead.scope),
                    eq(delivery.table.channel, lead.channel),
                    lead.endpoint === null
                        ? isNull(delivery.table.endpoint)
                        : eq(delivery.table.endpoint, lead.endpoint),
                    eq(delivery.table.state, "pending"),
                    eq(delivery.table.isSummarized, true),
                    eq(delivery.table.dueAt, lead.dueAt),
                    eq(notification.table.recipient, recipient),
                ),
            )
            .orderBy(delivery.table.id);

        return rows.map((entry) => entry.row);
    }

    /** Send the deliveries going out as one message. */
    async #send(
        call: Call,
        lead: DeliveryRow,
        sending: readonly Candidate[],
        addresses: Addresses,
        endpoint: Addresses["endpoints"][number] | undefined,
    ): Promise<Outcome> {
        // render one notification, or a summary line per declaration and thread
        const [only] = sending;
        const content: Content =
            sending.length === 1 && !lead.isSummarized
                ? only!.declaration.render(only!.notification)
                : { title: "Scheduled summary", body: summarize(sending).join("\n") };
        const level = lead.isSummarized ? "passive" : only!.declaration.definition.interruption;

        // email the recipient's address, keyed by the call
        if (lead.channel === "email") {
            return this.mail.send({
                to: addresses.email!,
                subject: content.title,
                text: [content.subtitle, content.body]
                    .filter((line) => line !== undefined)
                    .join("\n\n"),
                key: call.key!,
            });
        }

        // push the encrypted message under its topic
        const message = {
            notification: lead.isSummarized ? undefined : only!.notification.id,
            scope: lead.scope,
            content,
            actions: lead.isSummarized
                ? []
                : Object.entries(only!.declaration.definition.actions ?? {}).map(
                      ([name, action]) => ({
                          name,
                          title: action.title,
                          isDestructive: action.isDestructive === true,
                          hasText: action.text !== undefined,
                      }),
                  ),
        };
        const body = await PushEncryption.encrypt(
            endpoint!.keys,
            new TextEncoder().encode(JSON.stringify(message)),
        );

        return this.push.send({
            url: endpoint!.url,
            body,
            topic: topic(lead.isSummarized ? lead.id : only!.notification.id),
            urgency: URGENCIES[level],
            ttl: PUSH_TTL,
        });
    }

    /** Expand an announcement's next batch into notifications. */
    async #expand(call: Call): Promise<unknown> {
        // read the next batch of the audience after the cursor
        const row = call.target as AnnouncementRow;
        const source: ObjectReference = {
            packageId: row.parentPackageId!,
            type: row.parentType!,
            scope: row.scope,
            id: row.parentId!,
        };
        const page = { ...(row.cursor === null ? {} : { after: row.cursor }), limit: this.batch };

        // read the next batch of the audience
        const target = row.audience;
        const audience: readonly {
            readonly id: string;
            readonly owner: Subject;
            readonly reason: Reason;
        }[] =
            target.kind === "subscribers"
                ? await Subscription.list(call, source, { ...page, until: row.updatedAt })
                : (target.kind === "members"
                      ? await members(call, page)
                      : await holders(call, source, target.permission, page)
                  ).map((entry) => ({ ...entry, reason: target.reason }));

        // skip the author and excluded principals
        const skipped = new Set([row.author, ...row.excluded]);
        const recipients = audience.filter((entry) => !skipped.has(subjectKey(entry.owner)));

        // notify each reason's recipients together
        const reasons = new Map<Reason, Subject[]>();
        for (const entry of recipients) {
            const owners = reasons.get(entry.reason) ?? [];
            owners.push(entry.owner);
            reasons.set(entry.reason, owners);
        }
        for (const [reason, owners] of reasons) {
            await this.declaration(row).notify(call, {
                source,
                recipients: owners,
                reason,
                payload: row.payload,
                ...(row.key === null ? {} : { key: row.key }),
                thread: row.thread,
            });
        }

        // advance the cursor, finishing with a batch short of full
        return call.revise({
            cursor: audience.at(-1)?.id ?? row.cursor,
            expandedAt: audience.length < this.batch ? call.now : null,
        });
    }
}

/** Name the key a row reconciles under. */
function key(table: Table, row: Record<string, unknown>): string {
    // name summarized deliveries by their summary
    if (table === (delivery.table as Table)) {
        return row.isSummarized === true
            ? `summary ${String(row.scope)} ${String(row.channel)} ${String(row.dueAt)}`
            : `delivery ${String(row.id)}`;
    }
    // name announcements and notifications by their identifier
    else if (table === (announcement.table as Table)) {
        return `announcement ${String(row.id)}`;
    } else if (table === (notification.table as Table)) {
        return `notification ${String(row.id)}`;
    }
    // refuse tables the controller does not watch
    else {
        throw new TypeError(`notification dispatcher watches no table ${table[TABLE].name}`);
    }
}

/** Find the served copy of a type, which carries the system methods. */
function served<Type extends ObjectType>(server: Serving, type: Type): Type {
    const found = server.objects.find((object) => type.same(object));
    if (found === undefined) {
        throw new TypeError(`object server serves no ${type.name} of the dispatcher`);
    }

    return found as Type;
}

/** Decide whether a recipient may read a notification. */
async function isReadable(call: Call, row: NotificationRow, recipient: Subject): Promise<boolean> {
    // resolve the recipient's access and check read
    const authorizer = call.authorization!.authorizer;
    const snapshot = Snapshot.live(call.database);
    const access = await authorizer.resolve(snapshot, row.scope, {
        subjects: [recipient],
        now: call.now,
        attributes: {},
    });
    const decision = await authorizer.check(
        snapshot,
        notification.permission("read"),
        notification.reference(row.scope, row.id),
        access,
    );

    return decision.isAllowed;
}

/** Read what a channel's outcome makes of a delivery sent with it. */
function settled(row: DeliveryRow, outcome: Outcome, now: number): Attempt {
    // count refusals, retrying within the policy
    const attempts = row.attempts + 1;
    if (outcome.outcome === "sent") {
        return { id: row.id, state: "sent" };
    } else if (outcome.outcome === "gone") {
        return { id: row.id, state: "skipped", reason: "gone" };
    } else if (outcome.outcome === "retry" && RetryPolicy.isRetried(RETRY, attempts)) {
        const wait = outcome.after ?? RetryPolicy.interval(RETRY, attempts);

        return { id: row.id, state: "pending", dueAt: now + wait, attempts, error: outcome.error };
    }

    return { id: row.id, state: "failed", attempts, error: outcome.error };
}

/** Summarize notifications, one line per declaration and thread. */
function summarize(sending: readonly Candidate[]): readonly string[] {
    const groups = new Map<string, { declaration: Notification; count: number }>();
    for (const { declaration, notification: held } of sending) {
        const group = `${held.packageId} ${held.name} ${held.thread}`;
        const known = groups.get(group) ?? { declaration, count: 0 };
        groups.set(group, { declaration, count: known.count + held.count });
    }

    return [...groups.values()].map((group) => group.declaration.definition.summary(group.count));
}

/** Read a page of the space's members. */
async function members(
    call: Call,
    page: { readonly after?: string; readonly limit: number },
): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
    const listed = (await call.invoke(member, "list", {
        ...(page.after === undefined ? {} : { where: Condition.gt("id", page.after) }),
        order: [{ column: "id", direction: "asc" }],
        limit: page.limit,
    })) as { readonly items: readonly Member[] };

    return listed.items.map((row) => ({
        id: row.id,
        owner: principal.user.reference(GLOBAL_SCOPE, row.userId),
    }));
}

/** Read a page of the users holding a permission on an object. */
async function holders(
    call: Call,
    source: ObjectReference,
    permission: string,
    page: { readonly after?: string; readonly limit: number },
): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
    // list the user principals holding the permission
    const authorizer = call.authorization!.authorizer;
    const user = { packageId: principal.user.definition.packageId, type: principal.user.name };
    const subjects = await authorizer.subjects(
        Snapshot.live(call.database),
        authorizer.policy(source).permission(permission),
        source,
        user,
        call.now,
        page,
    );

    return subjects.map((owner) => ({ id: subjectKey(owner), owner }));
}

/** Read a push topic from an identifier's 32 hexadecimal digits, as RFC 8030 allows. */
function topic(id: string): string {
    return schema
        .string()
        .length(32)
        .parse(id.slice(id.indexOf("-") + 1).replaceAll("-", ""));
}
