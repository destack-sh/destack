import type { Connector } from "../command/chat";
import { type Item, Scene } from "../command/scene";

/** What you ask your agent for. */
const ask: Item = {
    kind: "ask",
    text: "add a press list next to the launch board, and share it with Maya",
};

/** The conversation through rented tools: one more tool, one more seat, one more connector. */
export const stackedScene = Scene.of([
    ask,
    {
        kind: "call",
        server: "airtable",
        running: "Creating a base",
        title: "Created Press base",
        request: 'create_base({ name: "Press", workspace: "Launch" })',
        response: "Press · fields Outlet, Contact, Status, Date",
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "reply",
        text: "I created a Press base in your Launch workspace with Outlet, Contact, Status and Date fields. I can't invite Maya through the API, so share it from Airtable's Share button. On your plan she'll count as a paid seat. Linking entries to the Trello board would need an automation, for example a Zapier zap.",
    },
]);

/** The conversation in your space: one more table, linked and shared in one go. */
export const destackedScene = Scene.of([
    ask,
    {
        kind: "call",
        server: "destack",
        running: "Adding a table",
        title: "Added Press to your space",
        request: 'client.table.create({ name: "press", link: "launch" })',
        response: "linked to the launch board · Maya can edit",
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "reply",
        text: "Done. Press is a new table next to the launch board, linked to its cards, and Maya can edit it.",
    },
]);

/** The connectors the stacked agent has on, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["airtable", "#18bfff"],
    ["trello", "#0052cc"],
    ["zapier", "#ff4f00"],
];
