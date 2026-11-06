import { testSimulation } from "@destack/object/test";
import type { Message } from "@destack/message/object";
import { type Activity, Inbox, Notification } from "@destack/notification/client";
import { Identifier, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Banner, BannerCapabilities } from "./banner.ts";

/** What banners offer on macOS, where they withdraw and offer actions. */
const APPLE = BannerCapabilities.of("darwin");

/** The reply a mention offers, as its banner shows it. */
const REPLY = {
    name: "reply",
    title: "Reply",
    isDestructive: false,
    text: { placeholder: "Reply", button: "Send" },
};

/** The space the mentions happened in. */
const SPACE = schema.identifier("space").parse("space-019f5530-8000-7000-8000-000000000004");

/** The home storing the person's notifications and messages. */
const HOME = schema.identifier("space").parse("space-019f5530-8000-7000-8000-000000000005");

/** The package declaring the mentions. */
const PACKAGE = schema.identifier("package").parse("package-019f5530-8000-7000-8000-000000000009");

/** The desktop client the person signed in on. */
const CLIENT = schema.identifier("client").parse("client-019f5530-8000-7000-8000-000000000006");

/** When the mentions occurred: 2026-01-01 in epoch milliseconds. */
const OCCURRED_AT = 1_767_225_600_000;

/** A mention's activity as a banner shows it. */
function occurrence(id: string, body: string): Banner["activity"] {
    return {
        id: schema.identifier("activity").parse(id),
        scope: SPACE,
        packageId: PACKAGE,
        content: { title: "Bob mentioned you", body },
        interruption: "active",
        actions: [
            { name: "reply", title: "Reply", text: { placeholder: "Reply", button: "Send" } },
        ],
    };
}

/** A desktop message of a mention, shown already where received. */
function mention(id: string, body: string, isDelivered = false): Banner {
    return new Banner({
        id,
        isDelivered,
        activity: occurrence("activity-019f5530-8000-7000-8000-000000000001", body),
    });
}

/** The columns every stored row has, written and managed by nobody at the occurrence. */
const AUDIT = {
    tags: {},
    revision: 1,
    createdAt: OCCURRED_AT,
    createdByPackageId: null,
    createdByType: null,
    createdByScope: null,
    createdById: null,
    updatedAt: OCCURRED_AT,
    updatedByPackageId: null,
    updatedByType: null,
    updatedByScope: null,
    updatedById: null,
    managerInstallationId: null,
    managerPackageId: null,
    managerName: null,
    detachedAt: null,
};

/** A mention's activity as the home's replica stores it, for the person and from Bob. */
function stored(occurred: Banner["activity"]): Activity {
    return {
        ...AUDIT,
        ...occurred,
        parentPackageId: PACKAGE,
        parentType: "document",
        parentId: "document-019f5530-8000-7000-8000-000000000008",
        recipientPackageId: PACKAGE,
        recipientType: "user",
        recipientScope: HOME,
        recipientId: "user-019f5530-8000-7000-8000-000000000007",
        actorPackageId: PACKAGE,
        actorType: "user",
        actorScope: HOME,
        actorId: "user-019f5530-8000-7000-8000-000000000010",
        packageId: PACKAGE,
        name: "mention",
        key: null,
        thread: occurred.id,
        reason: "mention",
        payload: {},
        count: 1,
        firstOccurredAt: OCCURRED_AT,
        occurredAt: OCCURRED_AT,
        summary: occurred.content.title,
        preference: { channels: ["desktop"], delivery: "immediate" },
    };
}

/** The notification of an activity in this test's home. */
function notificationOf(activityId: string): Identifier<"notification"> {
    return schema
        .identifier("notification")
        .parse(activityId.replace("activity-", "notification-"));
}

/** A person's state of an activity: when they read and archived it. */
function inboxState(
    occurred: Banner["activity"],
    readAt: number | null,
    archivedAt: number | null,
): Notification {
    return {
        ...AUDIT,
        id: notificationOf(occurred.id),
        scope: HOME,
        activityId: occurred.id,
        readAt,
        archivedAt,
        snoozedUntil: null,
        plannedAt: OCCURRED_AT,
        deliveryAt: null,
        delivered: [],
        summaryAt: null,
    };
}

test("show each new message's banner once and none shown before a relaunch, and withdraw the banners of messages gone", () => {
    const first = mention(
        "message-019f5530-8000-7000-8000-000000000001",
        "can you check the intro?",
    );
    const second = mention("message-019f5530-8000-7000-8000-000000000002", "and the outro?");
    const received = mention(
        "message-019f5530-8000-7000-8000-000000000004",
        "and the title?",
        true,
    );

    // show both, then nothing for the same rows, then withdraw the first once it is gone
    const shown = new Map([[first.id, first]]);
    expect([
        Banner.plan(new Map(), [first, second, received], APPLE),
        Banner.plan(new Map([...shown, [second.id, second]]), [first, second], APPLE),
        Banner.plan(new Map([...shown, [second.id, second]]), [second], APPLE),
    ]).toEqual([
        [
            {
                action: "banner.show",
                id: first.id,
                title: "Bob mentioned you",
                body: "can you check the intro?",
                interruption: "active",
                actions: [REPLY],
            },
            {
                action: "banner.show",
                id: second.id,
                title: "Bob mentioned you",
                body: "and the outro?",
                interruption: "active",
                actions: [REPLY],
            },
        ],
        [],
        [{ action: "banner.withdraw", id: first.id }],
    ]);
});

test("show banners without actions and withdraw none where the system's banners offer neither", () => {
    const first = mention(
        "message-019f5530-8000-7000-8000-000000000001",
        "can you check the intro?",
    );

    // show the mention without its reply, then leave its banner once the message is gone
    const windows = Banner.plan(new Map(), [first], BannerCapabilities.of("win32"));
    const gone = Banner.plan(new Map([[first.id, first]]), [], BannerCapabilities.of("linux"));
    expect([windows, gone]).toEqual([
        [
            {
                action: "banner.show",
                id: first.id,
                title: "Bob mentioned you",
                body: "can you check the intro?",
                interruption: "active",
                actions: [],
            },
        ],
        [],
    ]);
});

/** A message of an activity's notification to a destination. */
function sentMessage(
    id: string,
    occurred: Banner["activity"],
    to: Message["to"],
    status: Message["status"] = "sent",
): Pick<Message, "id" | "to" | "sourceId" | "status"> {
    return {
        id: schema.identifier("message").parse(id),
        to,
        status,
        sourceId: notificationOf(occurred.id),
    };
}

test("select the messages to this desktop whose notification is unread and in the inbox", () => {
    // run in a separate world
    const world = testSimulation().environment("test");

    // store three activities and their notifications: unread, read, and archived
    const unread = occurrence("activity-019f5530-8000-7000-8000-000000000001", "the intro?");
    const read = occurrence("activity-019f5530-8000-7000-8000-000000000002", "the outro?");
    const archived = occurrence("activity-019f5530-8000-7000-8000-000000000003", "the title?");
    const rows = [
        { ...inboxState(unread, null, null), activity: stored(unread) },
        { ...inboxState(read, OCCURRED_AT + 1, null), activity: stored(read) },
        { ...inboxState(archived, null, OCCURRED_AT + 1), activity: stored(archived) },
    ];

    // send a message of each to this desktop, one to another desktop, and one by email
    const desktop = { channel: "desktop", client: CLIENT } as const;
    const other = { channel: "desktop", client: Identifier.create("client", world) } as const;
    const messages = [
        sentMessage("message-019f5530-8000-7000-8000-000000000011", unread, desktop, "delivered"),
        sentMessage("message-019f5530-8000-7000-8000-000000000012", read, desktop),
        sentMessage("message-019f5530-8000-7000-8000-000000000013", archived, desktop),
        sentMessage("message-019f5530-8000-7000-8000-000000000014", unread, other),
        sentMessage("message-019f5530-8000-7000-8000-000000000015", unread, {
            channel: "email",
            address: "ada@example.test",
        }),
    ];

    // select the unread one to this desktop, received already
    expect(Banner.select(messages, new Inbox(rows, OCCURRED_AT), CLIENT)).toEqual([
        new Banner({
            id: "message-019f5530-8000-7000-8000-000000000011",
            isDelivered: true,
            activity: stored(unread),
        }),
    ]);
});

test("withdraw every shown banner once nobody is signed in to answer them", () => {
    const first = mention("message-019f5530-8000-7000-8000-000000000001", "the intro?");
    const second = mention("message-019f5530-8000-7000-8000-000000000002", "the outro?");

    // plan no banners over both shown
    const shown = new Map([
        [first.id, first],
        [second.id, second],
    ]);
    expect(Banner.plan(shown, [], APPLE)).toEqual([
        { action: "banner.withdraw", id: first.id },
        { action: "banner.withdraw", id: second.id },
    ]);
});
