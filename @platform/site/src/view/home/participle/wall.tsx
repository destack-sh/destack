import { media } from "@destack/style/media.stylex";
import {
    flat,
    monthlyOf,
    priceOf,
    type Price,
    sumOf,
    type Workload,
    workloadOf,
} from "../figure/price";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createEffect } from "@destack/view";

import { agentPrompt, setupGuide } from "../../content/site";
import { Favicon } from "../figure/glyph";
import { Tile } from "../figure/tile";
import { accentOf, type Entry, type Line } from "../figure/item";
import { ledgerSizes } from "../figure/ledger.stylex";
import { screen } from "../../layout/screen.stylex";
import { isMotionReduced } from "@destack/view/motion";

/** An app every space comes with: its name and its icon. */
type App = { name: string; icon: string };

/** The apps a space comes with, by the job they do. */
const apps = {
    pages: { name: "Pages", icon: "pages" },
    tasks: { name: "Tasks", icon: "tasks" },
    calendar: { name: "Calendar", icon: "calendar" },
    files: { name: "Files", icon: "bucket" },
    tables: { name: "Tables", icon: "table" },
    canvas: { name: "Canvas", icon: "canvas" },
    workflows: { name: "Workflows", icon: "sync" },
    chat: { name: "Chat", icon: "chat" },
    mail: { name: "Mail", icon: "mail" },
} satisfies Record<string, App>;

/** The job each app does, as the tally labels its row. */
const jobLabels: Record<Job, string> = {
    pages: "Docs",
    tables: "Databases",
    tasks: "Tasks",
    calendar: "Calendar",
    chat: "Chat",
    mail: "Mail",
    files: "Files",
    canvas: "Whiteboard",
    workflows: "Automation",
};

/** A job a space's app does. */
type Job = keyof typeof apps;

/** The apps every space holds, one to each kind of thing it keeps, in the space's order. */
const spaceJobs: readonly Job[] = [
    "pages",
    "tables",
    "tasks",
    "calendar",
    "chat",
    "mail",
    "files",
    "canvas",
];

/** The tally's rows for the services your space works with. */
const connections: readonly (readonly [label: string, names: readonly string[], app: App])[] = [
    [
        "Agents",
        ["ChatGPT", "Claude", "Gemini", "DeepSeek", "Grok"],
        { name: "Agents", icon: "agent" },
    ],
];

/** The lines of the bill. */
export const lineCount = spaceJobs.length + 1 + connections.length;

/** A tool's face on the wall: its icon, its name, and its tint. */
type Face = { icon: string; name: string; tint: string };

/** A tool you may use today and what your space does with it. */
type Tool =
    | (Face & { kind: "replace"; job: Job })
    | (Face & { kind: "both"; job: Job; status: string })
    | (Face & { kind: "connect"; status: string });

/** The tools your space replaces with its own apps. */
const replaced: readonly Tool[] = [
    { icon: "notion", name: "Notion", tint: "#000000", kind: "replace", job: "pages" },
    { icon: "googledocs", name: "Docs", tint: "#4285f4", kind: "replace", job: "pages" },
    { icon: "confluence", name: "Confluence", tint: "#172b4d", kind: "replace", job: "pages" },
    { icon: "linear", name: "Linear", tint: "#5e6ad2", kind: "replace", job: "tasks" },
    { icon: "jira", name: "Jira", tint: "#0052cc", kind: "replace", job: "tasks" },
    { icon: "asana", name: "Asana", tint: "#f06a6a", kind: "replace", job: "tasks" },
    { icon: "trello", name: "Trello", tint: "#0052cc", kind: "replace", job: "tasks" },
    { icon: "clickup", name: "ClickUp", tint: "#7b68ee", kind: "replace", job: "tasks" },
    { icon: "calendly", name: "Calendly", tint: "#006bff", kind: "replace", job: "calendar" },
    { icon: "caldotcom", name: "Cal.com", tint: "#292929", kind: "replace", job: "calendar" },
    { icon: "dropbox", name: "Dropbox", tint: "#0061ff", kind: "replace", job: "files" },
    { icon: "airtable", name: "Airtable", tint: "#18bfff", kind: "replace", job: "tables" },
    { icon: "googlesheets", name: "Sheets", tint: "#34a853", kind: "replace", job: "tables" },
    { icon: "coda", name: "Coda", tint: "#f46a54", kind: "replace", job: "tables" },
    { icon: "excel", name: "Excel", tint: "#107c41", kind: "replace", job: "tables" },
    { icon: "superhuman", name: "Superhuman", tint: "#000000", kind: "replace", job: "mail" },
    { icon: "thunderbird", name: "Thunderbird", tint: "#0a84ff", kind: "replace", job: "mail" },
    { icon: "miro", name: "Miro", tint: "#050038", kind: "replace", job: "canvas" },
    { icon: "excalidraw", name: "Excalidraw", tint: "#6965db", kind: "replace", job: "canvas" },
    { icon: "figma", name: "FigJam", tint: "#a259ff", kind: "replace", job: "canvas" },
    { icon: "mural", name: "Mural", tint: "#ff0b47", kind: "replace", job: "canvas" },
    { icon: "lucid", name: "Lucid", tint: "#282c33", kind: "replace", job: "canvas" },
    { icon: "box", name: "Box", tint: "#0061d5", kind: "replace", job: "files" },
    { icon: "obsidian", name: "Obsidian", tint: "#7c3aed", kind: "replace", job: "pages" },
    { icon: "word", name: "Word", tint: "#185abd", kind: "replace", job: "pages" },
    { icon: "n8n", name: "n8n", tint: "#ea4b71", kind: "replace", job: "workflows" },
    { icon: "zapier", name: "Zapier", tint: "#ff4f00", kind: "replace", job: "workflows" },
    { icon: "make", name: "Make", tint: "#6d00cc", kind: "replace", job: "workflows" },
    {
        icon: "powerautomate",
        name: "Power Automate",
        tint: "#0066ff",
        kind: "replace",
        job: "workflows",
    },
];

/** The tools your space can replace or keep connected. */
const either: readonly Tool[] = [
    {
        icon: "gmail",
        name: "Gmail",
        tint: "#ea4335",
        kind: "both",
        job: "mail",
        status: "inbox synced",
    },
    {
        icon: "googlecalendar",
        name: "Calendar",
        tint: "#4285f4",
        kind: "both",
        job: "calendar",
        status: "two-way sync",
    },
    {
        icon: "outlook",
        name: "Outlook",
        tint: "#0078d4",
        kind: "both",
        job: "mail",
        status: "inbox synced",
    },
    {
        icon: "outlook",
        name: "Outlook Calendar",
        tint: "#0078d4",
        kind: "both",
        job: "calendar",
        status: "two-way sync",
    },
    {
        icon: "onedrive",
        name: "OneDrive",
        tint: "#0078d4",
        kind: "both",
        job: "files",
        status: "mirrored",
    },
    {
        icon: "googledrive",
        name: "Drive",
        tint: "#4285f4",
        kind: "both",
        job: "files",
        status: "mirrored",
    },
    {
        icon: "slack",
        name: "Slack",
        tint: "#4a154b",
        kind: "both",
        job: "chat",
        status: "messages bridged",
    },
    {
        icon: "discord",
        name: "Discord",
        tint: "#5865f2",
        kind: "both",
        job: "chat",
        status: "messages bridged",
    },
    {
        icon: "teams",
        name: "Teams",
        tint: "#5059c9",
        kind: "both",
        job: "chat",
        status: "messages bridged",
    },
    {
        icon: "googlechat",
        name: "Google Chat",
        tint: "#00ac47",
        kind: "both",
        job: "chat",
        status: "messages bridged",
    },
    {
        icon: "whatsapp",
        name: "WhatsApp",
        tint: "#25d366",
        kind: "both",
        job: "chat",
        status: "messages bridged",
    },
    {
        icon: "protonmail",
        name: "Proton Mail",
        tint: "#6d4aff",
        kind: "both",
        job: "mail",
        status: "inbox synced",
    },
    {
        icon: "icloud",
        name: "iCloud",
        tint: "#3693f3",
        kind: "both",
        job: "files",
        status: "mirrored",
    },
];

/** The agents your space connects to, which work in it as you. */
const foundations: readonly Tool[] = [
    {
        icon: "openai",
        name: "ChatGPT",
        tint: "#000000",
        kind: "connect",
        status: "your agent",
    },
    {
        icon: "claude",
        name: "Claude",
        tint: "#d97757",
        kind: "connect",
        status: "your agent",
    },
    {
        icon: "gemini",
        name: "Gemini",
        tint: "#8e75b2",
        kind: "connect",
        status: "your agent",
    },
    {
        icon: "deepseek",
        name: "DeepSeek",
        tint: "#4d6bfe",
        kind: "connect",
        status: "your agent",
    },
    {
        icon: "grok",
        name: "Grok",
        tint: "#000000",
        kind: "connect",
        status: "your agent",
    },
];

/** The order the wall sets its tools in, within each row. */
const order: readonly string[] = [
    "Notion",
    "Docs",
    "Word",
    "Confluence",
    "Obsidian",
    "Airtable",
    "Sheets",
    "Excel",
    "Coda",
    "Linear",
    "Jira",
    "Asana",
    "Trello",
    "ClickUp",
    "Calendar",
    "Outlook Calendar",
    "Calendly",
    "Cal.com",
    "Slack",
    "Teams",
    "Discord",
    "Google Chat",
    "WhatsApp",
    "Gmail",
    "Outlook",
    "Superhuman",
    "Thunderbird",
    "Proton Mail",
    "Drive",
    "OneDrive",
    "Dropbox",
    "iCloud",
    "Box",
    "Miro",
    "FigJam",
    "Mural",
    "Lucid",
    "Excalidraw",
    "Zapier",
    "Make",
    "n8n",
    "Power Automate",
    "ChatGPT",
    "Claude",
    "Gemini",
    "DeepSeek",
    "Grok",
];

/** Every tool on the wall, in the wall's order. */
const tools: readonly Tool[] = order.map((name) =>
    present(
        [...replaced, ...either, ...foundations].find((tool) => tool.name === name),
        `wall tool ${name}`,
    ),
);

/** The tools picked before the reader picks any. */
export const firstPicks: readonly string[] = [
    "Notion",
    "Calendly",
    "Linear",
    "Dropbox",
    "Slack",
    "Claude",
];

/** The subscription each tool bills under when it shares one. */
const billedUnder: Readonly<Record<string, string>> = {
    Docs: "Google Workspace",
    Sheets: "Google Workspace",
    Calendar: "Google Workspace",
    Drive: "Google Workspace",
    Gmail: "Google Workspace",
    "Google Chat": "Google Workspace",
    Teams: "Microsoft 365",
    Outlook: "Microsoft 365",
    "Power Automate": "Microsoft 365",
    "Outlook Calendar": "Microsoft 365",
    OneDrive: "Microsoft 365",
    Word: "Microsoft 365",
    Excel: "Microsoft 365",
};

/** The tools whose mark is a picture from the vendor, by name. */
const pictures: Record<string, string> = {
    Superhuman: "/logos/superhuman.png",
};

/** Return the subscriptions a set of tools bills under, each once. */
function subscriptionsOf(picked: readonly Tool[]): readonly string[] {
    return [...new Set(picked.map((tool) => billedUnder[tool.name] ?? tool.name))];
}

/** The price of a service the space works with. */
const vendorBilled: Price = flat(0, "billed by its vendor");

/** Price one tally row: the subscriptions its tools add to those billed in the rows above it. */
function rowPriceOf(doers: readonly Tool[], billed: ReadonlySet<string>): Price {
    // bill nothing for a job nothing does yet
    const subscriptions = subscriptionsOf(doers);
    const [first] = subscriptions;
    if (first === undefined) {
        return flat(0, "nothing picked");
    }

    // bill only the subscriptions new to this row
    const added = subscriptions.filter((subscription) => !billed.has(subscription));
    const [only] = added;
    if (only === undefined) {
        return { kind: "bundled", basis: `in ${first}` };
    } else if (added.length === 1) {
        return priceOf(only);
    }

    return {
        kind: "fee",
        monthlyAt: (scale) => monthlyOf(added.map(priceOf), scale),
        basis: `${added.length} subscriptions`,
    };
}

/** Draw a tool's mark: its picture from the vendor, or its shape from the icon set in its tint. */
function ToolMark(properties: { tool: Tool; size: number }) {
    const picture = () => pictures[properties.tool.name];

    return picture() === undefined ? (
        <Favicon icon={properties.tool.icon} tint={properties.tool.tint} size={properties.size} />
    ) : (
        <img
            alt=""
            src={picture()}
            style={{ height: `${properties.size}px`, width: `${properties.size}px` }}
            {...style.attrs(styles.picture)}
        />
    );
}

/** Return the job of a tool your space can take over, or none when it only connects to it. */
function jobOf(tool: Tool): Job | undefined {
    return tool.kind === "connect" ? undefined : tool.job;
}

/** Return the status line of a tool your space stays connected to, or none when replaced. */
function statusOf(tool: Tool): string | undefined {
    return tool.kind === "replace" ? undefined : tool.status;
}

/** One row of the wall and the bill: a job, your space's app for it, and the tools that do it. */
type Row = { label: string; app: App; tools: readonly Tool[]; isConnection: boolean };

/** The rows of the wall and the bill: your space's jobs, then the services it works with. */
const rows: readonly Row[] = [
    ...[...spaceJobs, "workflows" as const].map((job) => ({
        label: jobLabels[job],
        app: apps[job],
        tools: tools.filter((tool) => jobOf(tool) === job),
        isConnection: false,
    })),
    ...connections.map(([label, names, app]) => ({
        label,
        app,
        tools: tools.filter((tool) => names.includes(tool.name)),
        isConnection: true,
    })),
];

/** The milliseconds a mark takes to move to its new place. */
const moveTime = 560;

/** The milliseconds between one mark's move and the next. */
const moveStep = 18;

/** The strong ease-out every move lands on. */
const landing = "cubic-bezier(0.23, 1, 0.32, 1)";

/** Draw your tools a row per job: to pick while stacked, folded into your apps once destacked. */
export function ToolWall(properties: {
    picks: readonly string[];
    isOpen: boolean;
    onPick: (name: string) => void;
}) {
    // hold the wall and where each mark sat at the last layout
    let wall: HTMLOListElement | undefined;
    let places = new Map<string, DOMRect>();

    // find a row's picked tools
    const pickedOf = (row: Row) => row.tools.filter((tool) => properties.picks.includes(tool.name));

    // move each mark from where it sat to where it sits now, once the new layout has painted
    createEffect(
        () => ({ isOpen: properties.isOpen, picks: properties.picks }),
        () => {
            const frame = requestAnimationFrame(() => {
                if (!wall) {
                    return;
                }
                places = move(wall, places);
            });

            return () => cancelAnimationFrame(frame);
        },
    );

    return (
        <ol ref={wall} data-component="ToolWall" {...style.attrs(styles.wall)}>
            {/* column heads, as tall as the bill's beside it so the rows line up */}
            <li aria-hidden="true" {...style.attrs(styles.row, styles.head)}>
                <span {...style.attrs(styles.label)}>Job</span>
                <span {...style.attrs(styles.heading)}>
                    <span {...style.attrs(styles.label)}>Tools</span>
                    <span {...style.attrs(styles.label)}>
                        {properties.isOpen ? "In your space" : "Pick the ones you use"}
                    </span>
                </span>
            </li>
            {rows.map((row) => (
                <li {...style.attrs(styles.row)}>
                    <span {...style.attrs(styles.label)}>{row.label}</span>
                    <span {...style.attrs(styles.marks)}>
                        {properties.isOpen ? (
                            <>
                                <AppMark row={row} picked={pickedOf(row)} />
                                <Note picked={pickedOf(row)} />
                            </>
                        ) : (
                            row.tools.map((tool) => (
                                <ToolButton
                                    tool={tool}
                                    isPicked={properties.picks.includes(tool.name)}
                                    onPick={properties.onPick}
                                />
                            ))
                        )}
                    </span>
                </li>
            ))}
        </ol>
    );
}

/** Draw a tool to pick: its mark and its name, underlined once picked. */
function ToolButton(properties: { tool: Tool; isPicked: boolean; onPick: (name: string) => void }) {
    return (
        <button
            type="button"
            data-key={properties.tool.name}
            aria-pressed={properties.isPicked ? "true" : "false"}
            title={[properties.tool.name, statusOf(properties.tool)].filter(Boolean).join(", ")}
            onClick={() => properties.onPick(properties.tool.name)}
            {...style.attrs(styles.chip, styles.tool, properties.isPicked && styles.picked)}
        >
            <ToolMark tool={properties.tool} size={16} />
            <span {...style.attrs(styles.name)}>{properties.tool.name}</span>
        </button>
    );
}

/** Draw your space's app for a row, grown out of the first picked tool. */
function AppMark(properties: { row: Row; picked: readonly Tool[] }) {
    return (
        <span
            data-key={`app:${properties.row.label}`}
            data-from={properties.picked[0]?.name ?? `app:${properties.row.label}`}
            {...style.attrs(styles.chip, styles.app, properties.picked.length === 0 && styles.idle)}
        >
            <Tile icon={properties.row.app.icon} size="small" />
            <span {...style.attrs(styles.appName)}>{properties.row.app.name}</span>
        </span>
    );
}

/** Draw what a row's app does with the picked tools: what it replaces, then what it connects to. */
function Note(properties: { picked: readonly Tool[] }) {
    // split the picks, leaving out the side with none
    const parts = () => relationsOf(properties.picked).filter(([, related]) => related.length > 0);

    return (
        <span {...style.attrs(styles.note)}>
            {parts().length === 0
                ? "included"
                : parts().map(([verb, related], index) => (
                      <>
                          {index > 0 ? (
                              <span {...style.attrs(styles.separator)}>·</span>
                          ) : undefined}
                          {verb}
                          {related.map((tool) => (
                              <span {...style.attrs(styles.source)}>
                                  <ToolMark tool={tool} size={12} />
                                  {tool.name}
                              </span>
                          ))}
                      </>
                  ))}
        </span>
    );
}

/** Split picked tools into the ones your space replaces and the ones it stays connected to. */
function relationsOf(picked: readonly Tool[]) {
    return [
        ["replaces", picked.filter((tool) => tool.kind === "replace")],
        ["connects to", picked.filter((tool) => tool.kind !== "replace")],
    ] as const;
}

/** Write what a row's app does with the picked tools, as the bill notes it. */
function noteOf(picked: readonly Tool[]) {
    // join the sides that have tools, or note the app as included
    const parts = relationsOf(picked)
        .filter(([, related]) => related.length > 0)
        .map(([verb, related]) => `${verb} ${listOf(related)}`);

    return parts.length === 0 ? "included" : parts.join(" · ");
}

/** Move every mark of the wall from its last place to its new one, and return the new places. */
function move(wall: HTMLElement, last: ReadonlyMap<string, DOMRect>) {
    // measure the new places against the wall
    const origin = wall.getBoundingClientRect();
    const places = new Map<string, DOMRect>();
    const isStill = isMotionReduced(wall);
    const cells = [...wall.querySelectorAll<HTMLElement>("[data-key]")];

    // slide each cell in from where it, or the tool it grew out of, sat before
    cells.forEach((element, index) => {
        // note where the cell sits now, and find where it sat before
        const box = element.getBoundingClientRect();
        const now = new DOMRect(
            box.left - origin.left,
            box.top - origin.top,
            box.width,
            box.height,
        );
        places.set(element.dataset["key"] ?? "", now);
        const before =
            last.get(element.dataset["from"] ?? "") ?? last.get(element.dataset["key"] ?? "");
        if (isStill || last.size === 0) {
            return;
        }

        // fade in a cell that had no place before
        const delay = index * moveStep;
        if (before === undefined) {
            element.animate(
                [
                    { opacity: 0, transform: "translateY(6px) scale(0.97)" },
                    { opacity: 1, transform: "none" },
                ],
                {
                    duration: moveTime * 0.6,
                    delay: 60 + index * 6,
                    easing: landing,
                    fill: "backwards",
                },
            );

            return;
        }
        const scale = Math.min(
            1,
            Math.max(0.3, Math.min(before.width / now.width, before.height / now.height)),
        );
        const dx = before.left + before.width / 2 - (now.left + now.width / 2);
        const dy = before.top + before.height / 2 - (now.top + now.height / 2);
        if (Math.abs(dx) < 1 && Math.abs(dy) < 1 && scale > 0.99) {
            return;
        }
        element.animate(
            [{ transform: `translate(${dx}px, ${dy}px) scale(${scale})` }, { transform: "none" }],
            { duration: moveTime, delay, easing: landing, fill: "backwards" },
        );

        // bring a growing cell's face in once it has mostly grown, so no squeezed text shows
        if (scale < 0.9) {
            element.firstElementChild?.animate(
                [{ opacity: 0 }, { opacity: 0, offset: 0.4 }, { opacity: 1 }],
                { duration: moveTime, delay, easing: landing, fill: "backwards" },
            );
        }
    });

    return places;
}

/** Tally the picks a row to each job your space does. */
export function tallyOf(picks: readonly string[]): readonly Line[] {
    // gather each row's picked tools
    const picked = rows.map((row) => ({
        ...row,
        doers: row.tools.filter((tool) => picks.includes(tool.name)),
    }));

    // price each row on what the rows above it have not billed yet, and leave the services the space works with to their vendors
    const billed = new Set<string>();

    return picked.map((row) => {
        const price = rowPriceOf(row.doers, billed);
        for (const subscription of subscriptionsOf(row.doers)) {
            billed.add(subscription);
        }

        return {
            entry: entryOf(
                row.label,
                row.doers,
                row.app,
                row.isConnection,
                row.isConnection ? vendorBilled : price,
            ),
            workload: workloadOfRow(row.doers),
            tint:
                row.doers[0] === undefined
                    ? color.mutedForeground
                    : accentOf(row.doers[0].name, row.doers[0].tint),
        };
    });
}

/** Set one tally row. */
function entryOf(
    label: string,
    doers: readonly Tool[],
    app: App,
    isConnection: boolean,
    price: Price,
): Entry {
    // name the picked tools and list them
    const [first] = doers;

    return {
        stacked: {
            label,
            mark:
                first === undefined
                    ? [{ placeholder: app.icon }]
                    : doers.slice(0, 3).map((tool) => ({ icon: tool.icon, tint: tool.tint })),
            name:
                first === undefined
                    ? "None"
                    : doers.length === 1
                      ? first.name
                      : `${doers.length} apps`,
            note:
                first === undefined
                    ? "pick one to compare"
                    : doers.map((tool) => tool.name).join(", "),
            price,
        },
        destacked: {
            label,
            mark: { app: app.icon },
            name: app.name,
            note: noteOf(doers),
            price: isConnection ? price : { kind: "included" },
        },
    };
}

/** Sum the work a row's tools take: their storage and compute, a floor where any of them is. */
function workloadOfRow(doers: readonly Tool[]): Workload {
    return sumOf(doers.map((tool) => workloadOf(tool.name)));
}

/** Write the prompt that sets up a space, brings in replaced tools and connects the rest. */
export function promptOf(picks: readonly string[]) {
    // split the picks into what moves in and what stays connected
    const picked = tools.filter((tool) => picks.includes(tool.name));
    const moved = listOf(picked.filter((tool) => tool.kind === "replace"));
    const linked = listOf(picked.filter((tool) => tool.kind !== "replace"));
    const asks = [
        ...(moved === "" ? [] : [`bring in my ${moved}`]),
        ...(linked === "" ? [] : [`connect my ${linked}`]),
    ];

    return asks.length === 0
        ? agentPrompt
        : `Set up my Destack, ${asks.join(" and ")}, following ${setupGuide}`;
}

/** Name tools in a list a person would write. */
function listOf(picked: readonly Tool[]) {
    const names = picked.map((tool) => tool.name);

    return names.length <= 1
        ? names.join("")
        : `${names.slice(0, -1).join(", ")} and ${names.at(-1) ?? ""}`;
}

/** The wall styles. */
const styles = style.create({
    picture: {
        borderRadius: "22%",
        flexShrink: 0,
        objectFit: "cover",
    },
    wall: {
        display: "flex",
        flexDirection: "column",
        height: "100%",
        listStyle: "none",
        margin: 0,
        minHeight: { default: 0, [screen.belowDesktop]: "30rem", [media.maxMd]: 0 },
        padding: 0,
    },
    row: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        columnGap: "0.75rem",
        display: "grid",
        flexBasis: { default: 0, [media.maxMd]: "auto" },
        flexGrow: 1,
        gridTemplateColumns: {
            default: "6.5rem minmax(0, 1fr)",
            [media.maxMd]: "5rem minmax(0, 1fr)",
        },
        minHeight: "2.25rem",
        paddingBlock: { default: 0, [media.maxMd]: "0.375rem" },
    },
    label: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.12em",
        lineHeight: "0.875rem",
        textTransform: "uppercase",
    },
    head: {
        alignItems: "start",
        borderBottomWidth: 0,
        boxSizing: "border-box",
        flexBasis: "auto",
        flexGrow: 0,
        height: ledgerSizes.head,
        paddingBlock: "0.25rem",
    },
    heading: {
        display: "grid",
    },
    marks: {
        alignItems: "center",
        display: "flex",
        gap: "0.3125rem",
        minWidth: 0,
        overflow: "hidden",
    },
    chip: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        display: "inline-flex",
        fontFamily: "inherit",
        flexShrink: 0,
        gap: "0.375rem",
        height: "1.875rem",
        paddingInline: "0.4375rem",
    },
    tool: {
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
        cursor: "pointer",
        transition: `background-color 160ms ${landing}, box-shadow 160ms ${landing}`,
    },
    picked: {
        backgroundColor: {
            default: color.card,
            ":hover": { default: null, [media.hover]: color.card },
        },
        boxShadow: `inset 0 -2px 0 ${color.primary}`,
    },
    app: {
        backgroundColor: color.card,
        boxShadow: `inset 0 -2px 0 ${color.primary}`,
        paddingInlineStart: "0.25rem",
    },
    idle: {
        boxShadow: "none",
        color: color.mutedForeground,
    },
    appName: {
        fontSize: "0.8125rem",
        whiteSpace: "nowrap",
    },
    name: {
        display: { default: "inline", [media.maxMd]: "none" },
        fontSize: "0.8125rem",
        whiteSpace: "nowrap",
    },
    note: {
        alignItems: "center",
        color: color.mutedForeground,
        display: { default: "flex", [media.maxMd]: "none" },
        fontSize: "0.75rem",
        marginInlineStart: "0.25rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    source: {
        alignItems: "center",
        color: color.foreground,
        display: "inline-flex",
        gap: "0.25rem",
        marginInlineStart: "0.375rem",
    },
    separator: {
        marginInline: "0.5rem",
    },
});
