import { client, user } from "@destack/account/object";
import { type DatabaseConnection, inArray } from "@destack/db";
import { type Catalog, Localization, t } from "@destack/locale";
import {
    Content,
    message,
    messageConditions,
    type Destination,
    MessageKey,
} from "@destack/message";
import { GONE } from "@destack/message/provider";
import {
    type Call,
    type CallOf,
    type NextOf,
    ObjectWatch,
    type ResultOf,
    type StatusCondition,
} from "@destack/object";
import { ObjectExtension } from "@destack/object/server";
import { aligned, Duration, found, present, schema, Text } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { setting } from "@destack/setting/object";
import { Scope, Subject } from "@destack/sync";
import { Contact, decide, type Decision, type Settings } from "../delivery/index.ts";
import type { Wording } from "../notification/content.ts";
import { ActivityReference, delivery, type Delivery } from "../object/delivery.ts";
import { notification, type Notification } from "../object/activity.ts";
import type { InterruptionLevel } from "../preference/preference.ts";
import { inboxService } from "../service/service.ts";
import { focus, preferenceSetting, summary } from "../setting/setting.ts";

/** The keys reconciled at once by default. */
const CONCURRENCY = 16;

/** How long a push service keeps a message for an offline browser by default: one day. */
const PUSH_TTL: Duration = { days: 1 };

/** The default email delay: a quarter hour, as chat and issue trackers commonly wait. */
const EMAIL_DELAY: Duration = { minutes: 15 };

/** How urgently each interruption level's push goes out, as RFC 8030's urgencies. */
const URGENCY = {
    passive: "very-low",
    active: "normal",
    timeSensitive: "high",
    critical: "high",
} as const satisfies Record<InterruptionLevel, string>;

/** What a home delivers its notifications with. */
export interface InboxOptions {
    /** The inbox's own catalogs, rendering its messages in each recipient's locale. */
    readonly catalogs: readonly Catalog[];
    /** Read the public key a space's messages are sealed to, its message home's. */
    readonly key: (space: string) => Promise<string>;
    /** The keys each controller reconciles at once, 16 by default. */
    readonly concurrency?: number;
    /** How long a push service keeps a message for an offline browser, a day by default. */
    readonly pushTtl?: Duration;
    /** How long an email waits for the notification to be read elsewhere, 15 minutes by default. */
    readonly emailDelay?: Duration;
}

/** Extend a home's store with the inbox: its notifications and their deliveries. */
export function implementInbox(options: InboxOptions): ObjectExtension {
    return {
        ...ObjectExtension.of(inboxService, Object.values(serveInbox(options))),
        policies: [user, setting, client, message],
    };
}

/** Serve a home's notifications and deliveries. */
export function serveInbox(options: InboxOptions) {
    return {
        notification: serveNotifications(options),
        delivery: serveDeliveries(options),
    };
}

/** Serve notifications, planning the deliveries of each unread one once its snooze ends. */
function serveNotifications(options: InboxOptions) {
    return notification.implement({
        methods: {
            read: (call, next) => readNotification(call, next),
            plan: (call) => plan(call),
        },
        controller: {
            pending: {
                readAt: { isNull: true },
                archivedAt: { isNull: true },
                OR: [{ plannedAt: { isNull: true } }, { snoozedUntil: { isNotNull: true } }],
            },
            concurrency: options.concurrency ?? CONCURRENCY,
            reconcile: async (reconciliation) => {
                // plan the notification's deliveries once its snooze ends
                const row = aligned(reconciliation.rows, 0);
                if (row.snoozedUntil !== null && row.snoozedUntil > reconciliation.now) {
                    return row.snoozedUntil - reconciliation.now;
                }
                await reconciliation.mutate([notification.call("plan", notification.target(row))]);

                return undefined;
            },
        },
    });
}

/** Serve deliveries, sending each due one alone or with its summary. */
function serveDeliveries(options: InboxOptions) {
    return delivery.implement({
        methods: {
            send: (call) => send(call, options),
            settle: (call) => settle(call),
        },
        controller: {
            pending: { state: { in: ["pending", "sending"] } },
            key: (row) => deliveryKey(row),
            // settle a delivery once the copy of its message changes
            watches: [ObjectWatch.of(message.table, (row) => [{ message: row.id }])],
            concurrency: options.concurrency ?? CONCURRENCY,
            reconcile: async (reconciliation) => {
                // settle the deliveries of a message
                const first = aligned(reconciliation.rows, 0);
                if (first.state === "sending") {
                    await reconciliation.mutate([delivery.call("settle", delivery.target(first))]);

                    return undefined;
                }
                // wait until the deliveries are due
                else if (first.dueAt > reconciliation.now) {
                    return first.dueAt - reconciliation.now;
                }

                // send a delivery, or a summary once per recipient and address
                const leads = first.isSummarized
                    ? await summaryLeads(reconciliation.database, reconciliation.rows)
                    : [first];
                for (const lead of leads) {
                    await reconciliation.mutate([delivery.call("send", delivery.target(lead))]);
                }

                return undefined;
            },
        },
    });
}

/** Key a delivery by the message, summary or lone send it goes out with. */
function deliveryKey(row: Delivery) {
    // settle the deliveries of one message together
    if (row.message !== null) {
        return { message: row.message };
    }
    // send a summary's deliveries together
    else if (row.isSummarized) {
        return { scope: row.scope, channel: row.channel, dueAt: row.dueAt, isSummarized: true };
    }
    // send every other alone
    else {
        return { id: row.id };
    }
}

/** Mark a notification read, and discard its desktop deliveries, which the desktops withdraw. */
async function readNotification(
    call: CallOf<typeof notification, "read">,
    next: NextOf<typeof notification, "read">,
): Promise<ResultOf<typeof notification, "read">> {
    // mark it read
    const read = await next();

    // discard its desktop deliveries
    const desktops = await call
        .read(delivery)
        .rows({ parentId: call.target.id, channel: "desktop" });
    await call.mutate(
        call.service,
        desktops.map((desktop) => delivery.call("discard", { id: desktop.id })),
    );

    return read;
}

/** Plan a notification's deliveries. */
async function plan(
    call: CallOf<typeof notification, "plan">,
): Promise<ResultOf<typeof notification, "plan">> {
    // discard the deliveries of an earlier occurrence
    const row = call.requireTarget();
    const earlier = await call.read(delivery).rows({ parentId: row.id });
    await call.mutate(
        call.service,
        earlier.map((each) => delivery.call("discard", { id: each.id })),
    );

    // schedule every desktop, browser and email of the recipient, which each send decides on
    const contact = await Contact.read(call.requireTransaction(), Subject.read(row.recipient));
    const targets = [
        ...contact.desktops.map((id) => ({ channel: "desktop" as const, client: id })),
        ...contact.browsers.map((browser) => ({ channel: "push" as const, client: browser.id })),
        { channel: "email" as const },
    ];
    await call.mutate(
        call.service,
        targets.map((target) =>
            delivery.call("schedule", {
                parentId: row.id,
                ...target,
                state: "pending",
                dueAt: call.now,
            }),
        ),
    );

    // mark the notification planned
    await call.update({ plannedAt: call.now, snoozedUntil: null });

    return {};
}

/** Decide a due delivery or summary, and send what goes out now as one message. */
async function send(
    call: CallOf<typeof delivery, "send">,
    options: InboxOptions,
): Promise<ResultOf<typeof delivery, "send">> {
    // read the delivery, its summary's siblings, their notifications and their recipient's contact
    const lead = call.requireTarget();
    const { recipient, rows, parents } = await deliveries(call, lead);
    const contact = await Contact.read(call.requireTransaction(), recipient);
    const emailDelay = Duration.milliseconds(options.emailDelay ?? EMAIL_DELAY);

    // decide each delivery, reading the settings of each release, declaration and space once
    const resolved = new Map<string, Settings>();
    const sending: Delivery[] = [];
    for (const row of rows) {
        const parent = found(parents, row.parentId);
        const group = `${parent.packageId} ${parent.release.version} ${parent.name} ${parent.space}`;
        const settings = resolved.get(group) ?? (await settingsOf(call, recipient, parent));
        resolved.set(group, settings);
        const decision = decide(row, parent, contact, settings, emailDelay, call.now);
        // send it
        if (decision.action === "send") {
            sending.push(row);
        }
        // record it waiting or skipped
        else {
            await call.mutate(call.service, [delivery.call("record", held(row, decision))]);
        }
    }
    if (sending.length === 0) {
        return {};
    }

    // show desktop deliveries from the notification's state on the desktop
    if (lead.channel === "desktop") {
        await call.mutate(
            call.service,
            sending.map((row) =>
                delivery.call("record", { id: row.id, state: "sent", sentAt: call.now }),
            ),
        );

        return {};
    }

    // send the rest as one message in the recipient's locale, keyed by the lead delivery
    const locale = Localization.of(contact.locale, options.catalogs);
    const parent = found(parents, lead.parentId);
    const id = messageOf(lead);
    const isSingle = sending.length === 1 && !lead.isSummarized;
    const content: Wording = isSingle
        ? parent.content
        : {
              title: locale.render(t`Scheduled summary`),
              body: summarize(sending.map((row) => found(parents, row.parentId))).join("\n"),
          };

    // seal the message's content to the home's message key before it leaves the inbox
    const addressed = address(lead, contact, content, parent, options);
    const key = MessageKey.of(await options.key(call.scope));
    const ciphertext = await key.seal(addressed.content, call.scope);
    await call.mutate(call.caller, [
        message.call("create", {
            id,
            to: addressed.to,
            source: delivery.reference(call.scope, lead.id),
            status: "pending",
            ciphertext,
        }),
    ]);

    // record the deliveries sending
    await call.mutate(
        call.service,
        sending.map((row) =>
            delivery.call("record", { id: row.id, state: "sending", message: id }),
        ),
    );

    return {};
}

/** Settle the deliveries of a message from its copy's Sent condition. */
async function settle(
    call: CallOf<typeof delivery, "settle">,
): Promise<ResultOf<typeof delivery, "settle">> {
    // read the message's copy, and the deliveries waiting for it
    const lead = call.requireTarget();
    const id = schema.identifier("message").parse(lead.message);
    const sent = await call.read(message).row({ id });
    if (sent === null) {
        throw new ServiceError("NOT_FOUND", { message: `no message ${id} of delivery ${lead.id}` });
    }
    const condition = messageConditions.read(sent, "Sent");
    if (condition === undefined || condition.status === "unknown") {
        return {};
    }
    const waiting = await call.read(delivery).rows({ message: id, state: "sending" });

    // forget a browser its push service forgot
    const isGone = condition.reason === GONE && lead.channel === "push" && lead.client !== null;
    if (isGone) {
        await forget(call, lead);
    }

    // record each delivery sent, skipped or failed
    const outcome = settled(condition, sent.settledAt, isGone);
    await call.mutate(
        call.service,
        waiting.map((row) => delivery.call("record", { id: row.id, ...outcome })),
    );

    return {};
}

/** Forget a browser's push subscription its push service forgot, in its recipient's scope. */
async function forget(call: Call, lead: Delivery): Promise<void> {
    // unsubscribe the browser client in the notification's recipient's scope
    const parent = await call.read(notification).row({ id: lead.parentId });
    const recipient = Subject.read(present(parent, "the delivery's notification").recipient);
    await call.mutate(call.caller, [
        client.call("unsubscribe", {
            id: present(lead.client, "the browser client of a push delivery"),
            userId: recipient.id,
        }),
    ]);
}

/** Describe the record of a delivery that waits or never goes out. */
function held(row: Delivery, decision: Exclude<Decision, { readonly action: "send" }>) {
    return decision.action === "skip"
        ? { id: row.id, state: "skipped" as const, reason: decision.reason }
        : {
              id: row.id,
              state: "pending" as const,
              dueAt: decision.until,
              isSummarized: decision.isSummarized,
          };
}

/** Describe the record of the deliveries of a settled message. */
function settled(condition: StatusCondition, settledAt: number | null, isGone: boolean) {
    // record them sent
    if (condition.status === "true") {
        return { state: "sent" as const, sentAt: present(settledAt, "a sent message's time") };
    }
    // record them skipped once their browser is gone
    else if (isGone) {
        return { state: "skipped" as const, reason: "gone" as const };
    }
    // record them failed
    else {
        return {
            state: "failed" as const,
            error: { code: condition.reason, message: condition.message },
        };
    }
}

/** Resolve the recipient's settings where the notification's package and space place them. */
async function settingsOf(call: Call, recipient: Subject, parent: Notification): Promise<Settings> {
    // read the recipient's placed values and the chain of the notification's space
    const preference = preferenceSetting(parent.release, parent);
    const placed = await call.read(setting).rows({
        AND: [
            { scope: recipient.id },
            {
                OR: [preference, focus, summary].map((declared) => ({
                    packageId: declared.reference.packageId,
                    name: declared.reference.name,
                })),
            },
        ],
    });
    const chain = (await Scope.chain(call.view(), parent.space)).map((link) => link.object.id);
    const selection = { scope: recipient.id, space: parent.space, package: parent.packageId };

    return {
        preference: preference.resolve(selection, placed, chain).value,
        focus: focus.resolve(selection, placed, chain).value,
        summary: summary.resolve(selection, placed, chain).value,
    };
}

/** Address a message to the lead delivery's email or browser, with its content on that channel. */
function address(
    lead: Delivery,
    contact: Contact,
    content: Wording,
    parent: Notification,
    options: InboxOptions,
): { readonly to: Destination; readonly content: Content } {
    // email the recipient's verified address
    if (lead.channel === "email") {
        const text = [content.subtitle, content.body].filter((line) => line !== undefined);

        return {
            to: { channel: "email", address: present(contact.email, "the verified email") },
            content: { channel: "email", subject: content.title, text: text.join("\n\n") },
        };
    }

    // push one notification with its activity and actions, or a summary, as urgent as its interruption
    const browser = present(
        contact.browsers.find((known) => known.id === lead.client),
        "the browser client",
    );
    const data = lead.isSummarized
        ? { scope: lead.scope, content, actions: [] }
        : {
              notification: parent.id,
              scope: lead.scope,
              content,
              activity: ActivityReference.parse(parent.source),
              actions: Object.entries(parent.actions).map(([name, action]) => ({
                  name,
                  title: action.title,
                  isDestructive: action.isDestructive === true,
                  hasText: action.text !== undefined,
              })),
          };
    const level = lead.isSummarized ? "passive" : parent.interruption;

    return {
        to: { channel: "push", url: browser.url, keys: browser.keys },
        content: {
            channel: "push",
            data,
            urgency: URGENCY[level],
            ttl: Duration.seconds(options.pushTtl ?? PUSH_TTL),
            topic: topic(lead.isSummarized ? lead.id : parent.id),
        },
    };
}

/** Read a delivery's recipient, the deliveries sent with it and their notifications. */
async function deliveries(
    call: Call,
    lead: Delivery,
): Promise<{
    readonly recipient: Subject;
    readonly rows: readonly Delivery[];
    readonly parents: ReadonlyMap<string, Notification>;
}> {
    // read the lead's notification
    const first = await call.read(notification).row({ id: lead.parentId });
    if (first === null) {
        throw new TypeError(`delivery ${lead.id} has no notification`);
    }

    // read its summary's siblings and their notifications
    const rows = lead.isSummarized ? await siblings(call, lead, first.recipient) : [lead];
    const notifications = await call
        .read(notification)
        .rows({ id: { in: rows.map((row) => row.parentId) } });

    return {
        recipient: Subject.read(first.recipient),
        rows,
        parents: new Map(notifications.map((known) => [known.id, known])),
    };
}

/** Read the pending deliveries sent in one summary. */
async function siblings(
    call: Call,
    lead: Delivery,
    recipient: string,
): Promise<readonly Delivery[]> {
    // read the pending summarized deliveries due with the lead, and keep those of the recipient
    const due = await call.read(delivery).rows(
        {
            scope: lead.scope,
            channel: lead.channel,
            client: lead.client === null ? { isNull: true } : lead.client,
            state: "pending",
            isSummarized: true,
            dueAt: lead.dueAt,
        },
        { orderBy: { id: "asc" } },
    );
    const recipients = await call
        .read(notification)
        .rows({ id: { in: due.map((row) => row.parentId) }, recipient });
    const addressed = new Set(recipients.map((row) => row.id));

    return due.filter((row) => addressed.has(row.parentId));
}

/** Pick the summarized delivery leading each recipient and address's summary. */
async function summaryLeads(
    database: DatabaseConnection,
    rows: readonly Delivery[],
): Promise<readonly Delivery[]> {
    // read each delivery's recipient
    const notifications = await database
        .select({ id: notification.table.id, recipient: notification.table.recipient })
        .from(notification.table)
        .where(
            inArray(
                notification.table.id,
                rows.map((row) => row.parentId),
            ),
        );
    const recipients = new Map(notifications.map((entry) => [entry.id, entry.recipient]));

    // keep the last delivery of each recipient and address in identifier order
    const leads = new Map<string, Delivery>();
    for (const row of rows.toSorted((left, right) => Text.compare(left.id, right.id))) {
        const recipient = found(recipients, row.parentId);
        leads.set(`${recipient} ${row.client}`, row);
    }

    return [...leads.values()];
}

/** Summarize notifications, one line per declaration and thread: its latest summary line. */
function summarize(sent: readonly Notification[]): readonly string[] {
    const groups = new Map<string, Notification>();
    for (const parent of sent) {
        const group = `${parent.packageId} ${parent.name} ${parent.thread}`;
        const known = groups.get(group);
        if (known === undefined || known.occurredAt <= parent.occurredAt) {
            groups.set(group, parent);
        }
    }

    return [...groups.values()].map((latest) => latest.summary);
}

/** Derive the identifier of the message sending a delivery, so a repeated send writes it once. */
function messageOf(lead: Delivery) {
    return schema.identifier("message").parse(`message-${lead.id.slice("delivery-".length)}`);
}

/** Read a push topic from an identifier's 32 hexadecimal digits, as RFC 8030 allows. */
function topic(id: string): string {
    return schema
        .string()
        .length(32)
        .parse(id.slice(id.indexOf("-") + 1).replaceAll("-", ""));
}
