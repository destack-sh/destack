import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";
import type { Item, Lighting } from "./ledger";
import { Tile } from "./tile";
import { Callout } from "./callout";

/** The twelve services a docs app runs on, numbered as its callouts: rented one by one, or built into one engine. */
export const services: readonly { stacked: Item; destacked: Item }[] = [
    {
        stacked: {
            label: "Identity",
            mark: { logo: "clerk.png" },
            name: "Clerk",
            chips: ["SDK", "webhook"],
            note: "users synced in by webhook",
        },
        destacked: {
            label: "Identity",
            mark: { icon: "user", tint: "#5b7f2e" },
            name: "Accounts",
            chips: ["passkeys"],
            note: "one sign-in, apps and agents",
        },
    },
    {
        stacked: {
            label: "Permissions",
            mark: { logo: "spicedb.png" },
            name: "SpiceDB",
            chips: ["SDK", "sync"],
            note: "permissions copied in by a job",
        },
        destacked: {
            label: "Permissions",
            mark: { icon: "auth", tint: "#a0485f" },
            name: "Access",
            chips: ["roles"],
            note: "one rule set, agents included",
        },
    },
    {
        stacked: {
            label: "Realtime",
            mark: { logo: "liveblocks.png" },
            name: "Liveblocks",
            chips: ["SDK", "auth"],
            note: "a room per page, its own auth",
        },
        destacked: {
            label: "Realtime",
            mark: { icon: "presence", tint: "#c64a17" },
            name: "Multiplayer",
            chips: ["presence"],
            note: "every object live",
        },
    },
    {
        stacked: {
            label: "Data",
            mark: { logo: "supabase.svg" },
            name: "Supabase",
            chips: ["schema"],
            note: "a schema of its own",
        },
        destacked: {
            label: "Data",
            mark: { icon: "storage", tint: "#2f7d8c" },
            name: "Database",
            chips: ["SQL"],
            note: "live queries, offline too",
        },
    },
    {
        stacked: {
            label: "Search",
            mark: { logo: "algolia.svg" },
            name: "Algolia",
            chips: ["SDK", "sync"],
            note: "a second index to keep in sync",
        },
        destacked: {
            label: "Search",
            mark: { icon: "search", tint: "#3d6fb0" },
            name: "Search",
            chips: ["full text"],
            note: "one index over every app",
        },
    },
    {
        stacked: {
            label: "Messaging",
            mark: { logo: "knock.png" },
            name: "Knock",
            chips: ["SDK"],
            note: "called by every service",
        },
        destacked: {
            label: "Messaging",
            mark: { icon: "notify", tint: "#b8862b" },
            name: "Notifications",
            chips: ["push"],
            note: "one inbox for every app",
        },
    },
    {
        stacked: {
            label: "Jobs",
            mark: { logo: "temporal.png" },
            name: "Temporal",
            chips: ["workers"],
            note: "workers you deploy and watch",
        },
        destacked: {
            label: "Jobs",
            mark: { icon: "sync", tint: "#4f8a5b" },
            name: "Workflows",
            chips: ["durable"],
            note: "runs survive restarts",
        },
    },
    {
        stacked: {
            label: "Config",
            mark: { logo: "doppler.png" },
            name: "Doppler",
            chips: ["CLI"],
            note: "keys copied into every SDK",
        },
        destacked: {
            label: "Config",
            mark: { icon: "vault", tint: "#12313c" },
            name: "Secrets",
            chips: ["vault"],
            note: "keys stay on the server",
        },
    },
    {
        stacked: {
            label: "Integrations",
            mark: { logo: "zapier.svg" },
            name: "Zapier",
            chips: ["MCP"],
            note: "actions set up per app",
        },
        destacked: {
            label: "Integrations",
            mark: { icon: "agent", tint: "#6b5ca5" },
            name: "API",
            chips: ["MCP", "OpenAPI"],
            note: "one API for every app and agent",
        },
    },
    {
        stacked: {
            label: "Observability",
            mark: { logo: "sentry.png" },
            name: "Sentry",
            chips: ["SDK"],
            note: "errors from each SDK alone",
        },
        destacked: {
            label: "Observability",
            mark: { icon: "telemetry", tint: "#6d7f86" },
            name: "Telemetry",
            chips: ["traces"],
            note: "every call traced",
        },
    },
    {
        stacked: {
            label: "Deploys",
            mark: { logo: "vercel.png" },
            name: "Vercel",
            chips: ["config"],
            note: "their cloud only",
        },
        destacked: {
            label: "Deploys",
            mark: { icon: "hosts", tint: "#4f8a5b" },
            name: "Hosting",
            chips: ["Node"],
            note: "laptop, server or cloud",
        },
    },
    {
        stacked: {
            label: "Code",
            mark: { logo: "github.png" },
            name: "GitHub",
            chips: ["CI"],
            note: "a pipeline of its own",
        },
        destacked: {
            label: "Code",
            mark: { icon: "source", tint: "#c64a17" },
            name: "Forge",
            chips: ["git"],
            note: "every app's source, forkable",
        },
    },
];

/** The pages in the sidebar, with the open one first. */
const pages: readonly string[] = [
    "Launch plan",
    "Clients",
    "Hiring loop",
    "Pricing",
    "Reading list",
    "Offsite",
];

/** The checklist before launch: each item and whether it is done. */
const checklist: readonly (readonly [item: string, isDone: boolean])[] = [
    ["Freeze the pricing page", true],
    ["Rehearse the demo with Kai", false],
    ["Schedule the announcement", false],
];

/** How far a task has come. */
type Status = "todo" | "started" | "done";

/** The launch tasks in the page's table: id, title, status, assignee and their tint, and due date. */
const tasks: readonly (readonly [
    id: string,
    title: string,
    status: Status,
    assignee: string,
    tint: string,
    due: string,
])[] = [
    ["LCH-12", "Pricing page", "done", "Ada", "#6b5ca5", "Mon 20"],
    ["LCH-14", "Record the demo", "started", "You", "#2f7d8c", "Thu 23"],
    ["LCH-15", "Draft the launch post", "todo", "Agent", "#b8862b", "Thu 23"],
    ["LCH-16", "Email the waitlist", "todo", "Kai", "#5b7f2e", "Fri 24"],
];

/** The words of each status. */
const statusNames: Record<Status, string> = { todo: "Todo", started: "In progress", done: "Done" };

/** Draw the Pages app with the launch plan open, each service it runs on marked by its number and badge. */
export function PagesApp(properties: { isOpen: boolean; lighting: Lighting }) {
    // mark the place of one service, dashed while stacked
    const at = (number: number) => ({
        number,
        isOpen: properties.isOpen,
        lighting: properties.lighting,
    });

    return (
        <div {...stylex.attrs(styles.app)}>
            <nav {...stylex.attrs(styles.sidebar)}>
                <Callout {...at(1)}>
                    <span {...stylex.attrs(styles.row, styles.strong)}>
                        <span
                            {...stylex.attrs(styles.avatar)}
                            style={{ "background-color": "#2f7d8c" }}
                        >
                            Y
                        </span>
                        Your space
                    </span>
                </Callout>
                <Callout {...at(5)}>
                    <span {...stylex.attrs(styles.search)}>
                        Search
                        <span {...stylex.attrs(styles.shortcut)}>⌘K</span>
                    </span>
                </Callout>
                <Callout {...at(6)}>
                    <span {...stylex.attrs(styles.row)}>
                        <span
                            style={{ "mask-image": "url(/diagram/notify.svg)" }}
                            {...stylex.attrs(styles.glyph)}
                        />
                        Inbox
                        <span {...stylex.attrs(styles.badge)}>3</span>
                    </span>
                </Callout>
                <p {...stylex.attrs(styles.group)}>Pages</p>
                <span {...stylex.attrs(styles.pageList)}>
                    {pages.map((page, index) => (
                        <p {...stylex.attrs(styles.pageItem, index === 0 && styles.selected)}>
                            <span
                                style={{ "mask-image": "url(/diagram/file.svg)" }}
                                {...stylex.attrs(styles.glyph)}
                            />
                            {page}
                        </p>
                    ))}
                </span>
                <span {...stylex.attrs(styles.agent)}>
                    <Callout {...at(9)} isBlock>
                        <span {...stylex.attrs(styles.agentCard)}>
                            <span {...stylex.attrs(styles.row, styles.strong)}>
                                <Tile name="agent" />
                                Agent
                            </span>
                            <span {...stylex.attrs(styles.quiet)}>
                                Drafted LCH-15 and asked Kai to review it
                            </span>
                        </span>
                    </Callout>
                </span>
            </nav>
            <div {...stylex.attrs(styles.main)}>
                <article {...stylex.attrs(styles.page)}>
                    <p {...stylex.attrs(styles.toolbar)}>
                        <b {...stylex.attrs(styles.title)}>Launch plan</b>
                        <span {...stylex.attrs(styles.tools)}>
                            <span {...stylex.attrs(styles.quiet, styles.nowrap)}>38 versions</span>
                            <Callout {...at(2)}>
                                <span {...stylex.attrs(styles.share)}>Share</span>
                            </Callout>
                        </span>
                    </p>
                    <p {...stylex.attrs(styles.properties)}>
                        <span {...stylex.attrs(styles.property)}>
                            <span {...stylex.attrs(styles.state, styles.started)} />
                            In progress
                        </span>
                        <span {...stylex.attrs(styles.property)}>Fri 24 Oct</span>
                        <Callout {...at(8)}>
                            <span {...stylex.attrs(styles.property)}>Stripe</span>
                        </Callout>
                        <Callout {...at(7)}>
                            <span {...stylex.attrs(styles.property)}>Reminder Thu</span>
                        </Callout>
                    </p>
                    <p {...stylex.attrs(styles.text)}>
                        Ship to the waitlist on Thursday.{" "}
                        <Callout {...at(3)}>
                            <span>
                                Pricing stays in step with billing
                                <span {...stylex.attrs(styles.cursor)}>Ada</span>
                            </span>
                        </Callout>
                    </p>
                    <Callout {...at(4)} isBlock>
                        <table {...stylex.attrs(styles.table)}>
                            <tbody>
                                {tasks.map(([id, title, status, assignee, tint, due]) => (
                                    <tr>
                                        <td {...stylex.attrs(styles.cell, styles.id)}>{id}</td>
                                        <td {...stylex.attrs(styles.cell, styles.taskTitle)}>
                                            <span {...stylex.attrs(styles.state, styles[status])} />
                                            {title}
                                        </td>
                                        <td
                                            {...stylex.attrs(
                                                styles.cell,
                                                styles.quiet,
                                                styles.wide,
                                            )}
                                        >
                                            {statusNames[status]}
                                        </td>
                                        <td {...stylex.attrs(styles.cell)}>
                                            <span {...stylex.attrs(styles.row)}>
                                                <span
                                                    {...stylex.attrs(styles.avatar)}
                                                    style={{ "background-color": tint }}
                                                >
                                                    {assignee.charAt(0)}
                                                </span>
                                                {assignee}
                                            </span>
                                        </td>
                                        <td
                                            {...stylex.attrs(
                                                styles.cell,
                                                styles.quiet,
                                                styles.wide,
                                            )}
                                        >
                                            {due}
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </Callout>
                    <b {...stylex.attrs(styles.heading)}>Notes from Monday</b>
                    <p {...stylex.attrs(styles.text)}>
                        The waitlist has 1,840 people. Kai sends the first batch of invites on
                        Thursday morning, and the rest follow once the demo is live.
                    </p>
                    <p {...stylex.attrs(styles.file)}>
                        <span
                            style={{ "mask-image": "url(/diagram/file.svg)" }}
                            {...stylex.attrs(styles.glyph)}
                        />
                        pricing-v4.pdf
                        <span {...stylex.attrs(styles.quiet)}>1.2 MB · added by Ada</span>
                    </p>
                    <p {...stylex.attrs(styles.comment)}>
                        <span
                            {...stylex.attrs(styles.avatar)}
                            style={{ "background-color": "#5b7f2e" }}
                        >
                            K
                        </span>
                        <span>
                            <b {...stylex.attrs(styles.strong)}>Kai</b>{" "}
                            <span {...stylex.attrs(styles.quiet)}>
                                Can we move the demo to Wednesday?
                            </span>
                        </span>
                    </p>
                    <p {...stylex.attrs(styles.text)}>
                        Pricing launches with three plans. The team plan replaces the old seat
                        pricing, and existing customers keep their rate for a year.
                    </p>
                    <b {...stylex.attrs(styles.heading)}>Before launch</b>
                    <ul {...stylex.attrs(styles.checklist)}>
                        {checklist.map(([item, isDone]) => (
                            <li {...stylex.attrs(styles.check)}>
                                <span {...stylex.attrs(styles.box, isDone && styles.boxDone)} />
                                <span {...stylex.attrs(isDone && styles.struck)}>{item}</span>
                            </li>
                        ))}
                    </ul>
                </article>
            </div>
            <p {...stylex.attrs(styles.statusBar)}>
                <Callout {...at(11)}>
                    <span {...stylex.attrs(styles.row, styles.nowrap)}>
                        <span {...stylex.attrs(styles.dot)} />
                        Your laptop
                        <span {...stylex.attrs(styles.dot)} />
                        Your server
                    </span>
                </Callout>
                <Callout {...at(10)}>
                    <span {...stylex.attrs(styles.nowrap)}>0 errors · 41 ms</span>
                </Callout>
                <Callout {...at(12)}>
                    <span {...stylex.attrs(styles.nowrap)}>@app/pages 1.4</span>
                </Callout>
            </p>
        </div>
    );
}

/** The Pages app styles. */
const styles = stylex.create({
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: "10rem minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr) auto",
        height: "100%",
        minHeight: 0,
        "@media (max-width: 767px)": { gridTemplateColumns: "minmax(0, 1fr)" },
    },
    sidebar: {
        backgroundColor: color.muted,
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
        display: "flex",
        flexDirection: "column",
        gap: "0.375rem",
        minHeight: 0,
        overflow: "hidden",
        padding: "0.875rem 0.75rem",
        "@media (max-width: 767px)": { display: "none" },
    },
    row: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
    },
    avatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: "#ffffff",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.125rem",
        justifyContent: "center",
        width: "1.125rem",
    },
    search: {
        alignItems: "center",
        flexGrow: 1,
        backgroundColor: color.background,
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.mutedForeground,
        display: "flex",
        justifyContent: "space-between",
        paddingBlock: "0.25rem",
        paddingInline: "0.5rem",
    },
    shortcut: {
        fontFamily: tokens.monoFont,
        fontSize: "0.66rem",
    },
    glyph: {
        backgroundColor: color.mutedForeground,
        flexShrink: 0,
        height: "0.875rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.875rem",
    },
    badge: {
        backgroundColor: tokens.signal,
        borderRadius: "999px",
        color: tokens.signalInk,
        fontSize: "0.62rem",
        fontWeight: 700,
        paddingInline: "0.375rem",
    },
    group: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.62rem",
        letterSpacing: "0.08em",
        margin: 0,
        paddingTop: "0.5rem",
        textTransform: "uppercase",
    },
    pageList: {
        display: "flex",
        flexDirection: "column",
        gap: "0.375rem",
        maskImage: "linear-gradient(to bottom, #000 calc(100% - 1.5rem), transparent)",
        minHeight: 0,
        overflow: "hidden",
    },
    heading: {
        fontSize: "1rem",
        fontWeight: 700,
        marginTop: "0.25rem",
    },
    file: {
        alignItems: "center",
        alignSelf: "start",
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "inline-flex",
        gap: "0.5rem",
        justifySelf: "start",
        margin: 0,
        paddingBlock: "0.375rem",
        paddingInline: "0.625rem",
    },
    checklist: {
        display: "grid",
        gap: "0.375rem",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    check: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
    },
    box: {
        borderColor: color.mutedForeground,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        flexShrink: 0,
        height: "0.875rem",
        width: "0.875rem",
    },
    boxDone: {
        backgroundColor: "#5e6ad2",
        borderColor: "#5e6ad2",
    },
    struck: {
        color: color.mutedForeground,
        textDecorationLine: "line-through",
    },
    comment: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        margin: 0,
    },
    pageItem: {
        alignItems: "center",
        borderRadius: "5px",
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        marginInline: "-0.375rem",
        paddingBlock: "0.1875rem",
        paddingInline: "0.375rem",
    },
    selected: {
        backgroundColor: "rgb(255 121 46 / 14%)",
    },
    main: {
        display: "grid",
        minHeight: 0,
        minWidth: 0,
    },
    toolbar: {
        alignItems: "center",
        display: "flex",
        gap: "1rem",
        margin: 0,
    },
    tools: {
        alignItems: "center",
        display: "flex",
        gap: "1.25rem",
        marginLeft: "auto",
    },
    share: {
        backgroundColor: color.foreground,
        borderRadius: "5px",
        color: color.background,
        fontWeight: 600,
        paddingBlock: "0.125rem",
        paddingInline: "0.625rem",
    },
    page: {
        alignContent: "start",
        display: "grid",
        gap: "0.875rem",
        maskImage: "linear-gradient(to bottom, #000 calc(100% - 3rem), transparent)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "1rem",
        paddingInline: { default: "2rem", "@media (max-width: 767px)": "1rem" },
    },
    title: {
        fontSize: "1.75rem",
        fontWeight: 700,
        letterSpacing: "-0.02em",
        lineHeight: 1.15,
    },
    properties: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem",
        margin: 0,
    },
    property: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "5px",
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
        whiteSpace: "nowrap",
    },
    text: {
        lineHeight: 1.6,
        margin: 0,
    },
    cursor: {
        backgroundColor: "#6b5ca5",
        borderRadius: "3px 3px 3px 0",
        color: "#ffffff",
        fontSize: "0.6rem",
        fontWeight: 600,
        marginLeft: "0.125rem",
        paddingInline: "0.3rem",
        verticalAlign: "super",
    },
    table: {
        borderCollapse: "collapse",
        width: "100%",
    },
    cell: {
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        paddingBlock: "0.4375rem",
        paddingRight: "0.75rem",
        whiteSpace: "nowrap",
    },
    id: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.7rem",
        paddingLeft: "0.25rem",
        width: "4.25rem",
    },
    taskTitle: {
        fontWeight: 500,
    },
    state: {
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1.5px",
        display: "inline-block",
        flexShrink: 0,
        height: "0.6875rem",
        marginRight: "0.5rem",
        verticalAlign: "-0.0625rem",
        width: "0.6875rem",
    },
    todo: {
        borderColor: color.mutedForeground,
    },
    started: {
        backgroundImage: "linear-gradient(90deg, #d9a21b 50%, transparent 50%)",
        borderColor: "#d9a21b",
        marginRight: 0,
    },
    done: {
        backgroundColor: "#5e6ad2",
        borderColor: "#5e6ad2",
    },
    agent: {
        display: "grid",
        flexShrink: 0,
        marginTop: "auto",
        paddingTop: "0.75rem",
    },
    agentCard: {
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
        fontSize: "0.75rem",
        gap: "0.375rem",
        lineHeight: 1.4,
        padding: "0.625rem",
    },
    strong: {
        fontWeight: 600,
    },
    quiet: {
        color: color.mutedForeground,
    },
    wide: {
        "@media (max-width: 767px)": { display: "none" },
    },
    nowrap: {
        whiteSpace: "nowrap",
    },
    statusBar: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderTopColor: tokens.rule,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        color: color.mutedForeground,
        display: "flex",
        flexWrap: "wrap",
        fontSize: "0.75rem",
        gap: "0.5rem 1.75rem",
        gridColumn: "1 / -1",
        margin: 0,
        paddingBlock: "0.5rem",
        paddingInline: "0.875rem",
    },
    dot: {
        backgroundColor: "#4caf6e",
        borderRadius: "50%",
        height: "0.5rem",
        width: "0.5rem",
    },
});
