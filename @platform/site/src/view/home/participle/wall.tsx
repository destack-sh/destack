import { media } from "@destack/style/media.stylex";
import {
    billOf,
    bytesOf,
    bytesTotalOf,
    monthlyOf,
    priceOf,
    sizeOf,
    type Price,
} from "../figure/price";
import { plan } from "../../content/generated/plan";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createEffect, createMemo, For } from "@destack/view";

import { agentPrompt, setupGuide } from "../../content/site";
import { Favicon } from "../figure/glyph";
import type { Entry } from "../figure/ledger";
import { palette } from "../../palette.stylex";
import { screen } from "../../layout/screen.stylex";
import { isMotionReduced } from "@destack/view/motion";

/** An app every space comes with: its name, its icon, and its tint. */
type App = { name: string; icon: string; tint: string };

/** The apps a space comes with, by the job they do. */
const apps = {
    pages: { name: "Pages", icon: "pages", tint: "#3d6fb0" },
    tasks: { name: "Tasks", icon: "tasks", tint: "#c64a17" },
    calendar: { name: "Calendar", icon: "calendar", tint: "#c64a17" },
    files: { name: "Files", icon: "bucket", tint: "#2f7d8c" },
    tables: { name: "Tables", icon: "table", tint: "#4f8a5b" },
    canvas: { name: "Canvas", icon: "vector", tint: "#a0485f" },
    workflows: { name: "Workflows", icon: "sync", tint: "#4f8a5b" },
    chat: { name: "Chat", icon: "chat", tint: "#4f8a5b" },
    mail: { name: "Mail", icon: "mail", tint: "#3d6fb0" },
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
    canvas: "Design",
    workflows: "Automation",
};

/** A job a space's app does. */
type Job = keyof typeof apps;

/** The apps every space holds, one to each kind of thing it keeps, in the order the space sets them. */
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

/** The tally's rows for the services your space works with: the label, the services, and what your space does in their place. */
const connections: readonly (readonly [label: string, names: readonly string[], app: App])[] = [
    ["Agents", ["ChatGPT", "Claude"], { name: "Agents", icon: "agent", tint: "#6b5ca5" }],
    ["Code", ["GitHub"], { name: "Forge", icon: "source", tint: "#c64a17" }],
    ["Cloud", ["AWS"], { name: "Hosting", icon: "hosts", tint: "#2f7d8c" }],
];

/** A tool's face on the wall: its icon, its name, and its tint. */
type Face = { icon: string; name: string; tint: string };

/** A tool you may use today and what your space does with it: replaces it with an app, does either, or connects to it with a status line. */
type Tool =
    | (Face & { kind: "replace"; job: Job })
    | (Face & { kind: "both"; job: Job; status: string })
    | (Face & { kind: "connect"; status: string });

/** The tools your space replaces with its own apps. */
const replaced: readonly Tool[] = [
    { icon: "notion", name: "Notion", tint: "#000000", kind: "replace", job: "pages" },
    { icon: "googledocs", name: "Docs", tint: "#4285f4", kind: "replace", job: "pages" },
    { icon: "framer", name: "Framer", tint: "#0a0a0a", kind: "replace", job: "pages" },
    { icon: "webflow", name: "Webflow", tint: "#146ef5", kind: "replace", job: "pages" },
    { icon: "linear", name: "Linear", tint: "#5e6ad2", kind: "replace", job: "tasks" },
    { icon: "jira", name: "Jira", tint: "#0052cc", kind: "replace", job: "tasks" },
    { icon: "asana", name: "Asana", tint: "#f06a6a", kind: "replace", job: "tasks" },
    { icon: "trello", name: "Trello", tint: "#0052cc", kind: "replace", job: "tasks" },
    { icon: "calendly", name: "Calendly", tint: "#006bff", kind: "replace", job: "calendar" },
    { icon: "dropbox", name: "Dropbox", tint: "#0061ff", kind: "replace", job: "files" },
    { icon: "airtable", name: "Airtable", tint: "#18bfff", kind: "replace", job: "tables" },
    { icon: "googlesheets", name: "Sheets", tint: "#34a853", kind: "replace", job: "tables" },
    { icon: "googleforms", name: "Forms", tint: "#7248b9", kind: "replace", job: "tables" },
    { icon: "hubspot", name: "HubSpot", tint: "#ff7a59", kind: "replace", job: "tables" },
    { icon: "slack", name: "Slack", tint: "#4a154b", kind: "replace", job: "chat" },
    { icon: "superhuman", name: "Superhuman", tint: "#000000", kind: "replace", job: "mail" },
    { icon: "mailchimp", name: "Mailchimp", tint: "#241c15", kind: "replace", job: "mail" },
    { icon: "figma", name: "Figma", tint: "#f24e1e", kind: "replace", job: "canvas" },
    { icon: "miro", name: "Miro", tint: "#050038", kind: "replace", job: "canvas" },
    { icon: "loom", name: "Loom", tint: "#625df5", kind: "replace", job: "files" },
    { icon: "granola", name: "Granola", tint: "#000000", kind: "replace", job: "pages" },
    { icon: "n8n", name: "n8n", tint: "#ea4b71", kind: "replace", job: "workflows" },
    { icon: "zapier", name: "Zapier", tint: "#ff4f00", kind: "replace", job: "workflows" },
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
        icon: "googledrive",
        name: "Drive",
        tint: "#4285f4",
        kind: "both",
        job: "files",
        status: "mirrored",
    },
];

/** The services your space connects to: the agents that work in it, the cloud it runs on, and mirrors. */
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
        icon: "github",
        name: "GitHub",
        tint: "#181717",
        kind: "connect",
        status: "mirrored",
    },
    {
        icon: "aws",
        name: "AWS",
        tint: "#ff9900",
        kind: "connect",
        status: "where it runs",
    },
];

/** The order the wall sets its tools in, a row to each family: the startup stack, the work trackers, Google, the customer-facing sites, and the rest with the services your space connects to. */
const order: readonly string[] = [
    "Notion",
    "Linear",
    "Slack",
    "Figma",
    "GitHub",
    "Superhuman",
    "Granola",
    "Loom",
    "Jira",
    "Trello",
    "Asana",
    "Airtable",
    "Miro",
    "Zapier",
    "Gmail",
    "Calendar",
    "Drive",
    "Docs",
    "Sheets",
    "Calendly",
    "Framer",
    "Webflow",
    "HubSpot",
    "Mailchimp",
    "Dropbox",
    "n8n",
    "Forms",
    "ChatGPT",
    "Claude",
    "AWS",
];

/** Every tool on the wall, in the wall's order. */
const tools: readonly Tool[] = order.map((name) =>
    present(
        [...replaced, ...either, ...foundations].find((tool) => tool.name === name),
        `wall tool ${name}`,
    ),
);

/** The tools picked before the reader picks any: the six of the first figure, a chat, an assistant and a cloud. */
export const firstPicks: readonly string[] = [
    "Notion",
    "Framer",
    "Linktree",
    "Calendly",
    "Linear",
    "Dropbox",
    "Slack",
    "Claude",
    "AWS",
];

/** The subscription each tool bills under when it shares one: Google's tools come with one Workspace. */
const billedUnder: Readonly<Record<string, string>> = {
    Docs: "Google Workspace",
    Sheets: "Google Workspace",
    Forms: "Google Workspace",
    Calendar: "Google Workspace",
    Drive: "Google Workspace",
    Gmail: "Google Workspace",
};

/** The tools whose mark is a picture from the vendor, by name. */
const pictures: Record<string, string> = {
    Superhuman: "/logos/superhuman.png",
    Granola: "/logos/granola.png",
};

/** Return the subscriptions a set of tools bills under, each once. */
function subscriptionsOf(picked: readonly Tool[]): readonly string[] {
    return [...new Set(picked.map((tool) => billedUnder[tool.name] ?? tool.name))];
}

/** The price of a service the space works with, which its vendor bills either way and the comparison leaves out. */
const vendorBilled: Price = { kind: "fee", monthly: 0, basis: "billed by its vendor" };

/** Price one tally row: the subscriptions its tools add to those billed in the rows above it. */
function rowPriceOf(doers: readonly Tool[], billed: ReadonlySet<string>): Price {
    // bill nothing for a job nothing does yet
    const subscriptions = subscriptionsOf(doers);
    const [first] = subscriptions;
    if (first === undefined) {
        return { kind: "fee", monthly: 0, basis: "nothing picked" };
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
        monthly: monthlyOf(added.map(priceOf)),
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

/** Return the status line of a tool your space stays connected to, or none when it only replaces it. */
function statusOf(tool: Tool): string | undefined {
    return tool.kind === "replace" ? undefined : tool.status;
}

/** One cell of the wall: a tool on its own, an app your picked tools merged into, or a tool your space stays connected to. */
type Cell =
    | { kind: "tool"; key: string; tool: Tool; isPicked: boolean }
    | { kind: "app"; key: string; job: Job; sources: readonly Tool[] }
    | { kind: "link"; key: string; tool: Tool; isPicked: boolean };

/** The wall's height on tablets, the taller of its two states there: the stacked grid of five rows. */
const TABLET_WALL = "31.5rem";

/** The wall's height on phones, the taller of its two states there: the space with its connections, down to 360 pixel wide screens. */
const PHONE_WALL = "47rem";

/** The cells to a row of the wall. */
const perRow = 6;

/** Lay out the wall: every tool on its own while stacked, or once destacked every app your space holds with the picked tools it replaces, and every service it connects to. */
function cellsOf(picks: readonly string[], isOpen: boolean): readonly Cell[] {
    // set every tool on its own while stacked
    if (!isOpen) {
        return tools.map((tool) => ({
            kind: "tool",
            key: tool.name,
            tool,
            isPicked: picks.includes(tool.name),
        }));
    }

    // set every app your space holds with the picked tools it replaces, then every service it connects to
    const picked = tools.filter((tool) => picks.includes(tool.name));

    return [
        ...spaceJobs.map((job): Cell => ({
            kind: "app",
            key: `app:${job}`,
            job,
            sources: picked.filter((tool) => jobOf(tool) === job),
        })),
        ...foundations.map((tool): Cell => ({
            kind: "link",
            key: tool.name,
            tool,
            isPicked: picks.includes(tool.name),
        })),
    ];
}

/** Describe what a cell shows, so a cell that shows the same thing keeps its object across layouts. */
function signatureOf(cell: Cell) {
    // name the kind and key, then the tools an app took over or the pick of a tool or service
    return cell.kind === "app"
        ? `${cell.kind}:${cell.key}:${cell.sources.map((tool) => tool.name).join(",")}`
        : `${cell.kind}:${cell.key}:${String(cell.isPicked)}`;
}

/** Return the key of the cell a cell grows out of: an app out of its first tool, and every other cell out of itself. */
function originOf(cell: Cell) {
    // grow an app out of its first tool, and return a picked tool to the app it went into
    if (cell.kind === "app") {
        return cell.sources[0]?.name ?? cell.key;
    } else if (cell.kind !== "tool" || !cell.isPicked) {
        return cell.key;
    }
    const job = jobOf(cell.tool);

    return job === undefined ? cell.key : `app:${job}`;
}

/** The milliseconds a cell takes to move to its new place. */
const moveTime = 560;

/** The milliseconds between one cell's move and the next. */
const moveStep = 18;

/** The strong ease-out every move lands on. */
const landing = "cubic-bezier(0.23, 1, 0.32, 1)";

/**
 * Draw your stack as a wall of tools to pick from, and once destacked as your space.
 *
 * While stacked, every tool keeps its own tile and the picked ones are lit.
 * Once destacked, only your space is left: the picked tools slide together and merge into the apps that take over their jobs, beside the tools your space connects to.
 */
export function ToolWall(properties: {
    picks: readonly string[];
    isOpen: boolean;
    onPick: (name: string) => void;
}) {
    // hold the wall and where each cell sat at the last layout
    let wall: HTMLDivElement | undefined;
    let places = new Map<string, DOMRect>();

    // move each cell from where it sat to where it sits now, once the new layout has painted
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

    // lay out the cells, keeping each unchanged cell's object so only changed cells render again
    let previous = new Map<string, Cell>();
    const cells = createMemo(() => {
        const next = cellsOf(properties.picks, properties.isOpen).map((cell) => {
            const kept = previous.get(cell.key);

            return kept !== undefined && signatureOf(kept) === signatureOf(cell) ? kept : cell;
        });
        previous = new Map(next.map((cell) => [cell.key, cell]));

        return next;
    });
    const list = (each: () => readonly Cell[]) => (
        <For each={each()} keyed={(cell) => cell.key}>
            {(cell) => (
                <CellView cell={cell()} isOpen={properties.isOpen} onPick={properties.onPick} />
            )}
        </For>
    );

    return (
        <div ref={wall} data-component="ToolWall" {...style.attrs(styles.wall, styles.wallFull)}>
            {properties.isOpen ? (
                <>
                    <section data-component="Space" {...style.attrs(styles.box)}>
                        <span {...style.attrs(styles.boxBar)}>
                            <Favicon icon="user" tint={color.mutedForeground} size={12} />
                            destack.app/@florian
                        </span>
                        <ul {...style.attrs(styles.grid, styles.space)}>
                            {list(() => cells().filter((cell) => cell.kind === "app"))}
                        </ul>
                        <Included />
                        <div data-component="Connected" {...style.attrs(styles.band)}>
                            <span {...style.attrs(styles.groupName)}>Connected</span>
                            <ul {...style.attrs(styles.grid, styles.space, styles.links)}>
                                {list(() => cells().filter((cell) => cell.kind === "link"))}
                            </ul>
                        </div>
                    </section>
                </>
            ) : (
                <ul {...style.attrs(styles.grid)}>{list(cells)}</ul>
            )}
        </div>
    );
}

/** The services every space comes with, which none of the tools above bring along: the icon, the name, and what it does. */
const included: readonly (readonly [icon: string, name: string, line: string])[] = [
    ["user", "Accounts", "one sign-in for you and your agent"],
    ["auth", "Access", "who sees what, one rule set"],
    ["vault", "Vault", "keys that never leave the server"],
    ["sync", "Workflows", "jobs that survive restarts"],
    ["agent", "Agents", "act as you, within your rules"],
    ["search", "Search", "one index over every app"],
    ["hosts", "Hosting", "your laptop, server or cloud"],
    ["source", "Forge", "every app's source, forkable"],
];

/** Draw what every space includes besides its apps, as a band along the bottom of the space. */
function Included() {
    return (
        <div data-component="Included" {...style.attrs(styles.band)}>
            <span {...style.attrs(styles.groupName)}>Included</span>
            <ul {...style.attrs(styles.grid, styles.space, styles.links)}>
                {included.map(([icon, name, line], index) => (
                    <li
                        title={line}
                        {...style.attrs(
                            styles.cell,
                            styles.spaceCell,
                            styles.rising,
                            styles.includedDelay(index),
                        )}
                    >
                        <span {...style.attrs(styles.face, styles.link)}>
                            <Favicon icon={icon} tint={color.foreground} size={18} />
                            <span {...style.attrs(styles.toolName)}>{name}</span>
                        </span>
                    </li>
                ))}
            </ul>
        </div>
    );
}

/** Move every cell of the grid from its last place to its new one, and return the new places. */
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

/** Draw one cell of the wall. */
function CellView(properties: { cell: Cell; isOpen: boolean; onPick: (name: string) => void }) {
    const cell = () => properties.cell;

    return (
        <li
            data-key={cell().key}
            data-from={originOf(cell())}
            {...style.attrs(styles.cell, properties.isOpen && styles.spaceCell)}
        >
            {contentOf(cell(), properties.onPick)}
        </li>
    );
}

/** Draw what a cell holds: a tool to pick, an app with the tools it took over, or a connection with its status. */
function contentOf(cell: Cell, onPick: (name: string) => void) {
    if (cell.kind === "app") {
        const app = apps[cell.job];

        return (
            <span
                data-component="SpaceApp"
                {...style.attrs(
                    styles.face,
                    styles.tool,
                    styles.app,
                    cell.sources.length > 0 && styles.picked,
                )}
            >
                <span style={{ "background-color": app.tint }} {...style.attrs(styles.appIcon)}>
                    <Favicon icon={app.icon} tint="#ffffff" size={18} />
                </span>
                <span {...style.attrs(styles.toolName, styles.appName)}>{app.name}</span>
                <span {...style.attrs(styles.sources)}>
                    {cell.sources.map((tool) => (
                        <ToolMark tool={tool} size={12} />
                    ))}
                </span>
            </span>
        );
    } else if (cell.kind === "link") {
        return (
            <button
                type="button"
                aria-pressed={cell.isPicked ? "true" : "false"}
                title={statusOf(cell.tool)}
                onClick={() => onPick(cell.tool.name)}
                {...style.attrs(
                    styles.face,
                    styles.link,
                    styles.pressable,
                    !cell.isPicked && styles.linkIdle,
                )}
            >
                <ToolMark tool={cell.tool} size={18} />
                <span {...style.attrs(styles.toolName)}>{cell.tool.name}</span>
                {cell.isPicked ? <span {...style.attrs(styles.dot)} /> : undefined}
            </button>
        );
    }

    return (
        <button
            type="button"
            aria-pressed={cell.isPicked ? "true" : "false"}
            onClick={() => onPick(cell.tool.name)}
            {...style.attrs(styles.face, styles.tool, cell.isPicked && styles.picked)}
        >
            <ToolMark tool={cell.tool} size={28} />
            <span {...style.attrs(styles.toolName, styles.toolNameWrapped)}>{cell.tool.name}</span>
        </button>
    );
}

/** Tally the picks a row to each job your space does: the picked tools that do it while stacked, and the app or service that takes it over once destacked. */
export function tallyOf(picks: readonly string[]): readonly Entry[] {
    // gather the picked tools, and the rows of jobs then of connections
    const picked = tools.filter((tool) => picks.includes(tool.name));
    const rows = [
        ...[...spaceJobs, "workflows" as const].map((job) => ({
            label: jobLabels[job],
            doers: picked.filter((tool) => jobOf(tool) === job),
            app: apps[job],
            isConnection: false,
        })),
        ...connections.map(([label, names, app]) => ({
            label,
            doers: picked.filter((tool) => names.includes(tool.name)),
            app,
            isConnection: true,
        })),
    ];

    // price each row on what the rows above it have not billed yet, and leave the services the space works with to their vendors
    const billed = new Set<string>();

    return rows.map((row) => {
        const price = rowPriceOf(row.doers, billed);
        for (const subscription of subscriptionsOf(row.doers)) {
            billed.add(subscription);
        }

        return entryOf(
            row.label,
            row.doers,
            row.app,
            row.isConnection,
            row.isConnection ? vendorBilled : price,
        );
    });
}

/** Set one tally row: the tools you picked for a job while stacked, and what your space does in their place once destacked, which still pays for the services it works with. */
function entryOf(
    label: string,
    doers: readonly Tool[],
    app: App,
    isConnection: boolean,
    price: Price,
): Entry {
    // name the picked tools and list them, and sum the data they hold
    const [first] = doers;
    const verb = isConnection ? "works with" : "replaces";
    const data = doers.reduce((sum, tool) => sum + sizeOf(tool.name), 0);

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
            detail: data === 0 ? "—" : bytesOf(data),
        },
        destacked: {
            label,
            mark: { icon: app.icon, tint: app.tint },
            name: app.name,
            note: first === undefined ? "included" : `${verb} ${listOf(doers)}`,
            price: isConnection ? price : { kind: "included" },
            detail: data === 0 ? "—" : bytesOf(data),
        },
    };
}

/** Total the bill: the picked tools and their invoices while stacked, or one space with the services it works with once destacked, and what either costs a month. */
export function totalOf(
    picks: readonly string[],
    isOpen: boolean,
): readonly [string, string, string] {
    // read the picked tools and the data they hold
    const picked = tools.filter((tool) => picks.includes(tool.name));
    const data = picked.map((tool) => sizeOf(tool.name));

    // bill only what the space takes over, since the services it works with bill on their own either way
    const isKept = (tool: Tool) => connections.some(([, names]) => names.includes(tool.name));
    const takenOver = picked.filter((tool) => !isKept(tool));
    const billed = subscriptionsOf(takenOver);

    return isOpen
        ? [`${spaceJobs.length} apps, 1 invoice`, billOf([], plan.monthly), bytesTotalOf(data)]
        : [
              `${takenOver.length} tools, ${billed.length} invoices`,
              billOf(billed.map(priceOf), 0),
              bytesTotalOf(data),
          ];
}

/** Write the prompt that sets up a space, brings in the picked tools it replaces, and connects the rest. */
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

/** An included service rising into place once the space has formed. */
const rise = style.keyframes({
    from: { opacity: 0, transform: "translateY(6px)" },
});

/** The wall styles. */
const styles = style.create({
    picture: {
        borderRadius: "22%",
        flexShrink: 0,
        objectFit: "cover",
    },
    wall: {
        alignContent: { default: "stretch", [screen.belowDesktop]: "start" },
        display: "grid",
        gap: "1.25rem",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        height: "100%",
        minHeight: { default: 0, [screen.belowDesktop]: TABLET_WALL, [media.maxMd]: PHONE_WALL },
    },
    wallFull: {
        gridTemplateRows: { default: "minmax(0, 1fr)", [screen.belowDesktop]: "auto" },
    },
    box: {
        borderColor: color.border,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        gap: "0.875rem",
        padding: "0.875rem",
    },
    boxBar: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        fontFamily: font.code,
        fontSize: "0.6875rem",
        gap: "0.375rem",
    },
    band: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        display: "grid",
        gap: "0.625rem",
        paddingTop: "0.75rem",
    },
    rising: {
        animationDuration: "420ms",
        animationFillMode: "both",
        animationName: { default: rise, [media.motionReduce]: "none" },
        animationTimingFunction: landing,
    },
    group: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
        minWidth: 0,
    },
    groupName: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.12em",
        textTransform: "uppercase",
    },
    grid: {
        backgroundColor: color.border,
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        gap: stroke.border,
        gridAutoFlow: "row dense",
        gridAutoRows: {
            default: "minmax(0, 1fr)",
            [screen.belowDesktop]: "5.75rem",
            [media.maxMd]: "3.5rem",
        },
        gridTemplateColumns: {
            default: `repeat(${perRow}, minmax(0, 1fr))`,
            [media.maxMd]: "repeat(3, minmax(0, 1fr))",
        },
        listStyle: "none",
        margin: 0,
        minHeight: 0,
        padding: 0,
    },
    cell: {
        backgroundColor: color.background,
        display: "grid",
        minHeight: 0,
        minWidth: 0,
    },
    space: {
        backgroundColor: "transparent",
        borderWidth: 0,
        gap: "0.75rem",
        gridAutoFlow: "row",
        gridAutoRows: { default: "minmax(5.75rem, 1fr)", [media.maxMd]: "3.75rem" },
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [media.maxMd]: "repeat(2, minmax(0, 1fr))",
        },
    },
    linkIdle: {
        backgroundColor: "transparent",
        color: color.mutedForeground,
        opacity: { default: 0.6, ":hover": { default: null, [media.hover]: 1 } },
    },
    links: {
        gridAutoRows: "2.75rem",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [media.maxMd]: "repeat(2, minmax(0, 1fr))",
        },
    },
    spaceCell: {
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        overflow: "hidden",
    },
    includedDelay: (index: number) => ({
        animationDelay: `${220 + index * 40}ms`,
    }),
    face: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        display: "flex",
        fontFamily: "inherit",
        gap: "0.625rem",
        height: "100%",
        minWidth: 0,
        paddingInline: "0.875rem",
        textAlign: "left",
        width: "100%",
    },
    tool: {
        alignContent: "center",
        cursor: "pointer",
        display: "grid",
        gap: { default: "0.4375rem", [media.maxMd]: "0.25rem" },
        justifyItems: "center",
        paddingInline: { default: "0.375rem", [media.maxMd]: "0.25rem" },
        transition: `background-color 160ms ${landing}, opacity 320ms ${landing}`,
        backgroundColor: { default: null, ":hover": { default: null, [media.hover]: color.muted } },
    },
    picked: {
        backgroundColor: {
            default: color.card,
            ":hover": { default: null, [media.hover]: color.card },
        },
        boxShadow: `inset 0 -2px 0 ${color.primary}`,
    },
    toolName: {
        fontSize: { default: "0.8125rem", [media.maxMd]: "0.75rem" },
        maxWidth: "100%",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    toolNameWrapped: {
        lineHeight: 1.2,
        textAlign: "center",
        whiteSpace: { default: "normal", [media.maxMd]: "nowrap" },
    },
    app: {
        backgroundColor: color.card,
        position: "relative",
    },
    appIcon: {
        alignItems: "center",
        borderRadius: "7px",
        display: "flex",
        flexShrink: 0,
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    link: {
        backgroundColor: color.card,
        position: "relative",
    },
    pressable: {
        cursor: "pointer",
    },
    appName: {
        fontWeight: 600,
    },
    sources: {
        alignItems: "center",
        bottom: "0.5rem",
        display: { default: "flex", [media.maxMd]: "none" },
        gap: "0.25rem",
        position: "absolute",
        right: "0.5rem",
    },
    dot: {
        backgroundColor: palette.green,
        borderRadius: "50%",
        flexShrink: 0,
        height: "0.4375rem",
        width: "0.4375rem",
    },
});
