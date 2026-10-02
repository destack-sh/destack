import type { InstanceOf } from "@destack/object";
import { principal } from "@destack/access";
import { Duration, identifier, schema } from "@destack/schema";
import { Replica, Scope, type ObjectReference, Subject } from "@destack/sync";
import {
    and,
    asc,
    eq,
    gt,
    inArray,
    isNull,
    type DatabaseConnection,
    type Table,
} from "@destack/db";
import { Snapshot } from "@destack/db/log";
import { Condition } from "@destack/db/query";
import { type Call } from "@destack/object";
import { RetryPolicy } from "@destack/service/timer";

import { user } from "@destack/account/object";
import { decide } from "../delivery/decide.ts";
import type { Outcome } from "../delivery/outcome.ts";
import type { MailTransport } from "../mail/transport.ts";
import type { Content } from "../notification/content.ts";
import type { Notification } from "../notification/notification.ts";
import { announcement, type Announcement } from "../object/announcement.ts";
import { type Banner, delivery, type Delivery } from "../object/delivery.ts";

import { notification } from "../object/notification.ts";
import { type Reason, Subscription } from "../object/subscription.ts";
import { focus, summary } from "../preference/preference.ts";

import { PushEncryption } from "../push/encryption.ts";
import { type PushTransport, URGENCY } from "../push/transport.ts";
import { Attempt, type Candidate } from "../delivery/attempt.ts";
import { Contact, type RecipientDirectory } from "../recipient/directory.ts";

/** The recipients one announcement batch notifies by default, as many as one notify call names. */
const BATCH = 100;

/** The keys reconciled at once by default: 16 in flight at 50 to 200 ms send 80 to 320 pushes a second. */
const CONCURRENCY = 16;

/** How long a push service keeps a message for an offline browser by default: one day. */
const PUSH_TTL: Duration = { days: 1 };

/** How refused sends retry by default: 5 s doubling to 15 min, 8 attempts over about an hour. */
const RETRY = RetryPolicy.of({
    initialInterval: 5_000,
    maximumInterval: 15 * 60_000,
    maximumAttempts: 8,
    jitter: "full",
});

/** The default email delay: a quarter hour, as Slack and Linear wait. */
const EMAIL_DELAY: Duration = { minutes: 15 };

/** The desktops and push endpoints a notification's deliveries go to. */
interface Targets {
    /** The devices whose desktops show banners. */
    readonly desktops: readonly string[];
    /** The push endpoints. */
    readonly endpoints: readonly string[];
}

/** What a host delivers a space's notifications with: the served declarations, its recipients and transports. */
export interface NotificationServerOptions {
    /** The declarations that render and decide the notifications. */
    readonly notifications: readonly Notification[];
    /** The recipients' personal scopes. */
    readonly recipients: RecipientDirectory;
    /** The push transport: direct with the platform's keys, or through the regional service. */
    readonly push: PushTransport;
    /** The mail transport. */
    readonly mail: MailTransport;
    /** The recipients one announcement batch notifies, 100 by default. */
    readonly batch?: number;
    /** The keys each controller reconciles at once, 16 by default. */
    readonly concurrency?: number;
    /** How long a push service keeps a message for an offline browser, a day by default. */
    readonly pushTtl?: Duration;
    /** How refused sends retry, about an hour of eight attempts by default. */
    readonly retry?: RetryPolicy;
    /** How long an email waits for the notification to be read elsewhere, 15 minutes by default. */
    readonly emailDelay?: Duration;
}

/** A host's server of a space's notifications: it plans, sends and expands them under their controllers. */
export class NotificationServer {
    /** The served declarations, the recipients' directory, and the transports. */
    readonly #options: NotificationServerOptions;

    /** Serve notifications with some declarations, recipients and transports. */
    constructor(options: NotificationServerOptions) {
        this.#options = options;
    }

    /** Serve notifications, deliveries and announcements under their controllers. */
    objects() {
        return {
            notification: notification
                .handle({
                    read: async (call, next) => {
                        // read it, and withdraw its banners from the recipient's desktops
                        const read = await next();
                        const banners = await call.database
                            .select({ id: delivery.table.id })
                            .from(delivery.table)
                            .where(
                                and(
                                    eq(delivery.table.parentId, call.target!.id),
                                    eq(delivery.table.channel, "desktop"),
                                ),
                            );
                        for (const banner of banners) {
                            await call.invoke(delivery, "discard", { id: banner.id });
                        }

                        return read;
                    },
                    act: (call) => this.#act(call),
                    deliver: {
                        prepare: (call) => this.#targetsOf(call),
                        effect: (call) => this.#plan(call),
                    },
                })
                .control({
                    pending: Condition.all(
                        Condition.missing("origin"),
                        Condition.missing("readAt"),
                        Condition.any(
                            Condition.missing("plannedAt"),
                            Condition.not(Condition.missing("snoozedUntil")),
                        ),
                    ),
                    concurrency: this.#options.concurrency ?? CONCURRENCY,
                    reconcile: async (reconciliation) => {
                        // wait out a snooze, then plan the notification's deliveries
                        const row = reconciliation.rows[0]!;
                        if (row.snoozedUntil !== null && row.snoozedUntil > reconciliation.now) {
                            return row.snoozedUntil - reconciliation.now;
                        }
                        await reconciliation.execute("deliver", [row]);

                        return undefined;
                    },
                }),
            delivery: delivery
                .handle({
                    send: {
                        prepare: (call) => this.#attempt(call),
                        // key each attempt by the delivery and its refusal count
                        key: (call) =>
                            `${String(call.target!.id)}/${String(call.target!.attempts)}`,
                        effect: (call) => this.#record(call),
                    },
                })
                .control({
                    pending: Condition.eq("state", "pending"),
                    // send a summary's deliveries together, and every other delivery alone
                    key: (row) =>
                        row.isSummarized
                            ? {
                                  scope: row.scope,
                                  channel: row.channel,
                                  dueAt: row.dueAt,
                                  isSummarized: true,
                              }
                            : { id: row.id },
                    concurrency: this.#options.concurrency ?? CONCURRENCY,
                    reconcile: async (reconciliation) => {
                        // wait until the deliveries are due
                        const [first] = reconciliation.rows;
                        if (first!.dueAt > reconciliation.now) {
                            return first!.dueAt - reconciliation.now;
                        }

                        // send a delivery, or a summary once per recipient and address
                        const leads = first!.isSummarized
                            ? await this.#summaryLeads(reconciliation.database, reconciliation.rows)
                            : [first!];
                        for (const lead of leads) {
                            await reconciliation.execute("send", [lead]);
                        }

                        return undefined;
                    },
                }),
            announcement: announcement.handle({ expand: (call) => this.#expand(call) }).control({
                pending: Condition.missing("expandedAt"),
                reconcile: async (reconciliation) => {
                    // expand the announcement's next batch
                    await reconciliation.execute("expand", reconciliation.rows);

                    return undefined;
                },
            }),
        };
    }

    /** Find the served declaration of a notification or announcement row. */
    #declarationOf(row: Pick<InstanceOf<typeof notification>, "packageId" | "name">): Notification {
        const found = this.#options.notifications.find((declared) => declared.is(row));
        if (found === undefined) {
            throw new TypeError(`notification ${row.packageId}/${row.name} is not served`);
        }

        return found;
    }

    /** Pick the summarized delivery leading each recipient and address's summary. */
    async #summaryLeads(
        database: DatabaseConnection,
        rows: readonly Delivery[],
    ): Promise<readonly Delivery[]> {
        // read each delivery's recipient
        const recipients = await database
            .select({ id: notification.table.id, recipient: notification.table.recipient })
            .from(notification.table)
            .where(
                inArray(
                    notification.table.id,
                    rows.map((row) => row.parentId),
                ),
            );

        // keep the last delivery of each recipient and address in identifier order
        const leads = new Map<string, Delivery>();
        for (const row of rows.toSorted((left, right) => left.id.localeCompare(right.id))) {
            const recipient = recipients.find((entry) => entry.id === row.parentId)!.recipient;
            leads.set(`${recipient} ${row.endpoint}`, row);
        }

        return [...leads.values()];
    }

    /** Answer a notification's action as its recipient, then read it. */
    #act(call: Call): Promise<unknown> {
        const { action, text } = call.input as { readonly action: string; readonly text?: string };

        return this.#declarationOf(call.target as InstanceOf<typeof notification>).act(
            call,
            action,
            text,
        );
    }

    /** Read the recipient's desktops and push endpoints before planning. */
    async #targetsOf(call: Call): Promise<Targets> {
        const row = call.target as InstanceOf<typeof notification>;
        const contact = await this.#options.recipients.contact(Subject.read(row.recipient));

        return {
            desktops: contact.desktops,
            endpoints: contact.endpoints.map((endpoint) => endpoint.id),
        };
    }

    /** Plan a notification's deliveries: one per desktop, one per push endpoint and one email. */
    async #plan(call: Call): Promise<unknown> {
        // discard the deliveries of an earlier occurrence
        const row = call.target as InstanceOf<typeof notification>;
        const { desktops, endpoints } = call.prepared as Targets;
        const earlier = await call.database
            .select({ id: delivery.table.id })
            .from(delivery.table)
            .where(eq(delivery.table.parentId, row.id));
        for (const each of earlier) {
            await call.invoke(delivery, "discard", { id: each.id });
        }

        // schedule every desktop, endpoint and email at its own due time
        const targets = [
            ...desktops.map((device) => ({ channel: "desktop", device })),
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

        return call.update({ plannedAt: call.now, snoozedUntil: null });
    }

    /** Decide and send a due delivery or summary before recording it. */
    async #attempt(call: Call): Promise<readonly Attempt[]> {
        // read the delivery and its summary's siblings
        const lead = call.target as Delivery;
        const [first] = await call.database
            .select()
            .from(notification.table)
            .where(eq(notification.table.id, lead.parentId));
        const recipient = Subject.read(first!.recipient);
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
        const contact = await this.#options.recipients.contact(recipient);
        const endpoint = contact.endpoints.find((known) => known.id === lead.endpoint);
        const devices =
            lead.channel === "push" ? await this.#options.recipients.devices(recipient) : [];
        const timeZone = await this.#options.recipients.timeZone(recipient);
        const candidates: Candidate[] = [];
        for (const row of rows) {
            const parent = notifications.find((known) => known.id === row.parentId)!;
            const declaration = this.#declarationOf(parent);
            // resolve the recipient's settings where the package's notification arrives
            const selection = {
                scope: recipient.id,
                space: row.scope as never,
                package: declaration.package.id,
            };
            const placed = await this.#options.recipients.settings(recipient, [
                declaration.preference,
                focus,
                summary,
            ]);
            const chain = (await Scope.chain(Snapshot.live(call.database), row.scope)).map(
                (link) => link.object.id,
            );
            const settings = {
                preference: declaration.preference.resolve(selection, placed, chain),
                focus: focus.resolve(selection, placed, chain),
                summary: summary.resolve(selection, placed, chain),
            };
            const decision = decide({
                channel: row.channel,
                notification: parent,
                interruption: declaration.definition.interruption,
                preference: settings.preference.value,
                focus: settings.focus.value,
                summary: settings.summary.value,
                timeZone,
                isReadable: await this.#isReadable(call, parent, recipient),
                isAddressed: Contact.reaches(contact, row),
                isPresent: devices.some((device) => device !== endpoint?.device),
                isSummaryDue: row.isSummarized && row.dueAt <= call.now,
                emailDelay: Duration.milliseconds(this.#options.emailDelay ?? EMAIL_DELAY),
                now: call.now,
            });
            candidates.push({ row, notification: parent, declaration, decision });
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

        // settle desktop deliveries with the banner the desktop shows
        if (lead.channel === "desktop") {
            return [
                ...attempts,
                ...sending.map(({ row, notification: parent, declaration }) => ({
                    id: row.id,
                    state: "sent" as const,
                    banner: this.#banner(parent, declaration),
                })),
            ];
        }
        const outcome = await this.#send(call, lead, sending, contact, endpoint);

        // forget an endpoint its push service forgot
        if (outcome.outcome === "gone" && lead.endpoint !== null) {
            await this.#options.recipients.forget(recipient, lead.endpoint);
        }

        return [
            ...attempts,
            ...sending.map(({ row }) =>
                Attempt.settled(row, outcome, call.now, this.#options.retry ?? RETRY),
            ),
        ];
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
    async #siblings(call: Call, lead: Delivery, recipient: string): Promise<readonly Delivery[]> {
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
        lead: Delivery,
        sending: readonly Candidate[],
        contact: Contact,
        endpoint: Contact["endpoints"][number] | undefined,
    ): Promise<Outcome> {
        // render one notification, or a summary line per declaration and thread
        const [only] = sending;
        const content: Content =
            sending.length === 1 && !lead.isSummarized
                ? only!.declaration.render(only!.notification)
                : { title: "Scheduled summary", body: this.#summarize(sending).join("\n") };
        const level = lead.isSummarized ? "passive" : only!.declaration.definition.interruption;

        // email the recipient's address, keyed by the call
        if (lead.channel === "email") {
            return this.#options.mail.send({
                to: contact.email!,
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

        return this.#options.push.send({
            url: endpoint!.url,
            body,
            topic: this.#topic(lead.isSummarized ? lead.id : only!.notification.id),
            urgency: URGENCY[level],
            ttl: Duration.milliseconds(this.#options.pushTtl ?? PUSH_TTL) / 1000,
        });
    }

    /** Expand an announcement's next batch into notifications. */
    async #expand(call: Call): Promise<unknown> {
        // read the next batch of the audience after the cursor
        const row = call.target as Announcement;
        const source: ObjectReference = {
            packageId: row.parentPackageId!,
            type: row.parentType!,
            scope: row.scope,
            id: row.parentId!,
        };
        const page = {
            ...(row.cursor === null ? {} : { after: row.cursor }),
            limit: this.#options.batch ?? BATCH,
        };

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
                      ? await this.#members(call, page)
                      : await this.#permitted(call, source, target.permission, page)
                  ).map((entry) => ({ ...entry, reason: target.reason }));

        // skip the author and excluded principals
        const skipped = new Set([row.author, ...row.excluded]);
        const recipients = audience.filter((entry) => !skipped.has(Subject.key(entry.owner)));

        // notify each reason's recipients together
        const reasons = new Map<Reason, Subject[]>();
        for (const entry of recipients) {
            const owners = reasons.get(entry.reason) ?? [];
            owners.push(entry.owner);
            reasons.set(entry.reason, owners);
        }
        for (const [reason, owners] of reasons) {
            await this.#declarationOf(row).notify(call, {
                source,
                recipients: owners,
                reason,
                payload: row.payload,
                ...(row.key === null ? {} : { key: row.key }),
                thread: row.thread,
            });
        }

        // advance the cursor, finishing with a batch short of full
        return call.update({
            cursor: audience.at(-1)?.id ?? row.cursor,
            expandedAt: audience.length < (this.#options.batch ?? BATCH) ? call.now : null,
        });
    }

    /** Decide whether a recipient may read a notification. */
    async #isReadable(
        call: Call,
        row: InstanceOf<typeof notification>,
        recipient: Subject,
    ): Promise<boolean> {
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

    /** Render the banner a desktop shows for a notification, with its declared actions. */
    #banner(row: InstanceOf<typeof notification>, declaration: Notification): Banner {
        const actions = Object.entries(declaration.definition.actions ?? {}).map(
            ([name, action]) => ({
                name,
                title: action.title,
                isDestructive: action.isDestructive === true,
                ...(action.text === undefined ? {} : { text: action.text }),
            }),
        );

        return { ...declaration.render(row), actions };
    }

    /** Summarize notifications, one line per declaration and thread. */
    #summarize(sending: readonly Candidate[]): readonly string[] {
        const groups = new Map<string, { declaration: Notification; count: number }>();
        for (const { declaration, notification: parent } of sending) {
            const group = `${parent.packageId} ${parent.name} ${parent.thread}`;
            const known = groups.get(group) ?? { declaration, count: 0 };
            groups.set(group, { declaration, count: known.count + parent.count });
        }

        return [...groups.values()].map((group) =>
            group.declaration.definition.summary(group.count),
        );
    }

    /** Read a page of the space's members: the users its copy follows because they joined it. */
    async #members(
        call: Call,
        page: { readonly after?: string; readonly limit: number },
    ): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
        const rows = await call.database
            .select({ id: user.table.id })
            .from(user.table)
            .where(
                and(
                    Replica.includes(call.scope, user.table as Table),
                    page.after === undefined
                        ? undefined
                        : gt(user.table.id, identifier("user").parse(page.after)),
                ),
            )
            .orderBy(asc(user.table.id))
            .limit(page.limit);

        return rows.map((row) => ({
            id: row.id,
            owner: principal.user.reference(Scope.universe.id, row.id),
        }));
    }

    /** Read a page of the users with a permission on an object. */
    async #permitted(
        call: Call,
        source: ObjectReference,
        permission: string,
        page: { readonly after?: string; readonly limit: number },
    ): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
        // list the user principals with the permission
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

        return subjects.map((owner) => ({ id: Subject.key(owner), owner }));
    }

    /** Read a push topic from an identifier's 32 hexadecimal digits, as RFC 8030 allows. */
    #topic(id: string): string {
        return schema
            .string()
            .length(32)
            .parse(id.slice(id.indexOf("-") + 1).replaceAll("-", ""));
    }
}
