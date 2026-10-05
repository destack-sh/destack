import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { type Contact, decide, focus, type Settings, summary, Window } from "../src/index.ts";
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

/** A push endpoint the recipient's phone has. */
const PHONE = schema
    .identifier("push-endpoint")
    .parse("push-endpoint-019f5530-8000-7000-8000-0000000000a1");

/** The desktop the recipient signed in. */
const LAPTOP = schema.identifier("device").parse("device-019f5530-8000-7000-8000-0000000000a2");

test("decide each channel by the first rule that applies, from read state down to the email delay", () => {
    // start from an unread, active mention the recipient receives everywhere, with nothing quiet
    const delivery: Parameters<typeof decide>[0] = {
        id: schema.identifier("delivery").parse("delivery-019f5530-8000-7000-8000-0000000000a3"),
        channel: "push",
        device: null,
        endpoint: PHONE,
        isSummarized: false,
        dueAt: SATURDAY,
    };
    const notified: Parameters<typeof decide>[1] = {
        packageId: notes.id,
        occurredAt: SATURDAY,
        readAt: null,
        snoozedUntil: null,
        interruption: "active",
    };
    const contact: Contact = {
        desktops: [LAPTOP],
        endpoints: [
            { id: PHONE, url: "https://push.example/phone", keys: { p256dh: "key", auth: "auth" } },
        ],
        email: "dana@example.com",
        timeZone: "UTC",
        locale: "en",
    };
    const settings: Settings = {
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        focus: focus.definition.default,
        summary: summary.definition.default,
    };
    const focused = { ...focus.definition.default, until: SATURDAY + HOUR };
    const desktop: Changes["delivery"] = { channel: "desktop", device: LAPTOP, endpoint: null };
    const email: Changes["delivery"] = { channel: "email", endpoint: null };
    const cases: readonly [string, Changes][] = [
        ["read", { notification: { readAt: SATURDAY } }],
        ["unaddressed", { contact: { endpoints: [] } }],
        ["snoozed", { notification: { snoozedUntil: SATURDAY + HOUR } }],
        [
            "critical in a focus, turned off",
            {
                notification: { interruption: "critical" },
                settings: { focus: focused, preference: { channels: [], delivery: "immediate" } },
            },
        ],
        [
            "turned off",
            { settings: { preference: { channels: ["desktop"], delivery: "immediate" } } },
        ],
        ["passive", { notification: { interruption: "passive" } }],
        [
            "summarized",
            {
                settings: {
                    preference: { channels: ["push"], delivery: "summary" },
                    summary: { times: ["18:00"], channels: ["push"] },
                },
            },
        ],
        [
            "summarized and due",
            {
                delivery: { isSummarized: true },
                settings: {
                    preference: { channels: ["push"], delivery: "summary" },
                    summary: { times: ["18:00"], channels: ["push"] },
                },
            },
        ],
        [
            "summarized on the desktop",
            {
                delivery: desktop,
                settings: { preference: { channels: ["desktop"], delivery: "summary" } },
            },
        ],
        [
            "time-sensitive, summarized",
            {
                notification: { interruption: "timeSensitive" },
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
            { notification: { interruption: "timeSensitive" }, settings: { focus: focused } },
        ],
        [
            "time-sensitive in a strict focus",
            {
                notification: { interruption: "timeSensitive" },
                settings: { focus: { ...focused, isTimeSensitiveAllowed: false } },
            },
        ],
        ["email", { delivery: email }],
        ["email after the delay", { delivery: email, now: SATURDAY + EMAIL_DELAY }],
    ];

    // decide each case
    const decided = cases.map(([name, changes]) => [
        name,
        decide(
            { ...delivery, ...changes.delivery },
            { ...notified, ...changes.notification },
            { ...contact, ...changes.contact },
            { ...settings, ...changes.settings },
            EMAIL_DELAY,
            changes.now ?? SATURDAY,
        ),
    ]);
    expect(decided).toEqual([
        ["read", { action: "skip", reason: "read" }],
        ["unaddressed", { action: "skip", reason: "unaddressed" }],
        ["snoozed", { action: "defer", until: SATURDAY + HOUR, isSummarized: false }],
        ["critical in a focus, turned off", { action: "send" }],
        ["turned off", { action: "skip", reason: "preference" }],
        ["passive", { action: "skip", reason: "preference" }],
        [
            "summarized",
            { action: "defer", until: Date.UTC(2026, 9, 24, 18, 0), isSummarized: true },
        ],
        ["summarized and due", { action: "send" }],
        ["summarized on the desktop", { action: "skip", reason: "preference" }],
        ["time-sensitive, summarized", { action: "send" }],
        ["in a focus", { action: "defer", until: SATURDAY + HOUR, isSummarized: false }],
        ["in a focus allowing the app", { action: "send" }],
        ["time-sensitive in a focus", { action: "send" }],
        [
            "time-sensitive in a strict focus",
            { action: "defer", until: SATURDAY + HOUR, isSummarized: false },
        ],
        ["email", { action: "defer", until: SATURDAY + EMAIL_DELAY, isSummarized: false }],
        ["email after the delay", { action: "send" }],
    ]);
});

/** What one case changes of the base scenario. */
interface Changes {
    /** The delivery's changed fields. */
    readonly delivery?: Partial<Parameters<typeof decide>[0]>;
    /** The notification's changed fields. */
    readonly notification?: Partial<Parameters<typeof decide>[1]>;
    /** The contact's changed fields. */
    readonly contact?: Partial<Contact>;
    /** The changed settings. */
    readonly settings?: Partial<Settings>;
    /** The changed time of the decision. */
    readonly now?: number;
}
