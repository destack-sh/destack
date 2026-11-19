import type { Connector } from "../command/chat";
import { type Item, Scene } from "../command/scene";

/** What you ask your agent for. */
const ask: Item = {
    kind: "ask",
    text: "can you set up a press list next to the launch board and share it with maya? she's doing outreach this week",
};

/** The conversation through rented tools: a base made, the invite refused, the cards copied. */
export const stackedScene = Scene.of([
    ask,
    {
        kind: "call",
        server: "airtable",
        running: "Creating a base",
        title: "Created Press base",
        request:
            'create_base({ name: "Press", workspaceId: "wspLaunch4fQ2", tables: [{ name: "Contacts", fields: ["Name", "Outlet", "Status", "Date"] }] })',
        response:
            '{ "id": "appPr3ssK8vN2qTd", "tables": [{ "id": "tblC0ntacts9xL", "name": "Contacts" }] }',
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "call",
        server: "airtable",
        running: "Inviting Maya",
        title: "Couldn't invite Maya",
        request:
            'add_base_collaborator({ baseId: "appPr3ssK8vN2qTd", email: "maya@symbol.industries", permissionLevel: "edit" })',
        response:
            '403 { "error": { "type": "INVALID_PERMISSIONS_OR_MODEL_NOT_FOUND", "message": "Invalid permissions, or the requested model was not found. Check that both your user and your token have the required permissions, and that the model names and/or ids are correct." } }',
        isFailed: true,
        isExpanded: true,
    },
    {
        kind: "call",
        server: "trello",
        running: "Reading the launch board",
        title: "Read Launch board",
        request: 'get_cards({ boardId: "launch", list: "Press" })',
        response: "14 cards · Press list",
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "approval",
        text: "wants to copy 14 Trello cards into Press",
        outcome: "Allowed once",
        grant: "once",
    },
    {
        kind: "call",
        server: "airtable",
        running: "Copying cards",
        title: "Created 14 records",
        request:
            'create_records({ baseId: "appPr3ssK8vN2qTd", table: "Contacts", records: […14] })',
        response: '{ "records": [ …14 created ] }',
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "reply",
        text: "Press is set up in Airtable with the 14 press cards from your launch board. I couldn't share it: inviting collaborators through the API needs Airtable Enterprise, so invite Maya from the Share button, and she'll be a paid seat at $20 a month. The records are a one-time copy, so new or edited cards won't show up. Keeping them in sync would need a Zap.",
    },
]);

/** The conversation in your space: one more table, linked to the board and shared in one go. */
export const destackedScene = Scene.of([
    ask,
    {
        kind: "call",
        server: "destack",
        running: "Adding a table",
        title: "Added Press to your space",
        request: 'client.table.create({ name: "press", link: "launch" })',
        response: "each contact linked to its launch card",
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "approval",
        text: "wants to share Press with Maya as an editor",
        outcome: "Allowed once",
        grant: "once",
    },
    {
        kind: "call",
        server: "destack",
        running: "Sharing Press",
        title: "Shared Press with Maya",
        request: 'client.press.share({ person: "maya", role: "editor" })',
        response: "editor · no new seat",
        isFailed: false,
        isExpanded: false,
    },
    {
        kind: "app",
        title: "Press",
        badge: "Live · shared with Maya",
        rows: [
            ["Ines Ferreira", "Orbit Labs Weekly", "Call booked"],
            ["Kai Brandt", "Kettle Podcast", "Replied"],
            ["Lena Okoye", "Fieldnotes", "Pitched"],
            ["Ravi Patel", "Brightpath Daily", "Pitched"],
        ],
        note: "destack.app/@florian/press · Maya can edit",
        isLive: true,
    },
    {
        kind: "reply",
        text: "Done. Press sits next to the launch board, each contact linked to its card, and Maya can edit it.",
    },
]);

/** The connectors the stacked agent has on, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["airtable", "#18bfff"],
    ["trello", "#0052cc"],
    ["notion", "#191919"],
    ["slack", "#4a154b"],
    ["gmail", "#ea4335"],
    ["googlecalendar", "#4285f4"],
    ["linear", "#5e6ad2"],
    ["confluence", "#1868db"],
    ["figma", "#a259ff"],
    ["googledrive", "#1fa463"],
    ["zapier", "#ff4f00"],
];
