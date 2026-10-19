import { media } from "@destack/style/media.stylex";
import {
    flat,
    monthlyOf,
    listedOf,
    priceOf,
    type Price,
    sumOf,
    type Workload,
    jobWorkloads,
} from "../figure/price";
import { color, font, shadow, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { createEffect } from "@destack/view";

import { agentPrompt, setupGuide } from "../../content/site";
import { Favicon } from "../figure/glyph";
import { Tile } from "../figure/tile";
import { Glyph } from "../figure/glyph";
import { palette } from "../../palette.stylex";
import { accentOf, type Entry, type Line } from "../figure/item";
import { chrome } from "../figure/chrome.stylex";
import { browserStyles } from "../figure/browser";
import { screen } from "../../layout/screen.stylex";
import { isMotionReduced } from "@destack/view/motion";
import { type Category, type Tool, toolOf, tools } from "../tool/tool";

/** An app every space comes with: its name and its icon. */
type App = { name: string; icon: string };

/** The apps a space comes with, by the job they do. */
const apps = {
    documents: { name: "Pages", icon: "pages" },
    tasks: { name: "Tasks", icon: "tasks" },
    calendar: { name: "Calendar", icon: "calendar" },
    tables: { name: "Tables", icon: "table" },
    canvas: { name: "Canvas", icon: "canvas" },
    workflows: { name: "Workflows", icon: "sync" },
    chat: { name: "Chat", icon: "chat" },
    mail: { name: "Mail", icon: "mail" },
} satisfies Record<string, App>;

/** The job each app does, as the tally labels its row. */
const jobLabels: Record<Job, string> = {
    documents: "Docs",
    tables: "Databases",
    tasks: "Tasks",
    calendar: "Calendar",
    chat: "Chat",
    mail: "Mail",
    canvas: "Whiteboard",
    workflows: "Automation",
};

/** A job a space's app does, by the category of tools it replaces or connects to. */
type Job = keyof typeof apps & Category;

/** The apps every space holds, one to each kind of thing it keeps, in the space's order. */
const spaceJobs: readonly Job[] = [
    "documents",
    "tables",
    "tasks",
    "calendar",
    "chat",
    "mail",
    "canvas",
];

/** The lines of the bill. */
export const lineCount = spaceJobs.length + 1;

/** The tools the wall offers: every tool of a job a space's app does. */
const offered: readonly Tool[] = tools.filter((tool) => tool.category in apps);

/** The tools picked before the reader picks any. */
export const firstPicks: readonly Tool[] = [
    "Notion",
    "Calendly",
    "Linear",
    "Airtable",
    "Slack",
].map(toolOf);

/** Return the subscriptions a set of tools bills under, each once. */
function subscriptionsOf(picked: readonly Tool[]): readonly string[] {
    return [...new Set(picked.map((tool) => tool.subscription))];
}

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
        listed: added.map(listedOf).join(" + "),
    };
}

/** Return the status line of a tool your space stays connected to, or none when replaced. */
function statusOf(tool: Tool): string | undefined {
    return tool.relation.kind === "bridge" ? tool.relation.status : undefined;
}

/** One row of the wall and the bill: a job, your space's app for it, and the tools that do it. */
type Row = { job: Job; label: string; app: App; tools: readonly Tool[] };

/** The rows of the wall and the bill, one per job your space does. */
const rows: readonly Row[] = [...spaceJobs, "workflows" as const].map((job) => ({
    job,
    label: jobLabels[job],
    app: apps[job],
    tools: offered.filter((tool) => tool.category === job),
}));

/** An app you could make in your space by joining the apps of several jobs. */
type Make = { name: string; icon: string; jobs: readonly Job[] };

/** The apps you could make, each joining the apps of the jobs it needs. */
const makes: readonly Make[] = [
    { name: "Launch room", icon: "chat", jobs: ["documents", "chat"] },
    { name: "Roadmap", icon: "tasks", jobs: ["documents", "tasks"] },
    { name: "Client portal", icon: "pages", jobs: ["documents", "tasks", "chat"] },
    { name: "CRM", icon: "table", jobs: ["tables", "mail", "calendar"] },
    { name: "Booking page", icon: "calendar", jobs: ["calendar", "mail"] },
    { name: "Standups", icon: "sync", jobs: ["tasks", "chat"] },
    { name: "Reading list", icon: "table", jobs: ["tables", "documents"] },
];

/** The most apps to make the wall offers at once, as many as fit beside each other. */
const MAKE_COUNT = 2;

/** The milliseconds a mark takes to move to its new place. */
const moveTime = 560;

/** The milliseconds between one mark's move and the next. */
const moveStep = 18;

/** The strong ease-out every move lands on. */
const landing = "cubic-bezier(0.23, 1, 0.32, 1)";

/** Draw your tools a row per job: to pick while stacked, folded into your apps once destacked. */
export function ToolWall(properties: {
    picks: readonly Tool[];
    isOpen: boolean;
    onPick: (tool: Tool) => void;
}) {
    // hold the wall and where each mark sat at the last layout
    let wall: HTMLOListElement | undefined;
    let places = new Map<string, DOMRect>();

    // find a row's picked tools
    const pickedOf = (row: Row) => row.tools.filter((tool) => properties.picks.includes(tool));

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
        <div data-component="ToolWall" {...style.attrs(styles.window)}>
            {/* window strip, level with the bill's beside it */}
            <div aria-hidden="true" {...style.attrs(styles.strip)}>
                <span {...style.attrs(browserStyles.lights)} />
            </div>

            <ol ref={wall} {...style.attrs(styles.wall)}>
                {/* column heads, as tall as the bill's beside it so the rows line up */}
                <li aria-hidden="true" {...style.attrs(styles.row, styles.head)}>
                    <span {...style.attrs(styles.label)}>Job</span>
                    <span {...style.attrs(styles.headLine)}>
                        <span {...style.attrs(styles.heading)}>
                            <span {...style.attrs(styles.label)}>Tools</span>
                            <span {...style.attrs(styles.label)}>
                                {properties.isOpen ? "In your space" : "Pick the ones you use"}
                            </span>
                        </span>

                        {/* the apps you could make from the jobs your picks cover */}
                        {properties.isOpen ? (
                            <span {...style.attrs(styles.makes)}>
                                <span {...style.attrs(styles.label)}>Make</span>
                                {makesOf(properties.picks).map((make) => (
                                    <span
                                        title={make.jobs.map((job) => apps[job].name).join(" + ")}
                                        {...style.attrs(styles.chip, styles.make)}
                                    >
                                        <Tile icon={make.icon} size="small" />
                                        <span {...style.attrs(styles.appName)}>{make.name}</span>
                                        <Glyph name="remix" size={11} />
                                    </span>
                                ))}
                            </span>
                        ) : undefined}
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
                                        isPicked={properties.picks.includes(tool)}
                                        onPick={properties.onPick}
                                    />
                                ))
                            )}
                        </span>
                    </li>
                ))}
            </ol>
        </div>
    );
}

/** Return the apps you could make from the jobs your picks cover, the fullest first. */
function makesOf(picks: readonly Tool[]): readonly Make[] {
    // read the jobs some pick covers, and keep the apps whose jobs all are
    const covered = new Set(
        offered.filter((tool) => picks.includes(tool)).map((tool) => tool.category),
    );
    const ready = makes.filter((make) => make.jobs.every((job) => covered.has(job)));

    return ready
        .toSorted((first, second) => second.jobs.length - first.jobs.length)
        .slice(0, MAKE_COUNT);
}

/** Draw a tool to pick: its mark and its name, underlined once picked. */
function ToolButton(properties: { tool: Tool; isPicked: boolean; onPick: (tool: Tool) => void }) {
    return (
        <button
            type="button"
            data-key={properties.tool.name}
            aria-pressed={properties.isPicked ? "true" : "false"}
            title={[properties.tool.name, statusOf(properties.tool)].filter(Boolean).join(", ")}
            onClick={() => properties.onPick(properties.tool)}
            {...style.attrs(styles.chip, styles.tool, properties.isPicked && styles.picked)}
        >
            <Favicon icon={properties.tool.icon} tint={properties.tool.tint} size={16} />
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
                                  <Favicon icon={tool.icon} tint={tool.tint} size={12} />
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
        ["replaces", picked.filter((tool) => tool.relation.kind === "replace")],
        ["connects to", picked.filter((tool) => tool.relation.kind === "bridge")],
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
export function tallyOf(picks: readonly Tool[]): readonly Line[] {
    // gather each row's picked tools
    const picked = rows.map((row) => ({
        ...row,
        doers: row.tools.filter((tool) => picks.includes(tool)),
    }));

    // price each row on what the rows above it have not billed yet
    const billed = new Set<string>();

    return picked.map((row) => {
        const price = rowPriceOf(row.doers, billed);
        for (const subscription of subscriptionsOf(row.doers)) {
            billed.add(subscription);
        }

        return {
            entry: entryOf(row.label, row.doers, row.app, price),
            workload: workloadOfRow(row.job, row.doers),
            tint:
                row.doers[0] === undefined
                    ? color.mutedForeground
                    : accentOf(row.doers[0].name, row.doers[0].tint),
        };
    });
}

/** Set one tally row. */
function entryOf(label: string, doers: readonly Tool[], app: App, price: Price): Entry {
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
            price: { kind: "included" },
        },
    };
}

/** Sum the work a row's tools take: the job's work once for each tool that does it. */
function workloadOfRow(job: Job, doers: readonly Tool[]): Workload {
    return sumOf(doers.map(() => jobWorkloads[job]));
}

/** Write the prompt that sets up a space, brings in replaced tools and connects the rest. */
export function promptOf(picks: readonly Tool[]) {
    // split the picks into what moves in and what stays connected
    const picked = offered.filter((tool) => picks.includes(tool));
    const moved = listOf(picked.filter((tool) => tool.relation.kind === "replace"));
    const linked = listOf(picked.filter((tool) => tool.relation.kind === "bridge"));
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
    const names = [...new Set(picked.map((tool) => tool.name))];

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
    window: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: chrome.radius,
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: { default: 0, [screen.belowDesktop]: "30rem", [media.maxMd]: 0 },
        overflow: "hidden",
    },
    headLine: {
        alignItems: "center",
        display: "flex",
        gap: "0.75rem",
        justifyContent: "space-between",
        minWidth: 0,
    },
    makes: {
        alignItems: "center",
        display: "flex",
        gap: "0.375rem",
        minWidth: 0,
        overflow: "hidden",
    },
    make: {
        color: palette.signal,
    },
    strip: {
        alignItems: "center",
        backgroundColor: color.muted,
        boxSizing: "border-box",
        display: "flex",
        height: chrome.strip,
        paddingInline: "0.875rem",
    },
    wall: {
        display: "flex",
        flexDirection: "column",
        listStyle: "none",
        margin: 0,
        minHeight: 0,
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
        paddingInline: "0.75rem",
        ":last-child": { borderBottomWidth: 0 },
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
        alignItems: "center",
        boxSizing: "border-box",
        flexBasis: "auto",
        flexGrow: 0,
        height: chrome.toolbar,
        paddingBlock: 0,
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
