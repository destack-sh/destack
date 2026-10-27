import { present } from "@destack/schema";

import type { Connector } from "../command/chat";
import { type Item, type Moment, Scene } from "../command/scene";
import type { Lane } from "./session";

/** Your question on launch morning. */
const ask: Item = {
    kind: "ask",
    text: "people say they got the launch email twice, and the pricing link on the launch post is broken. what else went wrong with last night's prep?",
};

/** Your agent reading its session from last night. */
const readLog: Item = {
    kind: "call",
    server: "destack",
    running: "Reading the log since Thu 17:00",
    title: "Read the log",
    request: 'client.query.log.findMany({ where: { by: "agent", after: "Thu 17:00" } })',
    response: "1 session · Prepared launch day · 8 steps · 25 changes",
    isFailed: false,
    isExpanded: false,
};

/** The slips your agent found, live in your space. */
const slips: Item = {
    kind: "app",
    title: "To review from launch prep",
    badge: "Live · 4 apps",
    rows: [
        ["Waitlist email", "also sent to the 400 invited Thursday", "sent"],
        ["Launch post", "links to /pricing, still a draft", "live"],
        ["#launch post", "Fri 09:00 UTC is 11:00 your time", "queued"],
        ["Record the demo", "closed on last week's demo.mp4", "done"],
    ],
    note: "the other 21 changes match your request",
    isLive: true,
};

/** The same slips once fixed. */
const slipsFixed: Item = {
    kind: "app",
    title: "To review from launch prep",
    badge: "Live · 4 apps",
    rows: [
        ["Waitlist email", "also sent to the 400 invited Thursday", "sent"],
        ["Launch post", "/pricing published", "fixed"],
        ["#launch post", "moved to 09:00 your time", "fixed"],
        ["Record the demo", "reopened", "fixed"],
    ],
    note: "3 fixed · the email stays sent",
    isLive: true,
};

/** Your agent fixing the three slips it can take back. */
const fix: Item = {
    kind: "call",
    server: "destack",
    running: "Fixing 3 changes",
    title: "Fixed 3 changes",
    request:
        'pages.publish("/pricing")\nchat.schedule("#launch", "Fri 09:00 Europe/Vienna")\ntasks.reopen("DST-12")',
    response: "3 changes · logged under this chat",
    isFailed: false,
    isExpanded: true,
};

/** The conversation in your space, in order. */
const items: readonly Item[] = [
    ask,
    readLog,
    {
        kind: "reply",
        text: "Last night I made 25 changes for launch prep. Four of them aren't what you meant:",
    },
    slips,
    {
        kind: "approval",
        text: "wants to publish /pricing, move the #launch post to 09:00 your time, and reopen Record the demo.",
        outcome: "Allowed · 3 changes",
        grant: "once",
    },
    fix,
    {
        kind: "reply",
        text: "Fixed. The 400 emails can't be unsent: want me to draft a short note to just those 400?",
    },
];

/** The conversation in your space, its steps placed on one clock with the log. */
export const destackedScene = new Scene(
    items.map((item) => ({ item, row: 0, change: "start" as const })),
);

/** The conversation through the vendors' connectors: two slips found, the rest out of sight. */
export const stackedScene = new Scene(
    (
        [
            ask,
            {
                kind: "call",
                server: "gmail",
                running: "Searching sent mail",
                title: "Searched sent mail",
                request: 'search({ query: "in:sent subject:(Destack opens Friday)" })',
                response: "4 messages · 500 + 500 + 500 + 340 recipients",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "call",
                server: "airtable",
                running: "Reading Signups",
                title: "Read Signups",
                request: 'list_records({ table: "Signups", filter: "{Status} = \'invited\'" })',
                response: "400 records · invited Thu 16:10",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "reply",
                text: "The launch email went to the whole waitlist in four batches, including the 400 you invited on Thursday.",
            },
            {
                kind: "call",
                server: "slack",
                running: "Reading #launch",
                title: "Read #launch",
                request: 'conversations.history({ channel: "launch", oldest: "Thu 17:00" })',
                response: "2 messages from Florian · no scheduled messages visible",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "call",
                server: "notion",
                running: "Searching Notion",
                title: "Searched Notion",
                request: 'search({ query: "pricing" })',
                response: "Pricing · Destack site · draft · edited Thu 16:40",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "reply",
                text: "The pricing page is still a draft in Notion. I can't see the rest of what I changed last night. I keep nothing between chats, and each tool shows where things stand now, not who changed them or why.",
            },
        ] as const
    ).map((item) => ({ item, row: 0, change: "start" as const })),
);

/** The connectors the stacked agent has on, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["gmail", "#ea4335"],
    ["airtable", "#18bfff"],
    ["slack", "#4a154b"],
    ["trello", "#0052cc"],
    ["notion", "#191919"],
];

/** Return what the chat in your space shows at a beat, the live view current once fixed. */
export function destackedChatAt(beat: number | undefined): Moment {
    // swap the live view's rows in place once the slips are fixed
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
