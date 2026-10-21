import { expect, test } from "@destack/test";
import type { Destination } from "@destack/message";
import { schema } from "@destack/schema";
import {
    focus,
    Recipient,
    Routing,
    type RoutingEntry,
    type Settings,
    summarySchedule,
    Window,
} from "../src/index.ts";
import type * as accountObject from "@destack/account/object";
import { notes } from "./fixture/document.ts";

/** How long an email waits in the scenarios: a quarter hour. */
const EMAIL_DELAY = 15 * 60_000;

/** Saturday 24 October 2026, 12:00 UTC, the day before Vienna leaves summer time at 01:00 UTC. */
const SATURDAY = Date.UTC(2026, 9, 24, 12, 0);

/** An hour, in milliseconds. */
const HOUR = 3_600_000;

test("end quiet hours in a time zone after the night the clocks go back", () => {
    // end quiet hours from 22:00 to 07:00 after the night the clocks go back, and defer none at noon
    const night = { days: [6], from: "22:00", to: "07:00" };
    const inside = Window.end("Europe/Vienna", [night], Date.UTC(2026, 9, 25, 1, 30));
    const outside = Window.end("Europe/Vienna", [night], SATURDAY);
    expect([inside, outside]).toEqual([Date.UTC(2026, 9, 25, 6, 0), undefined]);
});

/** The browser on the person's phone, subscribed to pushes. */
const PHONE = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000a1");

/** The person receiving the notifications. */
const PERSON = schema.identifier("user").parse("user-019f5530-8000-7000-8000-0000000000a3");

/** The desktop the person signed in. */
const LAPTOP = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000a2");

test("decide each destination by the first rule that applies, from read state down to the email delay", () => {
    // start from an unread, active mention the person receives everywhere, with nothing quiet
    const phone = { url: "https://push.example/phone", keys: { p256dh: "key", auth: "auth" } };
    const push: Destination = { channel: "push", client: PHONE, user: PERSON, ...phone };
    const entry: RoutingEntry = {
        isUnread: true,
        notification: { snoozedUntil: null },
        activity: { packageId: notes.id, occurredAt: SATURDAY, interruption: "active" },
    };
    const recipient: accountObject.Recipient = {
        desktops: [LAPTOP],
        browsers: [{ id: PHONE, ...phone }],
        email: "dana@example.com",
        timeZone: "UTC",
        locale: "en",
    };
    const settings: Settings = {
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        focus: focus.definition.default,
        summarySchedule: summarySchedule.definition.default,
    };
    const focused = { ...focus.definition.default, until: SATURDAY + HOUR };
    const desktop: Destination = { channel: "desktop", client: LAPTOP };
    const email: Destination = { channel: "email", address: "dana@example.com" };
    const summarized: Changes["settings"] = {
        preference: { channels: ["push"], delivery: "summary" },
        summarySchedule: { times: ["18:00"], channels: ["push"] },
    };
    const cases: readonly [string, Changes][] = [
        ["read", { entry: { isUnread: false } }],
        ["unaddressed", { recipient: { browsers: [] } }],
        ["snoozed", { entry: { notification: { snoozedUntil: SATURDAY + HOUR } } }],
        [
            "critical in a focus, turned off",
            {
                activity: { interruption: "critical" },
                settings: { focus: focused, preference: { channels: [], delivery: "immediate" } },
            },
        ],
        [
            "turned off",
            { settings: { preference: { channels: ["desktop"], delivery: "immediate" } } },
        ],
        ["passive", { activity: { interruption: "passive" } }],
        ["summarized", { settings: summarized }],
        ["summarized in the summary going out", { settings: summarized, summaryAt: SATURDAY }],
        [
            "summarized on the desktop",
            {
                to: desktop,
                settings: { preference: { channels: ["desktop"], delivery: "summary" } },
            },
        ],
        [
            "time-sensitive, summarized",
            {
                activity: { interruption: "timeSensitive" },
                settings: { preference: { channels: ["push"], delivery: "summary" } },
            },
        ],
        ["in a focus", { settings: { focus: focused } }],
        [
            "in a focus allowing the app",
            { settings: { focus: { ...focused, allowed: [notes.id] } } },
        ],
        [
            "time-sensitive in a focus",
            { activity: { interruption: "timeSensitive" }, settings: { focus: focused } },
        ],
        [
            "time-sensitive in a strict focus",
            {
                activity: { interruption: "timeSensitive" },
                settings: { focus: { ...focused, isTimeSensitiveAllowed: false } },
            },
        ],
        ["email", { to: email }],
        ["email after the delay", { to: email, now: SATURDAY + EMAIL_DELAY }],
    ];

    // decide each case
    const decided = cases.map(([name, changes]) => [
        name,
        new Routing(
            { ...entry, ...changes.entry, activity: { ...entry.activity, ...changes.activity } },
            new Recipient({ ...recipient, ...changes.recipient }, PERSON),
            { ...settings, ...changes.settings },
            {
                emailDelay: EMAIL_DELAY,
                ...(changes.summaryAt === undefined ? {} : { summaryAt: changes.summaryAt }),
            },
            changes.now ?? SATURDAY,
        ).decide(changes.to ?? push),
    ]);
    expect(decided).toEqual([
        ["read", { action: "skip", reason: "read" }],
        ["unaddressed", { action: "skip", reason: "unaddressed" }],
        ["snoozed", { action: "defer", until: SATURDAY + HOUR }],
        ["critical in a focus, turned off", { action: "send" }],
        ["turned off", { action: "skip", reason: "preference" }],
        ["passive", { action: "skip", reason: "preference" }],
        ["summarized", { action: "summarize", at: Date.UTC(2026, 9, 24, 18, 0) }],
        ["summarized in the summary going out", { action: "send" }],
        ["summarized on the desktop", { action: "skip", reason: "preference" }],
        ["time-sensitive, summarized", { action: "send" }],
        ["in a focus", { action: "defer", until: SATURDAY + HOUR }],
        ["in a focus allowing the app", { action: "send" }],
        ["time-sensitive in a focus", { action: "send" }],
        ["time-sensitive in a strict focus", { action: "defer", until: SATURDAY + HOUR }],
        ["email", { action: "defer", until: SATURDAY + EMAIL_DELAY }],
        ["email after the delay", { action: "send" }],
    ]);
});

/** What one case changes of the base scenario. */
interface Changes {
    /** The destination, the phone by default. */
    readonly to?: Destination;
    /** The entry's changed state. */
    readonly entry?: Partial<Omit<RoutingEntry, "activity">>;
    /** The activity's changed fields. */
    readonly activity?: Partial<RoutingEntry["activity"]>;
    /** The recipient's changed fields. */
    readonly recipient?: Partial<accountObject.Recipient>;
    /** The changed settings. */
    readonly settings?: Partial<Settings>;
    /** The summary going out now, absent outside one. */
    readonly summaryAt?: number;
    /** The changed time of the decision. */
    readonly now?: number;
}
