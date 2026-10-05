import { pushEndpoint } from "@destack/account/object";
import { and, type DatabaseConnection, eq, inArray, isNull, Snapshot } from "@destack/db";
import { type Catalog, Localization, t } from "@destack/locale";
import { message, type Message } from "@destack/message";
import { GONE } from "@destack/message/push";
import { type Call, type CallOf, type NextOf, ObjectWatch, type ResultOf } from "@destack/object";
import { aligned, Duration, found, present, schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import { Contact, decide, type Decision, type Settings } from "../delivery/index.ts";
import type { Content } from "../notification/content.ts";
import { ActivityReference, delivery, type Delivery } from "../object/delivery.ts";
import { notification, type Notification } from "../object/activity.ts";
import type { InterruptionLevel } from "../preference/preference.ts";
import { focus, preferenceSetting, summary } from "../setting/setting.ts";

/** The keys reconciled at once by default: 16 in flight at 5 to 20 ms of reads and writes plan 800 to 3200 a second. */
const CONCURRENCY = 16;

/** How long a push service keeps a message for an offline browser by default: one day. */
const PUSH_TTL: Duration = { days: 1 };

/** The default email delay: a quarter hour, as Slack and Linear wait. */
const EMAIL_DELAY: Duration = { minutes: 15 };

/** The milliseconds in a second, as push lifetimes count seconds. */
const MILLISECONDS = 1000;

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
    /** The keys each controller reconciles at once, 16 by default. */
    readonly concurrency?: number;
    /** How long a push service keeps a message for an offline browser, a day by default. */
    readonly pushTtl?: Duration;
    /** How long an email waits for the notification to be read elsewhere, 15 minutes by default. */
    readonly emailDelay?: Duration;
}

/** Serve a home's notifications and deliveries, planning the deliveries and sending each due one as a message. */
export function serveInbox(options: InboxOptions) {
    return {
        notification: serveNotifications(options),
        delivery: serveDeliveries(options),
    };
}

/** Serve notifications, planning the deliveries of each unread one once its snooze ends. */
function serveNotifications(options: InboxOptions) {
    return notification
        .handle({
            read: (call, next) => readNotification(call, next),
            deliver: (call) => plan(call),
        })
        .control({
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
                await reconciliation.execute("deliver", [row]);

                return undefined;
            },
        });
}

/** Serve deliveries, sending each due one alone or with its summary, and settling them from their message. */
function serveDeliveries(options: InboxOptions) {
    return delivery
        .handle({
            send: (call) => send(call, options),
            settle: (call) => settle(call),
        })
        .control({
            pending: { state: { in: ["pending", "sending"] } },
            // settle the deliveries of one message together, send a summary's together, and every other alone
            key: (row) =>
                row.message !== null
                    ? { message: row.message }
                    : row.isSummarized
                      ? {
                            scope: row.scope,
                            channel: row.channel,
                            dueAt: row.dueAt,
                            isSummarized: true,
                        }
                      : { id: row.id },
            // settle a delivery once the copy of its message changes
            watches: [ObjectWatch.of(message.table, (row) => [{ message: row.id }])],
            concurrency: options.concurrency ?? CONCURRENCY,
            reconcile: async (reconciliation) => {
                // settle the deliveries of a message
                const first = aligned(reconciliation.rows, 0);
                if (first.state === "sending") {
                    await reconciliation.execute("settle", [first]);

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
                    await reconciliation.execute("send", [lead]);
                }

                return undefined;
            },
        });
}

/** Read a notification, and discard its desktop deliveries, which the desktops withdraw. */
async function readNotification(
    call: CallOf<typeof notification, "read">,
    next: NextOf<typeof notification, "read">,
): Promise<ResultOf<typeof notification, "read">> {
    // read it
    const read = await next();

    // discard its desktop deliveries
    const desktops = await call.database
        .select({ id: delivery.table.id })
        .from(delivery.table)
        .where(
            and(eq(delivery.table.parentId, call.target.id), eq(delivery.table.channel, "desktop")),
        );
    for (const desktop of desktops) {
        await call.invoke(delivery).discard({ id: desktop.id });
    }

    return read;
}

/** Plan a notification's deliveries: one per desktop and push endpoint of its recipient, and one email. */
async function plan(
    call: CallOf<typeof notification, "deliver">,
): Promise<ResultOf<typeof notification, "deliver">> {
    // discard the deliveries of an earlier occurrence
    const row = call.requireTarget();
    const earlier = await call.database
        .select({ id: delivery.table.id })
        .from(delivery.table)
        .where(eq(delivery.table.parentId, row.id));
    for (const each of earlier) {
        await call.invoke(delivery).discard({ id: each.id });
    }

    // schedule every desktop, endpoint and email of the recipient for now
    const contact = await Contact.read(call.database, Subject.read(row.recipient));
    const targets = [
        ...contact.desktops.map((device) => ({ channel: "desktop" as const, device })),
        ...contact.endpoints.map((endpoint) => ({
            channel: "push" as const,
            endpoint: endpoint.id,
        })),
        { channel: "email" as const },
    ];
    for (const target of targets) {
        await call.invoke(delivery).schedule({
            parentId: row.id,
            ...target,
            state: "pending",
            dueAt: call.now,
        });
    }

    return call.update({ plannedAt: call.now, snoozedUntil: null });
}

/** Decide a due delivery or summary, and send what goes out now as one message. */
async function send(
    call: CallOf<typeof delivery, "send">,
    options: InboxOptions,
): Promise<ResultOf<typeof delivery, "send">> {
    // read the delivery, its summary's siblings, their notifications and their recipient's contact
    const lead = call.requireTarget();
    const { recipient, rows, parents } = await deliveries(call, lead);
    const contact = await Contact.read(call.database, recipient);
    const emailDelay = Duration.milliseconds(options.emailDelay ?? EMAIL_DELAY);

    // decide each delivery, recording what waits or never goes out
    const sending: Delivery[] = [];
    for (const row of rows) {
        const parent = found(parents, row.parentId);
        const settings = await settingsOf(call, recipient, parent);
        const decision = decide(row, parent, contact, settings, emailDelay, call.now);
        if (decision.action === "send") {
            sending.push(row);
        } else {
            await call.invoke(delivery).record(held(row, decision));
        }
    }
    if (sending.length === 0) {
        return {};
    }

    // show desktop deliveries from the notification's state on the desktop
    if (lead.channel === "desktop") {
        for (const row of sending) {
            await call.invoke(delivery).record({ id: row.id, state: "sent", sentAt: call.now });
        }

        return {};
    }

    // send the rest as one message in the recipient's locale, keyed by the lead delivery
    const locale = Localization.of(contact.locale, options.catalogs);
    const id = messageOf(lead);
    const isSingle = sending.length === 1 && !lead.isSummarized;
    const content: Content = isSingle
        ? found(parents, lead.parentId).content
        : {
              title: locale.render(t`Scheduled summary`),
              body: summarize(sending.map((row) => found(parents, row.parentId))).join("\n"),
          };
    await call.change(message, "create", {
        id,
        ...address(lead, contact, content, found(parents, lead.parentId), options),
    });
    for (const row of sending) {
        await call.invoke(delivery).record({ id: row.id, state: "sending", message: id });
    }

    return {};
}

/** Settle the deliveries of a message from its copy's Sent condition, forgetting a push endpoint its push service forgot. */
async function settle(
    call: CallOf<typeof delivery, "settle">,
): Promise<ResultOf<typeof delivery, "settle">> {
    // read the message's copy, and the deliveries waiting for it
    const lead = call.requireTarget();
    const id = schema.identifier("message").parse(lead.message);
    const [sent] = await call.database.select().from(message.table).where(eq(message.table.id, id));
    const condition = sent?.conditions["Sent"];
    if (sent === undefined || condition === undefined || condition.status === "unknown") {
        return {};
    }
    const waiting = await call.database
        .select({
            id: delivery.table.id,
            channel: delivery.table.channel,
            endpoint: delivery.table.endpoint,
        })
        .from(delivery.table)
        .where(and(eq(delivery.table.message, id), eq(delivery.table.state, "sending")));

    // record each sent, forgotten or failed
    const isGone = condition.reason === GONE && lead.channel === "push" && lead.endpoint !== null;
    if (isGone) {
        await forget(call, lead);
    }
    for (const row of waiting) {
        await call.invoke(delivery).record(
            condition.status === "true"
                ? { id: row.id, state: "sent", sentAt: sent.sentAt ?? call.now }
                : isGone
                  ? { id: row.id, state: "skipped", reason: "gone" }
                  : {
                        id: row.id,
                        state: "failed",
                        error: { code: condition.reason, message: condition.message },
                    },
        );
    }

    return {};
}

/** Forget a push endpoint its push service forgot, in its recipient's scope. */
async function forget(call: Call, lead: Delivery): Promise<void> {
    const [parent] = await call.database
        .select({ recipient: notification.table.recipient })
        .from(notification.table)
        .where(eq(notification.table.id, lead.parentId));
    const recipient = Subject.read(present(parent, "the delivery's notification").recipient);
    await call.change(pushEndpoint, "delete", { id: lead.endpoint, userId: recipient.id });
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

/** Resolve the recipient's settings where the notification's package and space place them. */
async function settingsOf(call: Call, recipient: Subject, parent: Notification): Promise<Settings> {
    // read the recipient's placed values and the chain of the notification's space
    const preference = preferenceSetting(parent.release, parent);
    const placed = await Contact.settings(call.database, recipient, [preference, focus, summary]);
    const chain = (await Scope.chain(Snapshot.live(call.database), parent.space)).map(
        (link) => link.object.id,
    );
    const selection = { scope: recipient.id, space: parent.space, package: parent.packageId };

    return {
        preference: preference.resolve(selection, placed, chain).value,
        focus: focus.resolve(selection, placed, chain).value,
        summary: summary.resolve(selection, placed, chain).value,
    };
}

/** Address a message to the lead delivery's email or push endpoint, with its content on that channel. */
function address(
    lead: Delivery,
    contact: Contact,
    content: Content,
    parent: Notification,
    options: InboxOptions,
): Pick<Message, "to" | "content"> {
    // email the recipient's verified address
    if (lead.channel === "email") {
        const text = [content.subtitle, content.body].filter((line) => line !== undefined);

        return {
            to: { channel: "email", address: present(contact.email, "the verified email") },
            content: { channel: "email", subject: content.title, text: text.join("\n\n") },
        };
    }

    // push one notification with its activity and actions, or a summary, as urgent as its interruption
    const endpoint = present(
        contact.endpoints.find((known) => known.id === lead.endpoint),
        "the push endpoint",
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
        to: { channel: "push", url: endpoint.url, keys: endpoint.keys },
        content: {
            channel: "push",
            data,
            urgency: URGENCY[level],
            ttl: Duration.milliseconds(options.pushTtl ?? PUSH_TTL) / MILLISECONDS,
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
    const [first] = await call.database
        .select()
        .from(notification.table)
        .where(eq(notification.table.id, lead.parentId));
    if (first === undefined) {
        throw new TypeError(`delivery ${lead.id} has no notification`);
    }

    // read its summary's siblings and their notifications
    const rows = lead.isSummarized ? await siblings(call, lead, first.recipient) : [lead];
    const notifications = await call.database
        .select()
        .from(notification.table)
        .where(
            inArray(
                notification.table.id,
                rows.map((row) => row.parentId),
            ),
        );

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
    for (const row of rows.toSorted(
        (left, right) => Number(left.id > right.id) - Number(left.id < right.id),
    )) {
        const recipient = found(recipients, row.parentId);
        leads.set(`${recipient} ${row.endpoint}`, row);
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

/** Name the message sending a delivery after it, so a repeated send writes it once. */
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
