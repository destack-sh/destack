import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createMemo, createSignal, For, onSettled } from "@destack/view";

import { type Shift, sound } from "../effect/sound";
import { tokens } from "../style/tokens.stylex";
import {
    boardCells,
    boardInset,
    columnLefts,
    columnWidth,
    quarterCentres,
    rowCells,
} from "./board";
import { Card, type Entity, type Reveal } from "./card";

/** The milliseconds each open scene holds before the next one begins. */
const sceneTime = 5200;
/** The milliseconds from the water fully draining to the first scene change. */
const firstSceneTime = 2400;
/** The milliseconds between neighbouring cards turning over, left to right. */
const flipStagger = 150;
/** The milliseconds after the stack opens before its apps turn into view, once the silos have gone down the drain. */
const openFlipAt = 1500;
/** The milliseconds a card's ink takes to fade out before it hands over its box, or to fade in after it takes one. */
const inkFade = 320;
/** The milliseconds a card takes to turn over and show the card replacing it. */
const flipTime = 1200;
/** The milliseconds cards take to travel between places. */
const moveTime = 1400;

/** The humans, agents, silos, and apps the scenes arrange. */
const entities: Record<string, Entity> = {
    // people
    you: { label: "You", icon: "user", role: "Human", tint: "#2f7d8c" },
    cofounder: { label: "Cofounder", icon: "user", role: "Human", tint: "#a0485f" },
    designer: { label: "Designer", icon: "user", role: "Human", tint: "#5b7f2e" },
    client: { label: "Client", icon: "user", role: "Guest", tint: "#8a6d3b" },
    candidate: { label: "Candidate", icon: "user", role: "Guest", tint: "#7a6aa8" },
    investor: { label: "Investor", icon: "user", role: "Guest", tint: "#3f6f73" },
    partner: { label: "Partner", icon: "user", role: "Family", tint: "#b0567f" },
    roommate: { label: "Roommate", icon: "user", role: "Friend", tint: "#6d7f3a" },

    // agents
    agent: { label: "Agent", icon: "agent", role: "Agent", tint: "#6b5ca5" },
    chatgpt: { label: "ChatGPT", icon: "openai", role: "Agent", tint: "#10a37f" },
    claude: { label: "Claude", icon: "claude", role: "Agent", tint: "#d97757" },

    // silos, at the list price of their recommended plan, billed yearly
    notion: {
        label: "Notion",
        icon: "notion",
        role: "$20/seat/mo + credits",
        tint: "#ffffff",
        glyph: "#191919",
    },
    airtable: {
        label: "Airtable",
        icon: "airtable",
        role: "$45/seat/mo + credits",
        tint: "#18bfff",
    },
    typeform: { label: "Typeform", icon: "typeform", role: "$91/mo + credits", tint: "#262627" },
    dropbox: { label: "Dropbox", icon: "dropbox", role: "$18/seat/mo", tint: "#0061ff" },
    figma: { label: "Figma", icon: "figma", role: "$55/seat/mo + credits", tint: "#f24e1e" },
    slack: { label: "Slack", icon: "slack", role: "$15/seat/mo", tint: "#4a154b" },
    loom: { label: "Loom", icon: "loom", role: "$15/seat/mo", tint: "#625df5" },
    gdocs: { label: "Google Docs", icon: "googledocs", role: "$14/seat/mo", tint: "#4285f4" },
    calendly: { label: "Calendly", icon: "calendly", role: "$16/seat/mo", tint: "#006bff" },
    linear: { label: "Linear", icon: "linear", role: "$16/seat/mo + credits", tint: "#5e6ad2" },
    github: { label: "GitHub", icon: "github", role: "$21/seat/mo + credits", tint: "#24292f" },
    homemade: { label: "Your app", icon: "tasks", role: "Homemade", tint: "#b8862b" },

    // apps
    pages: { label: "Pages", icon: "pages", role: "App", tint: "#3d6fb0" },
    notes: { label: "Notes", icon: "file", role: "App", tint: "#b8862b" },
    chat: { label: "Chat", icon: "chat", role: "App", tint: "#4f8a5b" },
    tasks: { label: "Tasks", icon: "tasks", role: "App", tint: "#c64a17" },
    source: { label: "Source", icon: "source", role: "App", tint: "#24292f" },
    crm: { label: "CRM", icon: "user", role: "App", tint: "#a0485f" },
    forms: { label: "Forms", icon: "template", role: "App", tint: "#6b5ca5" },
    calendar: { label: "Calendar", icon: "calendar", role: "App", tint: "#c64a17" },
    sheets: { label: "Sheets", icon: "table", role: "App", tint: "#4f8a5b" },
    mail: { label: "Mail", icon: "mail", role: "App", tint: "#3d6fb0" },
    expenses: { label: "Expenses", icon: "vault", role: "App", tint: "#8a6d3b" },
    files: { label: "Files", icon: "bucket", role: "App", tint: "#2f7d8c" },

    // remixes, each named after the apps it joins
    standup: { label: "Standup", icon: "chat", role: "Chat + Tasks", tint: "#4f8a5b" },
    incidents: {
        label: "Incidents",
        icon: "notify",
        role: "Chat + Tasks + Source",
        tint: "#c64a17",
    },
    feedback: {
        label: "Feedback loop",
        icon: "sync",
        role: "Forms + Tasks + CRM",
        tint: "#6b5ca5",
    },
    hiring: { label: "Hiring", icon: "user", role: "Forms + CRM + Calendar", tint: "#a0485f" },
    update: {
        label: "Investor update",
        icon: "vector",
        role: "Sheets + Pages + Mail",
        tint: "#3f6f73",
    },
    invoices: { label: "Invoices", icon: "table", role: "Sheets + CRM + Mail", tint: "#8a6d3b" },
    household: {
        label: "Household",
        icon: "storage",
        role: "Sheets + Expenses + Files",
        tint: "#2f7d8c",
    },
    contacts: {
        label: "Personal CRM",
        icon: "user",
        role: "Notes + Calendar + Mail",
        tint: "#b0567f",
    },
};

/** The apps each remix joins, in the order its role names them. */
const remixes: Readonly<Record<string, readonly string[]>> = {
    standup: ["chat", "tasks"],
    incidents: ["chat", "tasks", "source"],
    feedback: ["forms", "tasks", "crm"],
    hiring: ["forms", "crm", "calendar"],
    update: ["sheets", "pages", "mail"],
    invoices: ["sheets", "crm", "mail"],
    household: ["sheets", "expenses", "files"],
    contacts: ["notes", "calendar", "mail"],
};

/** How each person or agent gets into the silos of a locked scene today, one line per silo from left to right. */
const todayAccess: Readonly<Record<string, readonly [string, string, string]>> = {
    you: ["2FA code", "magic link", "SSO"],
    cofounder: ["admin", "seat pending", "billing owner"],
    designer: ["guest", "no seat", "viewer"],
    client: ["guest invite", "shared channel", "no access"],
    candidate: ["public form", "no access", "email only"],
    investor: ["PDF export", "no access", "forwarded"],
    partner: ["no seat", "no access", "your password"],
    roommate: ["public link", "no access", "no access"],
    agent: ["API 403", "paid tier", "MCP token"],
    chatgpt: ["connector", "read only", "no access"],
    claude: ["MCP token", "no access", "connector"],
};

/** The agent's balance in each silo that meters its AI in credits. */
const creditBalances: Readonly<Record<string, string>> = {
    notion: "12 credits left",
    figma: "out of credits",
    airtable: "resets in 9 days",
    linear: "top up $10",
    github: "out of AI credits",
    typeform: "enrichment credits",
    homemade: "out of credits",
};

/** What the homemade app rents under the searchlight today. */
const homemadeReveal: Reveal = {
    kind: "fields",
    rows: [
        ["Lovable", "credits"],
        ["Supabase", "$25/mo"],
        ["Vercel", "$20/seat/mo"],
    ],
};

/** What each card shows under the searchlight with Destack: one identity and its grants, or the app's source. */
const openReveals: { [id: string]: Reveal | undefined } = {
    // one account each, with just the grants it needs
    you: {
        kind: "fields",
        rows: [
            ["account", "one"],
            ["apps", "owner"],
            ["agents", "3 granted"],
        ],
    },
    cofounder: {
        kind: "fields",
        rows: [
            ["account", "one"],
            ["tasks", "admin"],
            ["crm", "edit"],
        ],
    },
    designer: {
        kind: "fields",
        rows: [
            ["account", "one"],
            ["tasks", "edit"],
            ["source", "review"],
        ],
    },
    client: {
        kind: "fields",
        rows: [
            ["account", "guest"],
            ["forms", "submit"],
            ["the rest", "hidden"],
        ],
    },
    candidate: {
        kind: "fields",
        rows: [
            ["account", "guest"],
            ["calendar", "book"],
            ["the rest", "hidden"],
        ],
    },
    investor: {
        kind: "fields",
        rows: [
            ["account", "guest"],
            ["update", "read"],
            ["the rest", "hidden"],
        ],
    },
    partner: {
        kind: "fields",
        rows: [
            ["account", "one"],
            ["household", "edit"],
            ["contacts", "read"],
        ],
    },
    roommate: {
        kind: "fields",
        rows: [
            ["account", "one"],
            ["household", "edit"],
            ["the rest", "hidden"],
        ],
    },
    agent: {
        kind: "fields",
        rows: [
            ["tasks", "triage"],
            ["source", "read"],
            ["deploy", "asks first"],
        ],
    },
    chatgpt: {
        kind: "fields",
        rows: [
            ["sheets", "analyse"],
            ["mail", "draft"],
            ["send", "asks first"],
        ],
    },
    claude: {
        kind: "fields",
        rows: [
            ["incidents", "triage"],
            ["crm", "read"],
            ["send", "asks first"],
        ],
    },

    // the apps' own source
    pages: code("pages.tsx", "export function Pages() {", "  return <List of={pages} />;", "}"),
    notes: code("notes.tsx", "export function Notes() {", "  return <List of={notes} />;", "}"),
    chat: code("chat.tsx", "export function Chat() {", "  return <Thread of={messages} />;", "}"),
    tasks: code("tasks.tsx", "export function Tasks() {", "  return <Board of={tasks} />;", "}"),
    source: code("source.tsx", "export function Source() {", "  return <Log of={commits} />;", "}"),
    crm: code("crm.tsx", "export function Crm() {", "  return <List of={people} />;", "}"),
    forms: code("forms.tsx", "export function Forms() {", "  return <Form of={fields} />;", "}"),
    calendar: code(
        "calendar.tsx",
        "export function Calendar() {",
        "  return <Month of={events} />;",
        "}",
    ),
    sheets: code("sheets.tsx", "export function Sheets() {", "  return <Grid of={cells} />;", "}"),
    mail: code("mail.tsx", "export function Mail() {", "  return <Thread of={mail} />;", "}"),
    expenses: code(
        "expenses.tsx",
        "export function Expenses() {",
        "  return <Ledger of={costs} />;",
        "}",
    ),
    files: code("files.tsx", "export function Files() {", "  return <List of={files} />;", "}"),

    // each remix, joining its apps' data
    standup: code("standup.tsx", "<Split>", "  <Chat /> <Tasks due={today} />", "</Split>"),
    incidents: code(
        "incidents.tsx",
        "<Room of={alert}>",
        "  <Chat /> <Tasks /> <Commits />",
        "</Room>",
    ),
    feedback: code(
        "feedback.ts",
        'forms.on("submit", (reply) =>',
        "  tasks.create({ for: reply.customer })",
        ");",
    ),
    hiring: code(
        "hiring.tsx",
        "<Pipeline of={candidates}>",
        "  <Form /> <Calendar book />",
        "</Pipeline>",
    ),
    update: code(
        "update.tsx",
        "<Update month={last}>",
        "  <Chart of={sheets.metrics} />",
        "</Update>",
    ),
    invoices: code(
        "invoices.ts",
        "crm.clients.map((client) =>",
        "  mail.send(invoice(client.hours))",
        ");",
    ),
    household: code(
        "household.tsx",
        "<Split between={flatmates}>",
        "  <Expenses /> <Files of={receipts} />",
        "</Split>",
    ),
    contacts: code(
        "contacts.tsx",
        "<People sort={lastSpoke}>",
        "  <Notes /> <Calendar /> <Mail />",
        "</People>",
    ),
};

/** Every card the scenes can show. */
const ids = Object.keys(entities);
/** The silos each iceberg carries in turn, from the left berg to the right. */
export const slotApps: readonly (readonly string[])[] = [
    ["notion", "figma", "typeform", "airtable", "dropbox"],
    ["slack", "loom", "calendly", "gdocs"],
    ["linear", "github", "homemade"],
];
/** The iceberg each silo rides. */
const slots = new Map(slotApps.flatMap((apps, slot) => apps.map((id) => [id, slot] as const)));
/** The silos, which ride the icebergs today. */
const vendors = [...slots.keys()];
/** The milliseconds a silo takes to bob up after the one it replaces starts to sink. */
const swapDelay = 1000;
/** The milliseconds a silo waits to land on the reformed ice after the water returns. */
const landingDelay = 2700;
/** The open apps and remixes, which anyone can fork. */
const apps = ids.filter((id) => entities[id].role === "App" || id in remixes);

/** One arrangement of the top two layers. */
type Scene = {
    /** The cards on the upper row, left to right. */
    upper: readonly string[];
    /** The cards on the lower row, left to right, and how many of the three columns each spans. */
    lower: readonly { id: string; span: number }[];
    /** Which upper card works with which lower card. */
    links: readonly [string, string][];
};

/** The locked stack today, one change per step: a silo swaps on its iceberg or someone new signs in, and the logins reshuffle. */
export const todayScenes: readonly Scene[] = [
    locked(
        ["you", "cofounder", "designer", "agent"],
        ["notion", "slack", "linear"],
        ["notion", "slack", "linear", "linear"],
    ),
    locked(
        ["you", "cofounder", "designer", "agent"],
        ["notion", "slack", "github"],
        ["notion", "slack", "slack", "github"],
    ),
    locked(
        ["you", "cofounder", "designer", "agent"],
        ["figma", "slack", "github"],
        ["github", "slack", "figma", "github"],
    ),
    locked(
        ["you", "cofounder", "designer", "claude"],
        ["figma", "loom", "github"],
        ["figma", "loom", "figma", "github"],
    ),
    locked(
        ["you", "cofounder", "client", "claude"],
        ["figma", "loom", "github"],
        ["loom", "github", "figma", "github"],
    ),
    locked(
        ["you", "cofounder", "client", "claude"],
        ["typeform", "loom", "linear"],
        ["linear", "loom", "typeform", "linear"],
    ),
    locked(
        ["you", "cofounder", "candidate", "claude"],
        ["typeform", "calendly", "linear"],
        ["calendly", "linear", "typeform", "calendly"],
    ),
    locked(
        ["you", "cofounder", "investor", "chatgpt"],
        ["airtable", "calendly", "homemade"],
        ["airtable", "homemade", "calendly", "airtable"],
    ),
    locked(
        ["you", "cofounder", "investor", "chatgpt"],
        ["airtable", "gdocs", "homemade"],
        ["homemade", "gdocs", "gdocs", "airtable"],
    ),
    locked(
        ["you", "partner", "roommate", "chatgpt"],
        ["airtable", "gdocs", "homemade"],
        ["airtable", "gdocs", "airtable", "homemade"],
    ),
    locked(
        ["you", "partner", "roommate", "chatgpt"],
        ["dropbox", "gdocs", "homemade"],
        ["dropbox", "gdocs", "dropbox", "homemade"],
    ),
    locked(
        ["you", "partner", "roommate", "agent"],
        ["dropbox", "slack", "linear"],
        ["slack", "dropbox", "slack", "linear"],
    ),
];

/** The open loop: everyone shares the apps, and each step joins apps into a remix or brings the next set of apps in. */
export const scenes: readonly Scene[] = [
    // team: a standup, then an incident room
    open(
        ["you", "cofounder", "designer", "agent"],
        ["chat", "tasks", "source"],
        ["tasks", "chat", "tasks", "source"],
    ),
    open(
        ["you", "cofounder", "designer", "agent"],
        ["standup", "source"],
        ["standup", "standup", "standup", "source"],
    ),
    open(
        ["you", "cofounder", "designer", "claude"],
        ["chat", "tasks", "source"],
        ["source", "chat", "tasks", "tasks"],
    ),
    open(
        ["you", "cofounder", "designer", "claude"],
        ["incidents"],
        ["incidents", "incidents", "incidents", "incidents"],
    ),

    // startup: close the loop with customers, then hire
    open(
        ["you", "cofounder", "client", "claude"],
        ["forms", "tasks", "crm"],
        ["tasks", "crm", "forms", "tasks"],
    ),
    open(
        ["you", "cofounder", "client", "claude"],
        ["feedback"],
        ["feedback", "feedback", "feedback", "feedback"],
    ),
    open(
        ["you", "cofounder", "candidate", "claude"],
        ["forms", "crm", "calendar"],
        ["calendar", "crm", "forms", "crm"],
    ),
    open(
        ["you", "cofounder", "candidate", "claude"],
        ["hiring"],
        ["hiring", "hiring", "hiring", "hiring"],
    ),

    // founder and freelancer: report to investors, then bill clients
    open(
        ["you", "cofounder", "investor", "chatgpt"],
        ["sheets", "pages", "mail"],
        ["sheets", "pages", "mail", "sheets"],
    ),
    open(
        ["you", "cofounder", "investor", "chatgpt"],
        ["update"],
        ["update", "update", "update", "update"],
    ),
    open(
        ["you", "designer", "client", "chatgpt"],
        ["sheets", "crm", "mail"],
        ["sheets", "crm", "mail", "sheets"],
    ),
    open(
        ["you", "designer", "client", "chatgpt"],
        ["invoices"],
        ["invoices", "invoices", "invoices", "invoices"],
    ),

    // life: run a household, then keep up with people
    open(
        ["you", "partner", "roommate", "chatgpt"],
        ["sheets", "expenses", "files"],
        ["sheets", "expenses", "files", "expenses"],
    ),
    open(
        ["you", "partner", "roommate", "chatgpt"],
        ["household"],
        ["household", "household", "household", "household"],
    ),
    open(
        ["you", "partner", "roommate", "agent"],
        ["notes", "calendar", "mail"],
        ["notes", "calendar", "mail", "mail"],
    ),
    open(
        ["you", "partner", "roommate", "agent"],
        ["contacts"],
        ["contacts", "contacts", "contacts", "contacts"],
    ),
];

/** The services each open app calls, each with the store that service keeps the app's state in, by label; remixes use their apps' own. */
export const appUses: Readonly<Record<string, readonly (readonly [string, string])[]>> =
    withRemixes({
        pages: [
            ["Access", "DB"],
            ["Search", "Bucket"],
        ],
        notes: [
            ["Access", "DB"],
            ["Search", "DB"],
        ],
        chat: [
            ["Access", "DB"],
            ["AI", "Vault"],
        ],
        tasks: [
            ["Access", "DB"],
            ["Settings", "DB"],
        ],
        source: [
            ["Access", "Audit"],
            ["Search", "Bucket"],
        ],
        crm: [
            ["Access", "DB"],
            ["Search", "DB"],
        ],
        forms: [
            ["Access", "DB"],
            ["Settings", "DB"],
        ],
        calendar: [
            ["Access", "DB"],
            ["Settings", "DB"],
        ],
        sheets: [
            ["Access", "DB"],
            ["AI", "Vault"],
        ],
        mail: [
            ["Access", "DB"],
            ["Search", "DB"],
            ["AI", "Vault"],
        ],
        expenses: [
            ["Access", "Audit"],
            ["Settings", "DB"],
        ],
        files: [
            ["Access", "Bucket"],
            ["Search", "Bucket"],
        ],
    });

/** The source step each open scene shows at work, by its label: installing a set of apps, then building its remix. */
export const sceneSources = scenes.map((scene, index) =>
    scene.lower.some((card) => card.id in remixes)
        ? "Build"
        : ["Registry", "Repository", "Templates"][Math.floor(index / 2) % 3],
);

/** How far the user row sits below the middle of its figure row, in CSS pixels, to leave room for the switch on the seam above it. */
const userDrop = 8;

/** The hole row the hosts' tops sit on, counted from the layer's top. */
const hostTop = 47;
/** The hole row the cables from the build run across to the hosts. */
const hostBus = 44.5;

/** The board's height in cells across the three figure rows the layer covers. */
const layerRows = rowCells * 3;

/** The placement of every card in each scene of the locked stack. */
const todayPlacements = todayScenes.map(arrange);
/** The placement of every card in each open scene. */
const scenePlacements = scenes.map(arrange);

/** How a card moves into its placement. */
type Step = "stay" | "enter" | "leave" | "park" | "fuse" | "flip" | "drain";

/** One placed card: its row, its span in whole board cells, and how it gets there. */
type Placement = {
    /** The row, upper or lower. */
    row: 0 | 1;
    /** The span's left edge in cells from the board's edge. */
    left: number;
    /** The span's width in cells. */
    width: number;
    /** Whether the card shows on the board. */
    isShown: boolean;
    /** How the card moves there: stays or travels on the board, enters or leaves across an edge, fuses into the cards replacing it, or parks out of sight. */
    step: Step;
    /** The milliseconds the card waits before it moves. */
    delay: number;
    /** Whether the card shows or hides at the end of its move rather than at its start. */
    isLate: boolean;
    /** How far the card is turned about its horizontal axis, in degrees: flipped away while it swaps places with another card in its slot. */
    turn?: number;
};

/** One kind of cable, which sets its look. */
type Kind = "locked" | "link" | "chain" | "drop";

/** A point in layer pixels. */
type Point = { x: number; y: number };

/**
 * A right-angled cable run between two ends.
 *
 * The bend is where its middle segment crosses, down then across then down, or across then down then across.
 */
type Run = { from: Point; to: Point; bend: number };

/** A card's edges and centre in layer pixels. */
type Box = { left: number; right: number; top: number; bottom: number; centre: number };

/** One cable between two cards, its springy middle, and how visible it is. */
type Cable = {
    /** The drawn cable. */
    path: SVGPathElement;
    /** The plugs at both ends. */
    ends: SVGPathElement;
    /** The straight-line middle on the previous frame, to feel how fast the ends move. */
    middle: Point | undefined;
    /** How far the middle swings off the straight line. */
    offset: Point;
    /** How fast the middle swings. */
    velocity: Point;
    /** The visibility from 0 to 1. */
    alpha: number;
    /** The time the cable starts to fade in. */
    showsAt: number;
    /** How far the cable has settled onto the grid, from 0 following its cards to 1 resting in the holes. */
    settle: number;
};

/** Show the top two layers, locked today or freely rearranging with Destack, every card draggable. */
export function Remix(properties: {
    isOpen: boolean;
    today: number;
    isLive: boolean;
    revealOf: (row: number) => number;
    onScene: (scene: number) => void;
}) {
    // hold the scene, the dragged card, and the elements to measure
    const [scene, setScene] = createSignal(0);
    const [drag, setDrag] = createSignal<{ id: string; x: number; y: number }>();
    let layer!: HTMLDivElement;
    let wires!: SVGGElement;
    const cards = new Map<string, HTMLDivElement>();
    const last = new Map<string, Placement>();

    // pick the scene on show: the locked stack today, the loop once open
    const shown = () => (properties.isOpen ? scenes[scene()] : todayScenes[properties.today]);

    // place every card of the scene: cards enter and leave across the nearest board edge, vendor apps sink in place
    let wasLaidOpen = properties.isOpen;
    const layout = createMemo(() => {
        // pick the placements and note whether the stack just opened or closed
        const current = properties.isOpen
            ? scenePlacements[scene()]
            : todayPlacements[properties.today];
        const isToggle = properties.isOpen !== wasLaidOpen;
        wasLaidOpen = properties.isOpen;

        // step each card toward its placement, collecting the cards that enter and leave across an edge, and those that morph
        const before = new Map(last);
        const entering: Placement[] = [];
        const leaving: Placement[] = [];
        const morphAt = isToggle ? 700 : 0;
        for (const id of ids) {
            const placement = current.get(id);
            const previous = last.get(id);
            const isVendor = vendors.includes(id);

            // keep or move a card that shows, growing it out of the cards it replaces, and taking over only at the end of a merge
            if (placement) {
                const isEntering = previous !== undefined && !previous.isShown && !isVendor;
                const isGrowing = isEntering && isOnBoard(previous);
                const isFlipping = isGrowing && previous.turn !== undefined;
                const next: Placement = {
                    ...placement,
                    step: isFlipping ? "flip" : isEntering ? "enter" : "stay",
                    delay: isVendor
                        ? vendorDelay(previous, isToggle)
                        : isFlipping
                          ? (isToggle ? openFlipAt : 0) + flipStagger * slotOf(placement)
                          : isGrowing
                            ? morphAt
                            : 0,
                    isLate: isGrowing && sourcesOf(placement, before, current) > 1,
                };
                last.set(id, next);
                if (isEntering && !isGrowing) {
                    entering.push(next);
                }
            }
            // sink a vendor app where it stands, or swirl it down the drain with the water when the stack opens
            else if (isVendor) {
                last.set(id, {
                    ...(previous ?? lockedPlacement(id)),
                    isShown: false,
                    step: isToggle ? "drain" : "stay",
                    delay: 0,
                    isLate: false,
                });
            }
            // fuse a card that showed into the cards replacing it, or send it off across its nearest edge
            else if (previous?.isShown) {
                const successors = [...current]
                    .filter(
                        ([other, place]) =>
                            !before.get(other)?.isShown && overlaps(place, previous),
                    )
                    .map(([, place]) => place);
                // flap every open card away on its hinge as the stack closes, before the water comes back
                if (isToggle && !properties.isOpen) {
                    last.set(id, {
                        ...previous,
                        isShown: false,
                        step: "flip",
                        delay: flipStagger * slotOf(previous) + previous.row * flipStagger * 2,
                        isLate: false,
                        turn: 90,
                    });
                    continue;
                }

                // turn a card over to show the one card that takes its slot
                if (
                    successors.length === 1 &&
                    sourcesOf(successors[0], before, current) === 1 &&
                    sameSlot(successors[0], previous)
                ) {
                    last.set(id, {
                        ...previous,
                        isShown: false,
                        step: "flip",
                        delay: flipStagger * slotOf(previous),
                        isLate: false,
                        turn: 90,
                    });
                } else if (successors.length) {
                    const isMerging =
                        successors.length === 1 && sourcesOf(successors[0], before, current) > 1;
                    last.set(id, {
                        ...cover(successors),
                        step: "fuse",
                        delay: morphAt,
                        isLate: isMerging,
                    });
                } else {
                    const next: Placement = { ...beyond(previous), step: "leave", isLate: true };
                    last.set(id, next);
                    leaving.push(next);
                }
            }
            // leave a card that is turning away alone until it is gone
            else if (previous?.step === "flip" && previous.turn !== undefined) {
                continue;
            }
            // park a hidden card on the cards it replaces when it next appears, or beyond the edge nearest where it appears
            else {
                const next = upcoming(id, scene());
                const following = properties.isOpen
                    ? scenePlacements[(scene() + 1) % scenes.length]
                    : scenePlacements[0];
                const origins = following.has(id)
                    ? [...current]
                          .filter(
                              ([other, place]) => !following.has(other) && overlaps(place, next),
                          )
                          .map(([, place]) => place)
                    : [];
                const isSwapping = origins.length === 1 && sameSlot(origins[0], next);
                last.set(id, {
                    ...(isSwapping ? next : origins.length ? cover(origins) : beyond(next)),
                    isShown: false,
                    step: "park",
                    isLate: false,
                    turn: isSwapping ? -90 : undefined,
                });
            }
        }

        // start entering cards after the leaving ones clear, farthest from its edge first; send leaving cards nearest first
        const enterAt = isToggle ? 700 : moveTime * 0.45;
        const leaveAt = isToggle ? 1200 : 0;
        stagger(entering, enterAt, (place) => -edgeDistance(place));
        stagger(leaving, leaveAt, edgeDistance);

        return new Map(last);
    });

    // follow a dragged card with the pointer, then let it spring home
    const grab = (id: string, event: PointerEvent) => {
        // hold the pointer start and pick the card up
        const startX = event.clientX;
        const startY = event.clientY;
        event.preventDefault();
        sound.play("lift");
        setDrag({ id, x: 0, y: 0 });

        // follow the pointer until it lifts, then let go of the card
        const move = (moving: PointerEvent) =>
            setDrag({ id, x: moving.clientX - startX, y: moving.clientY - startY });
        const drop = () => {
            // let go of the card and stop following the pointer
            sound.play("set");
            setDrag(undefined);
            window.removeEventListener("pointermove", move);
            window.removeEventListener("pointerup", drop);
            window.removeEventListener("pointercancel", drop);
        };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", drop);
        window.addEventListener("pointercancel", drop);
    };

    // trace the cables every frame once the cards are in the page
    onSettled(() => {
        // hold the cables, the frame, the scene timing, and the motion state
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        const cables = new Map<string, Cable>();
        let frame: number | undefined;
        let nextScene: number | undefined;
        let wasOpen = properties.isOpen;
        let wasScene = scene();
        let wasToday = properties.today;
        let changedAt = -Infinity;
        let delay = 0;
        let wakeUntil = 0;
        let isStirring = false;
        let isVisible = true;
        const narrowScreen = window.matchMedia("(max-width: 1099px)");

        // measure the boxes of the given cards within the layer, all before any cable is drawn
        const measure = (ids: Iterable<string>) => {
            // measure each card against the layer bounds
            const bounds = layer.getBoundingClientRect();
            const boxes = new Map<string, Box>();
            for (const id of ids) {
                const card = cards.get(id)?.firstElementChild?.firstElementChild;
                if (card) {
                    const rect = card.getBoundingClientRect();
                    boxes.set(id, {
                        left: rect.left - bounds.left,
                        right: rect.right - bounds.left,
                        top: rect.top - bounds.top,
                        bottom: rect.bottom - bounds.top,
                        centre: (rect.left + rect.right) / 2 - bounds.left,
                    });
                }
            }

            return { bounds, boxes };
        };

        // start a cable the first time it is needed
        const cable = (key: string, kind: Kind) => {
            let found = cables.get(key);
            if (!found) {
                const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
                const ends = document.createElementNS("http://www.w3.org/2000/svg", "path");
                path.setAttribute("class", stylex.attrs(styles.cable, cableKinds[kind]).class!);
                ends.setAttribute("class", stylex.attrs(plugKinds[kind]).class!);
                wires.append(path, ends);
                found = {
                    path,
                    ends,
                    middle: undefined,
                    offset: { x: 0, y: 0 },
                    velocity: { x: 0, y: 0 },
                    alpha: 0,
                    showsAt: 0,
                    settle: 0,
                };
                cables.set(key, found);
            }

            return found;
        };

        // fade a cable in once its moment in the choreography comes
        const show = (found: Cable, fade: number, now: number) => {
            // fade the cable in after the delay since the last change
            if (found.alpha === 0 && found.showsAt < changedAt) {
                found.showsAt = changedAt + delay;
            }
            if (now >= found.showsAt) {
                found.alpha = Math.min(1, found.alpha + 0.06);
            }
            const opacity = String(found.alpha * fade);
            found.path.style.opacity = opacity;
            found.ends.style.opacity = opacity;
            isStirring ||= found.alpha < 1;
        };

        // push a cable's middle against the motion of its ends, spring it back, and return how far it swings
        const swing = (found: Cable, middle: Point) => {
            // swing the middle against its ends' motion and damp it back
            const previous = found.middle ?? middle;
            found.middle = middle;
            const velocity = found.velocity;
            velocity.x =
                (velocity.x - (middle.x - previous.x) * 0.9 - found.offset.x * 0.09) * 0.84;
            velocity.y =
                (velocity.y - (middle.y - previous.y) * 0.9 - found.offset.y * 0.09) * 0.84;
            found.offset.x += velocity.x;
            found.offset.y += velocity.y;
            isStirring ||= Math.abs(velocity.x) + Math.abs(velocity.y) > 0.02;

            return found.offset;
        };

        // hang a locked cable between two cards as a taut curve that swings as they bob
        const hang = (key: string, from: Point, to: Point, now: number) => {
            // draw the plugs and a curve bent by the swing
            const found = cable(key, "locked");
            show(found, 1, now);
            found.ends.setAttribute("d", `${plug(from)}${plug(to)}`);
            const offset = swing(found, { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 });
            const half = (to.y - from.y) / 2;
            const bend = { x: offset.x * 1.33, y: offset.y * 1.33 };
            found.path.setAttribute(
                "d",
                `M${from.x} ${from.y}C${from.x + bend.x} ${from.y + half + bend.y} ${to.x + bend.x} ${to.y - half + bend.y} ${to.x} ${to.y}`,
            );
        };

        // run an open cable at right angles, following its cards while they move and easing into the holes once they rest
        const run = (
            key: string,
            kind: Kind,
            free: Run,
            grid: Run,
            isResting: boolean,
            isAcross: boolean,
            fade: number,
            now: number,
        ) => {
            // start the cable and fade it in
            const found = cable(key, kind);
            show(found, fade, now);

            // blend the free run toward the grid run as the cable settles
            const target = isResting ? 1 : 0;
            found.settle += (target - found.settle) * 0.14;
            if (Math.abs(target - found.settle) < 0.002) {
                found.settle = target;
            }
            isStirring ||= found.settle !== target;
            const settle = found.settle;
            const mix = (start: number, end: number) => start + (end - start) * settle;
            const from = { x: mix(free.from.x, grid.from.x), y: mix(free.from.y, grid.from.y) };
            const to = { x: mix(free.to.x, grid.to.x), y: mix(free.to.y, grid.to.y) };

            // swing the middle segment across its length while the cable is loose
            const offset = swing(found, { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 });
            const bend =
                mix(free.bend, grid.bend) + (isAcross ? offset.x : offset.y) * 1.33 * (1 - settle);

            // draw the plugs and the right-angled path
            found.ends.setAttribute("d", `${plug(from)}${plug(to)}`);
            found.path.setAttribute(
                "d",
                isAcross
                    ? `M${from.x} ${from.y}H${bend}V${to.y}H${to.x}`
                    : `M${from.x} ${from.y}V${bend}H${to.x}V${to.y}`,
            );
        };

        // trace every cable from the cards' measured positions
        const trace = (now: number) => {
            // measure the cards the scene links and reset the motion flag
            const current = shown();
            const isOpen = properties.isOpen;
            const needed = new Set([
                ...current.links.flat(),
                ...current.lower.map((card) => card.id),
            ]);
            const { bounds, boxes } = measure(needed);
            const kept = new Set<string>();
            const places = layout();
            const cell = bounds.width / boardCells;
            const cellRow = bounds.height / layerRows;
            isStirring = false;

            // snap a position across to the middle of the hole column it falls in
            const hole = (x: number) => (Math.floor(x / cell) + 0.5) * cell;

            // tell whether a card rests in its place, neither held nor travelling
            const rests = (id: string, box: Box) => {
                const place = places.get(id);
                const top = cellRow * (place?.row === 0 ? 2 : 11);

                return (
                    place !== undefined &&
                    drag()?.id !== id &&
                    Math.abs(box.centre - centre(place, cell)) < 1.5 &&
                    Math.abs(box.top - top) < 1.5
                );
            };

            // run every cable across a gap along one tidy bus: its middle, or above the row captions on narrow screens
            const share = narrowScreen.matches ? captionedBus : 0.5;
            const middle = (top: number, bottom: number) => top + (bottom - top) * share;

            // link the upper cards to the lower cards they work with: once open, each person runs straight down, across on their own lane, and down into the app
            const kind = isOpen ? "link" : "locked";
            current.links.forEach(([upper, lower]) => {
                // find both cards of the link, skipping links whose cards are missing
                const from = boxes.get(upper);
                const to = boxes.get(lower);
                if (!from || !to) {
                    return;
                }
                const key = `${kind}:${upper}:${lower}`;
                kept.add(key);
                const start = { x: from.centre, y: from.bottom };
                const end = { x: to.centre, y: to.top };
                if (!isOpen) {
                    hang(key, start, end, now);
                    return;
                }
                const bend = middle(from.bottom, to.top);
                run(
                    key,
                    kind,
                    { from: start, to: end, bend },
                    {
                        from: { x: hole(from.centre), y: from.bottom },
                        to: {
                            x: hole(to.centre),
                            y: to.top,
                        },
                        bend,
                    },
                    rests(upper, from) && rests(lower, to),
                    false,
                    1,
                    now,
                );
            });

            // once open, chain the lower cards along a hole row, drop each into the service it calls, and link each service to its store
            if (isOpen) {
                const reveal = properties.revealOf(2);
                const stored = properties.revealOf(3);
                const item = (label: string) => {
                    const found = layer.parentElement
                        ?.querySelector(`[data-item="${label}"]`)
                        ?.getBoundingClientRect();

                    return found
                        ? {
                              left: found.left - bounds.left,
                              width: found.width,
                              centre: (found.left + found.right) / 2 - bounds.left,
                              top: found.top - bounds.top,
                              bottom: found.bottom - bounds.top,
                          }
                        : undefined;
                };

                // list the calls into the services and the links down to the stores
                const calls = current.lower.flatMap((card) =>
                    [...new Set(appUses[card.id].map(([service]) => service))].map((service) => ({
                        id: card.id,
                        service,
                    })),
                );
                const links = [
                    ...new Map(
                        current.lower
                            .flatMap((card) => appUses[card.id])
                            .map(([service, store]) => [`${service}:${store}`, { service, store }]),
                    ).values(),
                ];
                const lower = current.lower.flatMap((card) => {
                    const found = boxes.get(card.id);
                    return found
                        ? [{ id: card.id, isResting: rests(card.id, found), ...found }]
                        : [];
                });
                lower.forEach((card, index) => {
                    // chain this card to the next one along the hole row
                    const next = lower[index + 1];
                    if (next) {
                        const key = `chain:${card.id}:${next.id}`;
                        kept.add(key);
                        const start = { x: card.right, y: (card.top + card.bottom) / 2 };
                        const end = { x: next.left, y: (next.top + next.bottom) / 2 };
                        const y = cellRow * 13.5;
                        run(
                            key,
                            "chain",
                            { from: start, to: end, bend: (start.x + end.x) / 2 },
                            {
                                from: { x: start.x, y },
                                to: { x: end.x, y },
                                bend: (start.x + end.x) / 2,
                            },
                            card.isResting && next.isResting,
                            true,
                            reveal,
                            now,
                        );
                    }

                    // drop this card into every service it calls, spread along both edges, turning lower for later services
                    const own = calls.filter((call) => call.id === card.id);
                    own.forEach((call) => {
                        // find the service cell, or skip a service its band does not show
                        const service = item(call.service);
                        if (!service) {
                            return;
                        }
                        const key = `drop:${card.id}:${call.service}`;
                        kept.add(key);
                        const place = places.get(card.id);
                        const anchor = {
                            x: hole(service.centre),
                            y: service.top,
                        };
                        const x = hole(place ? centre(place, cell) : card.centre);
                        const bend = middle(card.bottom, anchor.y);
                        run(
                            key,
                            "drop",
                            { from: { x: card.centre, y: card.bottom }, to: anchor, bend },
                            { from: { x, y: card.bottom }, to: anchor, bend },
                            card.isResting,
                            false,
                            reveal,
                            now,
                        );
                    });
                });

                // link each service the scene calls down to each store it keeps the scene's state in
                for (const link of links) {
                    const service = item(link.service);
                    const store = item(link.store);
                    if (!service || !store) {
                        continue;
                    }
                    const key = `store:${link.service}:${link.store}`;
                    kept.add(key);
                    const from = { x: hole(service.centre), y: service.bottom };
                    const to = {
                        x: hole(store.centre),
                        y: store.top,
                    };
                    const bend = middle(service.bottom, store.top);
                    const path = { from, to, bend };
                    run(key, "drop", path, path, true, false, stored, now);
                }

                // feed every host from the build
                const build = item("Build");
                if (build) {
                    quarterCentres.forEach((column, index) => {
                        // run down from the build, across the bus, and down into this host
                        const key = `host:${index}`;
                        kept.add(key);
                        const path = {
                            from: { x: build.centre, y: build.bottom },
                            to: { x: hole(cell * column), y: cellRow * hostTop },
                            bend: cellRow * hostBus,
                        };
                        run(key, "drop", path, path, true, false, properties.revealOf(5), now);
                    });
                }
            }

            // fade out cables the scene no longer uses, and remove them once gone
            for (const [key, found] of cables) {
                if (kept.has(key)) {
                    continue;
                }
                isStirring = true;
                found.alpha = Math.max(0, found.alpha - 0.12);
                found.path.style.opacity = String(found.alpha);
                found.ends.style.opacity = String(found.alpha);
                if (found.alpha === 0) {
                    found.path.remove();
                    found.ends.remove();
                    cables.delete(key);
                }
            }
        };

        // time the cables to the choreography, and step the scenes once the water has drained
        const loop = (now: number) => {
            // restart the cable timing when the stack opens, closes, or changes scene
            if (properties.isOpen !== wasOpen) {
                changedAt = now;
                delay = properties.isOpen ? 1100 : 3000;
                wasOpen = properties.isOpen;
            } else if (scene() !== wasScene || properties.today !== wasToday) {
                changedAt = now;
                delay = moveTime * 0.8;
                soundShifts(layout());
            }
            wasScene = scene();
            wasToday = properties.today;

            // reset the scenes while locked, and step them while open and live
            if (!properties.isOpen) {
                nextScene = undefined;
                if (scene() !== 0) {
                    setScene(0);
                    properties.onScene(0);
                }
            } else if (properties.isLive && !isStill) {
                nextScene ??= now + firstSceneTime;
                if (drag()) {
                    nextScene = Math.max(nextScene, now + 1200);
                } else if (now >= nextScene) {
                    const next = (scene() + 1) % scenes.length;
                    setScene(next);
                    properties.onScene(next);
                    nextScene = now + sceneTime;
                }
            }

            // trace only while something moves: bobbing ice, dancing users, travelling cards, a drag, or a swinging cable
            const isBobbing = !properties.isOpen && !isStill;
            const isBusy =
                isBobbing || drag() || now - changedAt < 4500 || now < wakeUntil || isStirring;
            if (isVisible && isBusy) {
                trace(now);
            }
            if (drag()) {
                wakeUntil = now + 1500;
            }
            frame = requestAnimationFrame(loop);
        };
        frame = requestAnimationFrame(loop);

        // retrace after the layout changes, and rest while off screen
        const sizes = new ResizeObserver(() => {
            wakeUntil = performance.now() + 300;
        });
        sizes.observe(layer);
        const sight = new IntersectionObserver(([entry]) => {
            isVisible = entry.isIntersecting;
        });
        sight.observe(layer);

        return () => {
            sizes.disconnect();
            sight.disconnect();
            if (frame !== undefined) {
                cancelAnimationFrame(frame);
            }
        };
    });

    return (
        <div
            ref={layer}
            style={{
                height: `calc(${tokens.cellRow} * ${layerRows})`,
                width: `calc(${tokens.cell} * ${boardCells})`,
            }}
            {...stylex.attrs(styles.layer)}
        >
            <svg aria-hidden="true" {...stylex.attrs(styles.lines)}>
                <g ref={wires} />
            </svg>

            {/* place each card on its row; cards travel between rows and can be dragged */}
            <For each={ids}>
                {(id) => {
                    // read the card's placement, drag offset, and kind
                    const placement = () => layout().get(id)!;
                    const held = () => (drag()?.id === id ? drag() : undefined);
                    const slot = slots.get(id);
                    const isVendor = slot !== undefined;
                    const isPerson = !isVendor && !apps.includes(id);

                    return (
                        <div
                            ref={(element) => cards.set(id, element)}
                            onPointerDown={(event) => grab(id, event)}
                            style={{
                                ...span(placement()),
                                top:
                                    placement().row === 0
                                        ? `calc(100% / 6 + ${userDrop}px)`
                                        : "50%",
                                ...follow(motion(isVendor, placement()), held()),
                            }}
                            class={stylex.attrs(styles.card, held() && styles.held).class}
                        >
                            <div
                                style={{
                                    "--toward":
                                        id === "you" || id === "designer" || id === "roommate"
                                            ? "1"
                                            : "-1",
                                    "animation-delay": `${-ids.indexOf(id) * 0.9}s`,
                                    transform: isVendor
                                        ? "translate(var(--sway, 0px), var(--lift, 0px)) rotate(var(--tilt, 0deg))"
                                        : undefined,
                                }}
                                data-bob={isVendor ? String(slot) : undefined}
                                class={
                                    stylex.attrs(
                                        styles.fill,
                                        isPerson && styles.dance,
                                        isPerson && properties.isOpen && styles.still,
                                    ).class
                                }
                            >
                                <Card
                                    entity={entities[id]}
                                    kind={isVendor ? "vendor" : "plain"}
                                    reveal={
                                        properties.isOpen
                                            ? openReveals[id]
                                            : todayReveal(id, todayScenes[properties.today])
                                    }
                                    style={styles.fill}
                                />
                            </div>
                        </div>
                    );
                }}
            </For>
        </div>
    );
}

/** How far down a gap the cable bus runs on narrow screens, as a share of the gap, clear of the row captions below it. */
const captionedBus = 0.3;
/** The easing of a card travelling between places. */
const easing = "cubic-bezier(0.6, 0, 0.2, 1)";
/** The easing of a card springing back from a drag. */
const spring = "cubic-bezier(0.3, 1.45, 0.5, 1)";
/** The milliseconds a silo takes to swirl down the drain as the stack opens. */
const drainTime = 1900;
/** How far below the silo row the drain lies, in hole rows: down through the figure to the footer. */
const drainDrop = 52;
/** How far a silo sinks below its berg before it is gone, deep enough for the water to hide it. */
const sinkDepth = "8rem";
/** The easing of a vendor app sinking with the ice. */
const sink = "cubic-bezier(0.5, 0, 0.9, 0.6)";

/** Return the left edge and width of a card's span, as CSS lengths within the layer. */
function span(place: Placement) {
    return {
        left: `calc(${tokens.cell} * ${place.left})`,
        width: `calc(${tokens.cell} * ${place.width})`,
    };
}

/** Return the centre of a card's span in pixels, for a given cell width. */
function centre(place: Placement, cell: number) {
    return (place.left + place.width / 2) * cell;
}

/** Return a card's inline motion: how it enters and leaves. */
function motion(isVendor: boolean, place: Placement): Record<string, string> {
    // read the card's visibility
    const isShown = place.isShown;

    // sink a silo under the water and fade it as it goes deep, and raise the next one out of it onto its berg
    if (isVendor) {
        return isShown
            ? {
                  opacity: "1",
                  translate: "0 0",
                  rotate: "0deg",
                  scale: "1",
                  transition: `opacity 500ms ease ${place.delay}ms, translate 1100ms ${spring} ${place.delay}ms, rotate 1100ms ${spring} ${place.delay}ms, scale 900ms ${spring} ${place.delay}ms`,
              }
            : place.step === "drain"
              ? {
                    opacity: "0",
                    translate: `calc(${tokens.cell} * ${boardCells / 2 - place.left - place.width / 2}) calc(${tokens.cellRow} * ${drainDrop})`,
                    rotate: `${place.left < boardCells / 3 ? 220 : -220}deg`,
                    scale: "0.08",
                    "pointer-events": "none",
                    transition: `translate ${drainTime}ms cubic-bezier(0.55, 0, 0.8, 0.4), rotate ${drainTime}ms cubic-bezier(0.4, 0, 0.9, 0.6), scale ${drainTime}ms cubic-bezier(0.7, 0, 0.9, 0.5), opacity 500ms ease ${drainTime - 500}ms`,
                }
              : {
                    opacity: "0",
                    translate: `0 ${sinkDepth}`,
                    rotate: "5deg",
                    scale: "1",
                    "pointer-events": "none",
                    transition: `translate 1300ms ${sink}, rotate 1300ms ${sink}, opacity 700ms ease 500ms`,
                };
    }

    // turn a card over in its slot: the old one turns away, and the new one turns into view from behind it
    if (place.step === "flip" || place.turn !== undefined) {
        const half = flipTime / 2;
        const at = place.delay + (isShown ? half : 0);
        return {
            "--ink": "1",
            "--ink-time": "0ms",
            opacity: isShown ? "1" : "0",
            rotate: `x ${isShown ? 0 : (place.turn ?? 90)}deg`,
            "transform-origin": "50% 0",
            translate: "0 0",
            "z-index": isShown ? "2" : "1",
            transition:
                place.step === "park"
                    ? "none"
                    : `rotate ${half}ms ${isShown ? "cubic-bezier(0.2, 0.7, 0.3, 1.2)" : "cubic-bezier(0.6, 0, 0.9, 0.5)"} ${at}ms, opacity 0ms linear ${place.delay + half}ms`,
            ...(isShown ? {} : { "pointer-events": "none" }),
        };
    }

    // travel cards whole or morph them into each other: a card going away fades its ink out, then hands its box over to the card
    //  taking its place, whose ink fades in, so the boxes never blink
    const swap = place.isLate ? place.delay + moveTime : place.delay + inkFade;
    const move = `left ${moveTime}ms ${easing} ${place.delay}ms, top ${moveTime}ms ${easing} ${place.delay}ms, width ${moveTime}ms ${easing} ${place.delay}ms, opacity 0ms linear ${place.step === "stay" ? 0 : swap}ms`;
    const ink = isShown
        ? {
              "--ink": "1",
              "--ink-time": `${inkFade}ms`,
              "--ink-delay": `${place.step === "stay" ? 0 : place.isLate ? swap : place.delay + moveTime * 0.3}ms`,
          }
        : { "--ink": "0", "--ink-time": `${inkFade}ms`, "--ink-delay": `${swap - inkFade}ms` };

    return {
        ...ink,
        rotate: "x 0deg",
        opacity: isShown ? "1" : "0",
        "z-index": isShown ? "2" : "1",
        translate: "0 0",
        transition: place.step === "park" ? "none" : move,
        ...(place.isShown ? {} : { "pointer-events": "none" }),
    };
}

/** Sound the cards moving in a remix: turning over, fusing, and sliding in and out, once per kind and moment. */
function soundShifts(places: ReadonlyMap<string, Placement>) {
    const heard = new Set<string>();
    for (const place of places.values()) {
        // pick the sounding move and when it happens: a turn halfway through, a fuse as it lands, a slide as it starts
        let shift: { kind: Shift; at: number } | undefined;
        if (place.step === "flip" && place.isShown) {
            shift = { kind: "flip", at: place.delay + flipTime / 2 };
        } else if (place.step === "fuse" && !place.isShown) {
            shift = { kind: "fuse", at: place.delay + moveTime * 0.8 };
        } else if (place.step === "enter" && place.isShown) {
            shift = { kind: "enter", at: place.delay };
        } else if (place.step === "leave" && !place.isShown) {
            shift = { kind: "leave", at: place.delay };
        }

        // sound each kind once per moment, where across the board it happens
        const key = shift && `${shift.kind}:${Math.round(shift.at / 80)}`;
        if (shift && key && !heard.has(key)) {
            heard.add(key);
            sound.shift(shift.kind, shift.at / 1000, (place.left + place.width / 2) / boardCells);
        }
    }
}

/** Add the drag offset to a card's motion: the card follows the pointer while held and springs home when let go. */
function follow(
    motion: Record<string, string>,
    held: { x: number; y: number } | undefined,
): Record<string, string> {
    // spring the offset home on its own, apart from the entrance and exit delays
    const offset = held ? `translate(${held.x}px, ${held.y}px)` : "translate(0px, 0px)";
    const release = held ? "transform 0ms" : `transform 700ms ${spring}`;

    return {
        ...motion,
        transform: `${offset} translateY(-50%)`,
        transition: motion.transition === "none" ? release : `${motion.transition}, ${release}`,
    };
}

/** Return a small square plug at a cable end, as an SVG path. */
function plug(point: Point) {
    return `M${point.x - 2} ${point.y - 2}h4v4h-4Z`;
}

/**
 * Place every card a scene shows in whole board cells.
 *
 * The upper row shares the board evenly with four-cell gaps; the lower row snaps to the three board columns.
 */
function arrange(scene: Scene): Map<string, Placement> {
    // share the upper row evenly between its cards
    const placed = new Map<string, Placement>();
    const gap = 3;
    const width =
        (boardCells - boardInset * 2 - gap * (scene.upper.length - 1)) / scene.upper.length;
    scene.upper.forEach((id, index) => {
        const left = boardInset + index * (width + gap);
        placed.set(id, {
            row: 0,
            left,
            width,
            isShown: true,
            step: "stay",
            delay: 0,
            isLate: false,
        });
    });

    // lay the lower row's cards across the columns they span
    let column = 0;
    for (const card of scene.lower) {
        const left = columnLefts[column];
        const end = columnLefts[column + card.span - 1] + columnWidth;
        placed.set(card.id, {
            row: 1,
            left,
            width: end - left,
            isShown: true,
            step: "stay",
            delay: 0,
            isLate: false,
        });
        column += card.span;
    }

    return placed;
}

/** Return how long a silo waits to show: landing on the reformed ice after the water returns, bobbing up in a swap, or staying put. */
function vendorDelay(previous: Placement | undefined, isToggle: boolean) {
    // land after the ice reforms
    if (isToggle) {
        return landingDelay;
    }
    // bob up after the silo it replaces starts to sink
    else if (!previous?.isShown) {
        return swapDelay;
    }
    // stay where it floats
    else {
        return 0;
    }
}

/** Return a silo's place on its iceberg, from the first scene of the locked stack that shows it. */
function lockedPlacement(id: string): Placement {
    // find the first locked scene with the silo
    const placement = todayPlacements.find((placements) => placements.has(id))?.get(id);
    if (!placement) {
        throw new Error(`silo ${id} never rides an iceberg`);
    }

    return placement;
}

/** Return how many cards that showed before and are gone now a placement replaces. */
function sourcesOf(
    place: Placement,
    before: ReadonlyMap<string, Placement>,
    current: ReadonlyMap<string, Placement>,
) {
    return [...before].filter(
        ([id, previous]) => previous.isShown && !current.has(id) && overlaps(previous, place),
    ).length;
}

/** Return which third of the board a placement starts in, from 0 at the left to 3 at the right. */
function slotOf(place: Placement) {
    return Math.round((place.left / boardCells) * 4);
}

/** Return whether two placements take the same slot: the same row, edge, and width. */
function sameSlot(first: Placement, second: Placement) {
    return (
        first.row === second.row &&
        Math.abs(first.left - second.left) < 0.5 &&
        Math.abs(first.width - second.width) < 0.5
    );
}

/** Return whether two placements on the same row overlap. */
function overlaps(first: Placement, second: Placement) {
    return (
        first.row === second.row &&
        first.left < second.left + second.width &&
        second.left < first.left + first.width
    );
}

/** Return a hidden placement covering all the given placements on their row. */
function cover(places: Placement[]): Placement {
    const left = Math.min(...places.map((place) => place.left));
    const right = Math.max(...places.map((place) => place.left + place.width));

    return {
        row: places[0].row,
        left,
        width: right - left,
        isShown: false,
        step: "stay",
        delay: 0,
        isLate: false,
    };
}

/** Return whether a placement lies on the board rather than beyond an edge. */
function isOnBoard(place: Placement) {
    return place.left >= 0 && place.left + place.width <= boardCells;
}

/** Return how far a placement sits from the board edge it enters and leaves by, in cells. */
function edgeDistance(place: Placement) {
    const middle = place.left + place.width / 2;
    return middle <= boardCells / 2 ? place.left : boardCells - place.left - place.width;
}

/** Return a placement moved just beyond the board edge nearest to it, hidden. */
function beyond(place: Placement): Placement {
    const middle = place.left + place.width / 2;
    const left = middle <= boardCells / 2 ? -place.width - 2 : boardCells + 2;

    return { ...place, left, isShown: false, delay: 0 };
}

/** Delay each card by its turn: the lowest key goes first, then one every 180 milliseconds after a start. */
function stagger(places: Placement[], start: number, key: (place: Placement) => number) {
    const order = [...places].sort((first, second) => key(first) - key(second));
    order.forEach((place, turn) => {
        place.delay = start + turn * 180;
    });
}

/** Return where a card next appears after an open scene, or in the locked stack. */
function upcoming(id: string, from: number): Placement {
    // search the scenes that follow in order
    for (let step = 1; step <= scenes.length; step++) {
        const placement = scenePlacements[(from + step) % scenes.length].get(id);
        if (placement) {
            return placement;
        }
    }

    // find the card in the locked stack
    const locked = todayPlacements[0].get(id);
    if (locked) {
        return locked;
    }

    // fail when no scene shows the card
    throw new Error(`card ${id} never appears`);
}

/** Build a locked scene: the people above, one silo per iceberg below, and the silo each person signs in to. */
function locked(
    upper: readonly string[],
    lower: readonly [string, string, string],
    targets: readonly string[],
): Scene {
    return {
        upper,
        lower: lower.map((id) => ({ id, span: 1 })),
        links: upper.map((id, index) => [id, targets[index]] as [string, string]),
    };
}

/** Build an open scene: the people above, the apps below each as wide as the apps it joins, and the app each person works in. */
function open(
    upper: readonly string[],
    lower: readonly string[],
    targets: readonly string[],
): Scene {
    return {
        upper,
        lower: lower.map((id) => ({ id, span: remixes[id]?.length ?? 1 })),
        links: upper.map((id, index) => [id, targets[index]] as [string, string]),
    };
}

/** Return a code reveal: a file name and its lines. */
function code(name: string, ...lines: string[]): Reveal {
    return { kind: "code", name, lines };
}

/** Add each remix's services and stores, the union of the apps it joins. */
function withRemixes(
    uses: Record<string, readonly (readonly [string, string])[]>,
): Record<string, readonly (readonly [string, string])[]> {
    // join the pairs of each remix's apps, once each
    const joined = Object.entries(remixes).map(([remix, parts]) => {
        const pairs = parts.flatMap((part) => uses[part]);
        const unique = [...new Map(pairs.map((pair) => [pair.join(":"), pair])).values()];

        return [remix, unique] as const;
    });

    return { ...uses, ...Object.fromEntries(joined) };
}

/** Return what a card shows under the searchlight today: a person's way into each silo on show, the agent's credits where a silo meters them, the silos' ciphertext, or the homemade app's rent. */
function todayReveal(id: string, scene: Scene): Reveal | undefined {
    // show how the person gets into each silo of the scene
    const access = todayAccess[id];
    if (access) {
        return {
            kind: "fields",
            rows: scene.lower.map((card, index) => [
                entities[card.id].label,
                (id === "agent" && creditBalances[card.id]) || access[index],
            ]),
        };
    }
    // rent for the homemade app, ciphertext for the silos
    else if (id === "homemade") {
        return homemadeReveal;
    } else if (vendors.includes(id)) {
        return { kind: "cipher" };
    }

    return undefined;
}

/** The sway of the people cards dancing while locked. */
const dance = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0) rotate(0deg)" },
    "50%": {
        transform:
            "translate(calc(var(--toward) * 2px * var(--sway)), calc(-1.5px * var(--sway))) rotate(calc(var(--toward) * 0.6deg * var(--sway)))",
    },
});

/** The cable strokes for each cable kind. */
const cableKinds = stylex.create({
    locked: { stroke: color.mutedForeground },
    link: { stroke: color.primary },
    chain: { stroke: color.primary },
    drop: { stroke: color.primary },
});

/** The plug fills for each cable kind. */
const plugKinds = stylex.create({
    locked: { fill: color.mutedForeground },
    link: { fill: color.primary },
    chain: { fill: color.primary },
    drop: { fill: color.primary },
});

/** The remix layer styles. */
const styles = stylex.create({
    layer: {
        clipPath: "inset(-100vh 0)",
        perspective: "900px",
        left: 0,
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        zIndex: 1,
    },
    lines: {
        height: "100%",
        inset: 0,
        overflow: "visible",
        pointerEvents: "none",
        position: "absolute",
        width: "100%",
    },
    cable: {
        fill: "none",
        strokeWidth: 1.25,
    },
    card: {
        cursor: "grab",
        display: "flex",
        justifyContent: "center",
        pointerEvents: "auto",
        position: "absolute",
        touchAction: "none",
        userSelect: "none",
    },
    held: {
        cursor: "grabbing",
        zIndex: 2,
    },
    dance: {
        animationDuration: "6.5s",
        animationIterationCount: "infinite",
        animationName: dance,
        animationTimingFunction: "ease-in-out",
        transition: `--sway 900ms ${easing}`,
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    still: {
        "--sway": "0",
        animationPlayState: "paused",
    },
    fill: {
        width: "100%",
    },
});
