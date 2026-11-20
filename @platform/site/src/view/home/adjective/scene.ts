import { present } from "@destack/schema";

import type { Connector } from "../command/chat";
import { type Item, type Moment, Scene } from "../command/scene";
import type { Lane } from "./session";

/** Your question in the morning. */
const ask: Item = {
    kind: "ask",
    text: "you changed my inbox last night right? show me what you did before I switch to it",
};

/** Your agent reading its session from last night. */
const readLog: Item = {
    kind: "call",
    server: "destack",
    running: "Reading the log since yesterday 22:00",
    title: "Read the log",
    request:
        'client.query.log.findMany({ where: { by: "agent", app: "mail", after: "yesterday 22:00" } })',
    response: "1 session · Tidied your inbox · 4 steps · 247 changes, all in preview",
    isFailed: false,
    isExpanded: false,
};

/** The changes your agent made, running against your real inbox in preview. */
const slips: Item = {
    kind: "app",
    title: "Your inbox, in preview",
    badge: "Preview · 4 changes",
    rows: [
        ["Needs me first", "12 threads up top", "ready"],
        ["Receipts", "31 filed, each matched to its payment", "ready"],
        ["Newsletters", "214 archived, Kai's sketch among them", "check"],
        ["Morning digest", "repeats your Monday page", "check"],
    ],
    note: "on your real inbox · nothing live yet",
    isLive: true,
};

/** The same changes once you kept three and dropped one. */
const slipsFixed: Item = {
    kind: "app",
    title: "Your inbox",
    badge: "Live · 3 changes",
    rows: [
        ["Needs me first", "12 threads up top", "live"],
        ["Receipts", "31 filed, each matched to its payment", "live"],
        ["Newsletters", "213 archived, Kai's sketch in your inbox", "live"],
        ["Morning digest", "dropped", "dropped"],
    ],
    note: "each change its own change set · kept through mail updates",
    isLive: true,
};

/** What you say once you have seen the preview. */
const keep: Item = {
    kind: "ask",
    text: "keep the first three but put kai back, drop the digest",
};

/** Your agent applying the changes you keep. */
const fix: Item = {
    kind: "call",
    server: "destack",
    running: "Applying 3 changes",
    title: "Applied 3 changes",
    request:
        'changes.apply(["sort", "receipts", "newsletters"], { except: ["Kai Brandt"] })\nchanges.drop("digest")',
    response: "3 live · 1 dropped · logged under this chat",
    isFailed: false,
    isExpanded: true,
};

/** The conversation in your space, in order. */
const items: readonly Item[] = [
    ask,
    readLog,
    {
        kind: "reply",
        text: "4 changes from last night, all still in preview. Two need a look:",
    },
    slips,
    keep,
    {
        kind: "approval",
        text: "wants to apply 3 changes to your mail, keep Kai's sketch in your inbox, and drop the digest",
        outcome: "Allowed · 3 changes",
        grant: "once",
    },
    fix,
    {
        kind: "reply",
        text: "Done. Three changes are live, Kai's mail is back in your inbox, and the digest is gone.",
    },
];

/** The conversation in your space, its steps placed on one clock with the log. */
export const destackedScene = Scene.of(items);

/** The conversation through the vendors' connectors: filters and labels, all live already. */
export const stackedScene = Scene.of([
    ask,
    {
        kind: "call",
        server: "gmail",
        running: "Reading your filters",
        title: "Read 3 filters",
        request: 'users.settings.filters.list({ userId: "me" })',
        response:
            '{ "filter": [{ "criteria": { "from": "billing" }, "action": { "addLabelIds": ["Label_Receipts"] } }, …3] }',
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "call",
        server: "gmail",
        running: "Counting the mail I moved",
        title: "Found 245 moved",
        request:
            'users.messages.list({ userId: "me", q: "label:Receipts OR in:archive newer_than:1d" })',
        response: '{ "resultSizeEstimate": 245 }',
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "reply",
        text: "Last night I added 3 filters and 2 labels and moved 245 mails. Filters have no preview, so it's all live already, and the digest is an Apps Script that runs at 8. Want me to undo any of it?",
    },
]);

/** The connectors the stacked agent has on, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["gmail", "#ea4335"],
    ["googlecalendar", "#4285f4"],
    ["googledrive", "#1fa463"],
    ["googlesheets", "#0f9d58"],
    ["notion", "#191919"],
    ["zapier", "#ff4f00"],
];

/** Return what the chat in your space shows at a beat, the live view current once fixed. */
export function destackedChatAt(beat: number | undefined): Moment {
    // swap the preview's rows for the live ones once the changes are kept
    const moment = destackedScene.at(beat);
    const isFixed = laneAt(beat ?? destackedScene.length) === "fixed";

    return isFixed
        ? { ...moment, items: moment.items.map((item) => (item === slips ? slipsFixed : item)) }
        : moment;
}

/** Return what the log shows at a beat: the session, then its slips once read, then their fixes. */
export function laneAt(beat: number): Lane {
    // flag the slips once your agent has read the log, and fix them once it fixes
    if (beat < cueOf(readLog).end) {
        return "prepared";
    } else if (beat < cueOf(fix).start + 4) {
        return "reviewed";
    }

    return "fixed";
}

/** Return where an item of the conversation sits on the clock, stretched over the step after it. */
function cueOf(item: Item): { start: number; end: number } {
    // find the item's step and the one after it
    const index = destackedScene.cues.findIndex((cue) => cue.step.item === item);
    const cue = present(destackedScene.cues[index], "scene cue");
    const next = destackedScene.cues[index + 1] ?? cue;

    return { start: cue.start, end: next.start + next.beats };
}
