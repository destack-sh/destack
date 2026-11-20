import type { Connector } from "../command/chat";
import { type Item, Scene } from "../command/scene";

/** What you ask your agent for. */
const ask: Item = {
    kind: "ask",
    text: "when I'm reading a mail, like ines's here about phase 2, show me my notes on her and when we last talked / next meet. right next to the mail pls",
};

/** What you ask once it works. */
const before: Item = {
    kind: "ask",
    text: "nice. and before a call put the open threads with them in the calendar event",
};

/** The conversation through rented tools: the facts found, the mail itself out of reach. */
export const stackedScene = Scene.of([
    ask,
    {
        kind: "call",
        server: "gmail",
        running: "Reading the thread",
        title: "Read Re: Phase 2",
        request: 'get_thread({ threadId: "19a0f3c8e2b7d4a1" })',
        response: '{ "messages": [{ "from": "Ines Ferreira", "subject": "Re: Phase 2" }, …3] }',
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "call",
        server: "notion",
        running: "Searching your notes",
        title: "Found 4 notes",
        request:
            'search({ query: "Ines Ferreira", filter: { property: "object", value: "page" } })',
        response: '{ "results": [ …4 pages ], "has_more": false }',
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "call",
        server: "googlecalendar",
        running: "Finding your meetings with her",
        title: "Found 2 meetings",
        request: 'list_events({ q: "Ines Ferreira", timeMin: "2025-10-01T00:00:00Z" })',
        response:
            '{ "items": [{ "summary": "Call · Ines", "start": "Mon 11:30" }, { "summary": "Review · Ines", "start": "Wed 11:00" }] }',
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "reply",
        text: "For Ines: 4 notes, the last one says she wants card payments at booking. You last talked Monday at 11:30 and meet again Wednesday at 11. I can't put this inside Gmail itself. The closest is a Google Workspace add-on: I'd write it in Apps Script and it would show in Gmail's side panel as a card once you deploy it, reading Notion with a token you create. Want me to set that up?",
    },
    before,
    {
        kind: "reply",
        text: "Calendar can't show live mail in an event. I can add an Apps Script trigger that runs every morning and pastes links to open threads into the event descriptions. They'd be up to a day old, and only see Gmail. Should I?",
    },
]);

/** The conversation in your space: your mail and calendar changed, each in one call. */
export const destackedScene = Scene.of([
    ask,
    {
        kind: "call",
        server: "destack",
        running: "Changing Mail",
        title: "Changed Mail",
        request: 'client.app.change("mail", { panel: "person", reads: ["notes", "calendar"] })',
        response: "person card · notes · last and next meeting",
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "app",
        title: "Mail · Ines Ferreira",
        badge: "Live",
        rows: [
            ["Notes", "4 notes · wants cards at booking", "Notes"],
            ["Last talked", "Mon 11:30 · Call", "Calendar"],
            ["Next", "Wed 11:00 · Review", "Calendar"],
        ],
        note: "the same person card People shows",
        isLive: true,
    },
    {
        kind: "reply",
        text: "Done. Your mail now shows the person card for whoever wrote the email you're reading. It's the same card your People app uses, so a note opens in Notes and a meeting in Calendar.",
    },
    before,
    {
        kind: "approval",
        text: "wants to change Calendar to show open threads in events",
        outcome: "Allowed once",
        grant: "once",
    },
    {
        kind: "call",
        server: "destack",
        running: "Changing Calendar",
        title: "Changed Calendar",
        request: 'client.app.change("calendar", { event: "threads", where: "attendees" })',
        response: "open threads with each attendee, kept current",
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "reply",
        text: "Your events now list the open threads with the people in them, and stay current as mail comes in.",
    },
]);

/** The connectors the stacked agent has on, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["gmail", "#ea4335"],
    ["googlecalendar", "#4285f4"],
    ["notion", "#191919"],
    ["slack", "#4a154b"],
    ["airtable", "#18bfff"],
    ["trello", "#0052cc"],
    ["linear", "#5e6ad2"],
    ["confluence", "#1868db"],
    ["figma", "#a259ff"],
    ["googledrive", "#1fa463"],
    ["zapier", "#ff4f00"],
];
