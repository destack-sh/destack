import { expect, test } from "@destack/test";
import { type Circumstances, decide, focus, summary, TimeZone } from "../src/index.ts";

/** How long an email waits in the scenarios: a quarter hour. */
const EMAIL_DELAY = 15 * 60_000;
import { notes } from "./fixture/document.ts";

/** Saturday 24 October 2026, 12:00 UTC, the day before Vienna leaves summer time at 01:00 UTC. */
const SATURDAY = Date.UTC(2026, 9, 24, 12, 0);

/** An hour, in milliseconds. */
const HOUR = 3_600_000;

test("place summary times and quiet hours in a time zone across the end and start of summer time", () => {
    // place 08:00 on Sunday in winter time, an hour later in UTC than on Saturday
    const sunday = TimeZone.next("Europe/Vienna", ["08:00"], SATURDAY);
    const later = TimeZone.next("Europe/Vienna", ["08:00", "18:00"], SATURDAY);

    // end quiet hours from 22:00 to 07:00 after the night the clocks go back, and defer none at noon
    const night = { days: [6], from: "22:00", to: "07:00" };
    const inside = TimeZone.end("Europe/Vienna", [night], Date.UTC(2026, 9, 25, 1, 30));
    const outside = TimeZone.end("Europe/Vienna", [night], SATURDAY);

    // move 02:30 on the night the clocks go forward past the gap, to 03:30 summer time
    const gap = TimeZone.next("Europe/Vienna", ["02:30"], Date.UTC(2026, 2, 28, 12, 0));
    expect([sunday, later, inside, outside, gap]).toEqual([
        Date.UTC(2026, 9, 25, 7, 0),
        Date.UTC(2026, 9, 24, 16, 0),
        Date.UTC(2026, 9, 25, 6, 0),
        undefined,
        Date.UTC(2026, 2, 29, 1, 30),
    ]);
    expect(() => TimeZone.next("Mars/Olympus", ["08:00"], SATURDAY)).toThrow(RangeError);
});

test("decide each channel by the first rule that applies, from access down to presence", () => {
    // start from an unread, active mention the recipient receives everywhere, with nothing quiet
    const base: Circumstances = {
        channel: "push",
        emailDelay: EMAIL_DELAY,
        notification: {
            parentPackageId: notes.id,
            occurredAt: SATURDAY,
            readAt: null,
            snoozedUntil: null,
        },
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        focus: focus.definition.default,
        summary: summary.definition.default,
        timeZone: "UTC",
        isReadable: true,
        isAddressed: true,
        isPresent: false,
        isSummaryDue: false,
        now: SATURDAY,
    };
    const focused = { ...focus.definition.default, until: SATURDAY + HOUR };
    const cases: readonly [string, Partial<Circumstances>][] = [
        ["unreadable", { isReadable: false }],
        ["read", { notification: { ...base.notification, readAt: SATURDAY } }],
        ["unaddressed", { isAddressed: false }],
        ["snoozed", { notification: { ...base.notification, snoozedUntil: SATURDAY + HOUR } }],
        [
            "critical in a focus, turned off",
            {
                interruption: "critical",
                focus: focused,
                preference: { channels: [], delivery: "immediate" },
            },
        ],
        ["turned off", { preference: { channels: ["desktop"], delivery: "immediate" } }],
        ["passive", { interruption: "passive" }],
        [
            "summarized",
            {
                preference: { channels: ["push"], delivery: "summary" },
                summary: { times: ["18:00"], channels: ["push"] },
            },
        ],
        [
            "summarized and due",
            {
                preference: { channels: ["push"], delivery: "summary" },
                summary: { times: ["18:00"], channels: ["push"] },
                isSummaryDue: true,
            },
        ],
        [
            "summarized on the desktop",
            { channel: "desktop", preference: { channels: ["desktop"], delivery: "summary" } },
        ],
        [
            "time-sensitive, summarized",
            {
                interruption: "timeSensitive",
                preference: { channels: ["push"], delivery: "summary" },
            },
        ],
        ["in a focus", { focus: focused }],
        ["in a focus allowing the app", { focus: { ...focused, allowed: [notes.id] } }],
        ["time-sensitive in a focus", { interruption: "timeSensitive", focus: focused }],
        [
            "time-sensitive in a strict focus",
            { interruption: "timeSensitive", focus: { ...focused, isTimeSensitiveAllowed: false } },
        ],
        ["email", { channel: "email" }],
        ["email after the delay", { channel: "email", now: SATURDAY + EMAIL_DELAY }],
        ["present elsewhere", { isPresent: true }],
        ["present elsewhere, on the desktop", { channel: "desktop", isPresent: true }],
    ];

    // decide each case
    expect(cases.map(([name, changes]) => [name, decide({ ...base, ...changes })])).toEqual([
        ["unreadable", { action: "skip", reason: "withheld" }],
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
        ["present elsewhere", { action: "skip", reason: "present" }],
        ["present elsewhere, on the desktop", { action: "send" }],
    ]);
});
