import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as stylex from "@destack/style";
import { createEffect } from "@destack/view";

import { agentPrompt } from "../content/site";
import { tokens } from "../style/tokens.stylex";
import { Favicon, Glyph } from "./glyph";

/** An app every space comes with: its name, its icon, and its tint. */
type App = { name: string; icon: string; tint: string };

/** The apps a space comes with, by the job they do. */
const apps = {
    home: { name: "Home", icon: "user", tint: "#2f7d8c" },
    pages: { name: "Pages", icon: "pages", tint: "#3d6fb0" },
    tasks: { name: "Tasks", icon: "tasks", tint: "#c64a17" },
    calendar: { name: "Calendar", icon: "calendar", tint: "#c64a17" },
    files: { name: "Files", icon: "bucket", tint: "#2f7d8c" },
    tables: { name: "Tables", icon: "table", tint: "#4f8a5b" },
    forms: { name: "Forms", icon: "template", tint: "#b8862b" },
    canvas: { name: "Canvas", icon: "vector", tint: "#a0485f" },
    workflows: { name: "Workflows", icon: "sync", tint: "#4f8a5b" },
    vault: { name: "Vault", icon: "vault", tint: "#12313c" },
    chat: { name: "Chat", icon: "chat", tint: "#4f8a5b" },
    mail: { name: "Mail", icon: "mail", tint: "#3d6fb0" },
    forge: { name: "Forge", icon: "source", tint: "#c64a17" },
} satisfies Record<string, App>;

/** A job a space's app does. */
type Job = keyof typeof apps;

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
    { icon: "googledocs", name: "Google Docs", tint: "#4285f4", kind: "replace", job: "pages" },
    { icon: "confluence", name: "Confluence", tint: "#172b4d", kind: "replace", job: "pages" },
    { icon: "coda", name: "Coda", tint: "#f46a54", kind: "replace", job: "pages" },
    { icon: "gamma", name: "Gamma", tint: "#000000", kind: "replace", job: "pages" },
    {
        icon: "googleslides",
        name: "Google Slides",
        tint: "#fbbc04",
        kind: "replace",
        job: "pages",
    },
    { icon: "framer", name: "Framer", tint: "#0a0a0a", kind: "replace", job: "pages" },
    { icon: "webflow", name: "Webflow", tint: "#146ef5", kind: "replace", job: "pages" },
    { icon: "squarespace", name: "Squarespace", tint: "#000000", kind: "replace", job: "pages" },
    { icon: "linear", name: "Linear", tint: "#5e6ad2", kind: "replace", job: "tasks" },
    { icon: "jira", name: "Jira", tint: "#0052cc", kind: "replace", job: "tasks" },
    { icon: "asana", name: "Asana", tint: "#f06a6a", kind: "replace", job: "tasks" },
    { icon: "trello", name: "Trello", tint: "#0052cc", kind: "replace", job: "tasks" },
    { icon: "monday", name: "Monday", tint: "#6161ff", kind: "replace", job: "tasks" },
    { icon: "calendly", name: "Calendly", tint: "#006bff", kind: "replace", job: "calendar" },
    { icon: "dropbox", name: "Dropbox", tint: "#0061ff", kind: "replace", job: "files" },
    { icon: "box", name: "Box", tint: "#0061d5", kind: "replace", job: "files" },
    { icon: "airtable", name: "Airtable", tint: "#18bfff", kind: "replace", job: "tables" },
    {
        icon: "googlesheets",
        name: "Google Sheets",
        tint: "#34a853",
        kind: "replace",
        job: "tables",
    },
    { icon: "hubspot", name: "HubSpot", tint: "#ff7a59", kind: "replace", job: "tables" },
    { icon: "googleforms", name: "Google Forms", tint: "#7248b9", kind: "replace", job: "forms" },
    { icon: "slack", name: "Slack", tint: "#4a154b", kind: "replace", job: "chat" },
    { icon: "superhuman", name: "Superhuman", tint: "#000000", kind: "replace", job: "mail" },
    { icon: "intercom", name: "Intercom", tint: "#1f8ded", kind: "replace", job: "chat" },
    { icon: "zendesk", name: "Zendesk", tint: "#03363d", kind: "replace", job: "mail" },
    { icon: "mailchimp", name: "Mailchimp", tint: "#241c15", kind: "replace", job: "mail" },
    { icon: "figma", name: "Figma", tint: "#f24e1e", kind: "replace", job: "canvas" },
    { icon: "miro", name: "Miro", tint: "#050038", kind: "replace", job: "canvas" },
    { icon: "loom", name: "Loom", tint: "#625df5", kind: "replace", job: "files" },
    { icon: "granola", name: "Granola", tint: "#000000", kind: "replace", job: "pages" },
    { icon: "n8n", name: "n8n", tint: "#ea4b71", kind: "replace", job: "workflows" },
    { icon: "zapier", name: "Zapier", tint: "#ff4f00", kind: "replace", job: "workflows" },
    { icon: "1password", name: "1Password", tint: "#3b66bc", kind: "replace", job: "vault" },
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
        name: "Google Calendar",
        tint: "#4285f4",
        kind: "both",
        job: "calendar",
        status: "two-way sync",
    },
    {
        icon: "googledrive",
        name: "Google Drive",
        tint: "#4285f4",
        kind: "both",
        job: "files",
        status: "mirrored",
    },
];

/** The services your space runs alongside and connects to. */
const foundations: readonly Tool[] = [
    { icon: "openai", name: "ChatGPT", tint: "#000000", kind: "connect", status: "signed in" },
    { icon: "claude", name: "Claude", tint: "#d97757", kind: "connect", status: "signed in" },
    {
        icon: "github",
        name: "GitHub",
        tint: "#181717",
        kind: "connect",
        status: "florian/site mirrored",
    },
    { icon: "aws", name: "AWS", tint: "#ff9900", kind: "connect", status: "eu-central-1" },
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
    "Confluence",
    "Trello",
    "Asana",
    "Monday",
    "Airtable",
    "Miro",
    "Zapier",
    "Gmail",
    "Google Calendar",
    "Google Drive",
    "Google Docs",
    "Google Sheets",
    "Google Slides",
    "Google Forms",
    "Calendly",
    "Framer",
    "Webflow",
    "Squarespace",
    "HubSpot",
    "Mailchimp",
    "Intercom",
    "Zendesk",
    "Gamma",
    "Dropbox",
    "Box",
    "Coda",
    "n8n",
    "1Password",
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

/** The accounts that sign in to more than one tool, by the tools they cover. */
const accounts: Record<string, string> = {
    "Google Docs": "Google",
    "Google Sheets": "Google",
    "Google Forms": "Google",
    "Google Slides": "Google",
    "Google Calendar": "Google",
    "Google Drive": "Google",
    Gmail: "Google",
    Jira: "Atlassian",
    Confluence: "Atlassian",
    Trello: "Atlassian",
};

/** The tools whose mark is a picture from the vendor, by name. */
const pictures: Record<string, string> = {
    Superhuman: "/logos/superhuman.png",
    Granola: "/logos/granola.png",
    Gamma: "/logos/gamma.svg",
    Monday: "/logos/monday.png",
};

/** Count the logins a set of tools needs, one per account they sign in with. */
function loginsOf(picked: readonly Tool[]) {
    return new Set(picked.map((tool) => accounts[tool.name] ?? tool.name)).size;
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
            {...stylex.attrs(styles.picture)}
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
    | { kind: "link"; key: string; tool: Tool }
    | { kind: "add"; key: string };

/** The media query for phone screens, where the wall sets fewer cells to a row. */
const mobile = "@media (max-width: 767px)";

/** The cells to a row of the wall. */
const perRow = 8;

/** Lay out the wall: every tool on its own while stacked, or once destacked only your space, the picked tools merged into their apps and the ones it connects to. */
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

    // merge the picked tools into their apps, then the connections
    const picked = tools.filter((tool) => picks.includes(tool.name));
    const jobs = [
        ...new Set(
            picked.flatMap((tool) => {
                const job = jobOf(tool);

                return job === undefined ? [] : [job];
            }),
        ),
    ];

    // set the apps your tools merged into, then the services your space connects to
    const space: readonly Cell[] = [
        ...jobs.map((job): Cell => ({
            kind: "app",
            key: `app:${job}`,
            job,
            sources: picked.filter((tool) => jobOf(tool) === job),
        })),
        ...picked
            .filter((tool) => tool.kind === "connect")
            .map((tool): Cell => ({ kind: "link", key: tool.name, tool })),
    ];

    // close the last row with cells to add another tool
    const open = perRow - (space.length % perRow);
    const adds = Array.from({ length: open }, (_, index): Cell => ({
        kind: "add",
        key: `add:${index}`,
    }));

    return [...space, ...adds];
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
    // hold the grid and where each cell sat at the last layout
    let grid: HTMLUListElement | undefined;
    let places = new Map<string, DOMRect>();

    // move each cell from where it sat to where it sits now, once the new layout has painted
    createEffect(
        () => ({ isOpen: properties.isOpen, picks: properties.picks }),
        () => {
            const frame = requestAnimationFrame(() => {
                if (!grid) {
                    return;
                }
                places = move(grid, places);
            });

            return () => cancelAnimationFrame(frame);
        },
    );

    return (
        <div
            data-component="ToolWall"
            {...stylex.attrs(styles.wall, !properties.isOpen && styles.wallFull)}
        >
            <ul ref={grid} {...stylex.attrs(styles.grid, properties.isOpen && styles.space)}>
                {cellsOf(properties.picks, properties.isOpen).map((cell) => (
                    <CellView cell={cell} isOpen={properties.isOpen} onPick={properties.onPick} />
                ))}
            </ul>
            {properties.isOpen ? (
                <>
                    <Outside picks={properties.picks} onPick={properties.onPick} />
                    <Included />
                </>
            ) : undefined}
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

/** Draw what every space includes besides its apps, in the room the replaced tools left. */
function Included() {
    return (
        <section data-component="Included" {...stylex.attrs(styles.included)}>
            <span {...stylex.attrs(styles.includedName)}>Included in every space</span>
            <ul {...stylex.attrs(styles.includedGrid)}>
                {included.map(([icon, name, line], index) => (
                    <li style={{ "--index": String(index) }} {...stylex.attrs(styles.includedItem)}>
                        <Favicon icon={icon} tint={color.foreground} size={16} />
                        <span {...stylex.attrs(styles.includedWords)}>
                            <b>{name}</b>
                            <span {...stylex.attrs(styles.includedLine)}>{line}</span>
                        </span>
                    </li>
                ))}
            </ul>
        </section>
    );
}

/** Draw the tools outside your space as two strips of small logos: the ones you destacked struck through, and the rest still there to add. */
function Outside(properties: { picks: readonly string[]; onPick: (name: string) => void }) {
    // split the tools into the ones you destacked and the ones you could add
    const isDestacked = (tool: Tool) =>
        properties.picks.includes(tool.name) && tool.kind !== "connect";
    const destacked = () => tools.filter((tool) => isDestacked(tool));
    const rest = () => tools.filter((tool) => !properties.picks.includes(tool.name));

    return (
        <div data-component="Outside" {...stylex.attrs(styles.behind)}>
            <span {...stylex.attrs(styles.line)}>
                <span {...stylex.attrs(styles.behindName)}>Destacked</span>
                <Chips tools={destacked()} isStruck={true} onPick={properties.onPick} />
            </span>
            <span {...stylex.attrs(styles.line)}>
                <span {...stylex.attrs(styles.behindName)}>Add more</span>
                <span {...stylex.attrs(styles.more)}>
                    <Chips tools={rest()} isStruck={false} onPick={properties.onPick} />
                </span>
            </span>
        </div>
    );
}

/** Draw tools as small logo keys that pick or drop each one. */
function Chips(properties: {
    tools: readonly Tool[];
    isStruck: boolean;
    onPick: (name: string) => void;
}) {
    return (
        <ul {...stylex.attrs(styles.strip)}>
            {properties.tools.map((tool) => (
                <li>
                    <button
                        type="button"
                        title={tool.name}
                        aria-label={tool.name}
                        aria-pressed={properties.isStruck ? "true" : "false"}
                        onClick={() => properties.onPick(tool.name)}
                        {...stylex.attrs(styles.chip, properties.isStruck && styles.chipLeft)}
                    >
                        <ToolMark tool={tool} size={18} />
                    </button>
                </li>
            ))}
        </ul>
    );
}

/** Move every cell of the grid from its last place to its new one, and return the new places. */
function move(grid: HTMLUListElement, last: ReadonlyMap<string, DOMRect>) {
    // measure the new places against the grid
    const origin = grid.getBoundingClientRect();
    const places = new Map<string, DOMRect>();
    const isStill = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const cells = [...grid.querySelectorAll<HTMLElement>("[data-key]")];

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
        <li data-key={cell().key} data-from={originOf(cell())} {...stylex.attrs(styles.cell)}>
            {contentOf(cell(), properties.onPick)}
        </li>
    );
}

/** Draw what a cell holds: a tool to pick, an app with the tools it took over, or a connection with its status. */
function contentOf(cell: Cell, onPick: (name: string) => void) {
    if (cell.kind === "add") {
        return (
            <span data-component="AddTool" {...stylex.attrs(styles.face, styles.tool, styles.add)}>
                <Glyph name="plus" size={28} weight={1.5} />
                <span {...stylex.attrs(styles.toolName)}>Add a tool</span>
            </span>
        );
    } else if (cell.kind === "app") {
        const app = apps[cell.job];

        return (
            <span data-component="SpaceApp" {...stylex.attrs(styles.face, styles.tool, styles.app)}>
                <span style={{ "background-color": app.tint }} {...stylex.attrs(styles.appIcon)}>
                    <Favicon icon={app.icon} tint="#ffffff" size={18} />
                </span>
                <span {...stylex.attrs(styles.toolName, styles.appName)}>{app.name}</span>
                <span {...stylex.attrs(styles.sources)}>
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
                aria-pressed="true"
                title={`Connected · ${statusOf(cell.tool) ?? ""}`}
                onClick={() => onPick(cell.tool.name)}
                {...stylex.attrs(styles.face, styles.tool, styles.link)}
            >
                <ToolMark tool={cell.tool} size={28} />
                <span {...stylex.attrs(styles.toolName)}>{cell.tool.name}</span>
                <span {...stylex.attrs(styles.sources)}>
                    <span {...stylex.attrs(styles.dot)} />
                </span>
            </button>
        );
    }

    return (
        <button
            type="button"
            aria-pressed={cell.isPicked ? "true" : "false"}
            onClick={() => onPick(cell.tool.name)}
            {...stylex.attrs(styles.face, styles.tool, cell.isPicked && styles.picked)}
        >
            <ToolMark tool={cell.tool} size={28} />
            <span {...stylex.attrs(styles.toolName)}>{cell.tool.name}</span>
        </button>
    );
}

/** Count what the picks add up to: tools while stacked, or apps and connections once destacked. */
export function countOf(picks: readonly string[], isOpen: boolean) {
    // gather the picked tools and the apps they merge into
    const picked = tools.filter((tool) => picks.includes(tool.name));
    const jobs = new Set(picked.flatMap((tool) => (tool.kind === "connect" ? [] : [tool.job])));
    const links = picked.filter((tool) => tool.kind === "connect").length;

    return isOpen
        ? `${jobs.size} apps in one space, ${links} connected`
        : `${picked.length} tools, ${loginsOf(picked)} logins`;
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
        : `Set up my Destack, ${asks.join(" and ")}, following https://destack.sh/docs/setup.md`;
}

/** Name tools in a list a person would write. */
function listOf(picked: readonly Tool[]) {
    const names = picked.map((tool) => tool.name);

    return names.length <= 1
        ? names.join("")
        : `${names.slice(0, -1).join(", ")} and ${names.at(-1) ?? ""}`;
}

/** An included service rising into place once the space has formed. */
const rise = stylex.keyframes({
    from: { opacity: 0, transform: "translateY(6px)" },
});

/** The wall styles. */
const styles = stylex.create({
    picture: {
        borderRadius: "22%",
        flexShrink: 0,
        objectFit: "cover",
    },
    wall: {
        display: "grid",
        gap: "1.25rem",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        height: "100%",
        minHeight: 0,
    },
    wallFull: {
        gridTemplateRows: "minmax(0, 1fr)",
    },
    behind: {
        alignSelf: "start",
        display: "grid",
        gap: "0.625rem",
        minWidth: 0,
    },
    line: {
        alignItems: "center",
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: "6.5rem minmax(0, 1fr)",
        minWidth: 0,
    },
    behindName: {
        color: color.mutedForeground,
        flexShrink: 0,
        fontFamily: tokens.monoFont,
        fontSize: "0.625rem",
        letterSpacing: "0.12em",
        textTransform: "uppercase",
    },
    more: {
        display: "flex",
        flexGrow: 1,
        maskImage: "linear-gradient(90deg, #000 calc(100% - 3rem), transparent)",
        minWidth: 0,
        overflow: "hidden",
    },
    add: {
        color: color.mutedForeground,
        cursor: "default",
    },
    strip: {
        display: "flex",
        flexWrap: "nowrap",
        flexShrink: 1,
        minWidth: 0,
        overflow: "hidden",
        gap: "0.125rem",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    chip: {
        alignItems: "center",
        backgroundColor: { default: "transparent", ":hover": color.muted },
        borderRadius: "6px",
        borderWidth: 0,
        cursor: "pointer",
        display: "flex",
        height: "1.625rem",
        justifyContent: "center",
        opacity: { default: 0.55, ":hover": 1 },
        position: "relative",
        transition: `opacity 160ms ${landing}, background-color 160ms ${landing}`,
        width: "1.625rem",
    },
    chipLeft: {
        filter: "grayscale(1)",
        opacity: { default: 0.35, ":hover": 1 },
        "::after": {
            backgroundColor: color.foreground,
            content: "''",
            height: "1.5px",
            left: "15%",
            position: "absolute",
            rotate: "-35deg",
            top: "50%",
            width: "70%",
        },
    },
    grid: {
        backgroundColor: color.background,
        borderColor: tokens.rule,
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
        gap: tokens.hairline,
        gridAutoFlow: "row dense",
        gridAutoRows: "minmax(0, 1fr)",
        gridTemplateColumns: `repeat(${perRow}, minmax(0, 1fr))`,
        listStyle: "none",
        margin: 0,
        minHeight: 0,
        padding: 0,
        [mobile]: { gridAutoRows: "5rem", gridTemplateColumns: "repeat(4, minmax(0, 1fr))" },
    },
    cell: {
        backgroundColor: color.background,
        display: "grid",
        minHeight: 0,
        minWidth: 0,
        boxShadow: `0 0 0 ${tokens.hairline} ${tokens.rule}`,
    },
    space: {
        gridAutoFlow: "row",
        gridAutoRows: "6.9375rem",
    },
    included: {
        alignSelf: "end",
        borderTopColor: tokens.rule,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "grid",
        gap: "1rem",
        paddingTop: "1.125rem",
    },
    includedName: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.625rem",
        letterSpacing: "0.12em",
        textTransform: "uppercase",
    },
    includedGrid: {
        display: "grid",
        gap: "1.125rem 2rem",
        gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
        [mobile]: { gridTemplateColumns: "repeat(2, minmax(0, 1fr))" },
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    includedItem: {
        alignItems: "flex-start",
        animationDelay: "calc(220ms + var(--index) * 40ms)",
        animationDuration: "420ms",
        animationFillMode: "both",
        animationName: rise,
        animationTimingFunction: landing,
        display: "flex",
        gap: "0.625rem",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    includedWords: {
        display: "grid",
        fontSize: "0.8125rem",
        gap: "0.125rem",
    },
    includedLine: {
        color: color.mutedForeground,
        fontSize: "0.75rem",
    },
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
        gap: "0.4375rem",
        justifyItems: "center",
        paddingInline: "0.375rem",
        transition: `background-color 160ms ${landing}, opacity 320ms ${landing}`,
        ":hover": { backgroundColor: color.muted },
    },
    picked: {
        backgroundColor: { default: color.card, ":hover": color.card },
        boxShadow: `inset 0 -2px 0 ${tokens.signal}`,
    },
    toolName: {
        fontSize: "0.8125rem",
        maxWidth: "100%",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
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
        cursor: "pointer",
        position: "relative",
    },
    appName: {
        fontWeight: 600,
    },
    sources: {
        alignItems: "center",
        bottom: "0.5rem",
        display: "flex",
        gap: "0.25rem",
        position: "absolute",
        right: "0.5rem",
        [mobile]: { display: "none" },
    },
    dot: {
        backgroundColor: "#3c8f58",
        borderRadius: "50%",
        flexShrink: 0,
        height: "0.4375rem",
        width: "0.4375rem",
    },
});
