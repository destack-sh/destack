import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { tokens } from "../style/tokens.stylex";
import { coverOf, launch, messages, month, perks, posts, sources, tasks, times } from "./corner";
import { Fade } from "./fade";
import { Favicon, Glyph } from "./glyph";
import type { Entry } from "./ledger";
import type { Stagger } from "./stagger";
import { rentedPageOf } from "./vendor";

/** Who looks at your space. */
export type Viewer = "Me" | "Agent" | "Public";

/** Everyone who can look at your space, in the order the switcher offers them. */
export const viewers: readonly Viewer[] = ["Me", "Agent", "Public"];

/** What you keep, numbered as the browser marks them: rented as a site each, or held in your own space. */
export const holdings: readonly Entry[] = [
    {
        stacked: {
            label: "Launch plan",
            mark: { icon: "notion", tint: "#191919" },
            name: "Notion",
            chips: ["public link"],
            note: "open to whoever finds the URL",
        },
        destacked: {
            label: "Launch plan",
            mark: { icon: "pages", tint: "#3d6fb0" },
            name: "Pages",
            chips: ["agent"],
            note: "shared with your agent by name",
        },
    },
    {
        stacked: {
            label: "Tasks",
            mark: { icon: "linear", tint: "#5e6ad2" },
            name: "Linear",
            chips: ["per seat"],
            note: "a paid seat for every viewer",
        },
        destacked: {
            label: "Tasks",
            mark: { icon: "tasks", tint: "#c64a17" },
            name: "Tasks",
            chips: ["Me"],
            note: "yours until you share it",
        },
    },
    {
        stacked: {
            label: "Chat",
            mark: { icon: "slack", tint: "#4a154b" },
            name: "Slack",
            chips: ["guest invite"],
            note: "a guest invite for every channel",
        },
        destacked: {
            label: "Chat",
            mark: { icon: "chat", tint: "#4f8a5b" },
            name: "Chat",
            chips: ["agent"],
            note: "the same thread, in your space",
        },
    },
    {
        stacked: {
            label: "Code",
            mark: { icon: "github", tint: "#181717" },
            name: "GitHub",
            chips: ["separate repo"],
            note: "code kept apart from the apps",
        },
        destacked: {
            label: "Code",
            mark: { icon: "source", tint: "#c64a17" },
            name: "Forge",
            chips: ["fork"],
            note: "source beside every app",
        },
    },
    {
        stacked: {
            label: "Website",
            mark: { icon: "framer", tint: "#0a0a0a" },
            name: "Framer",
            chips: ["their domain"],
            note: "edited only in their builder",
        },
        destacked: {
            label: "Website",
            mark: { icon: "pages", tint: "#3d6fb0" },
            name: "Pages",
            chips: ["public"],
            note: "a page on your own domain",
        },
    },
    {
        stacked: {
            label: "Booking",
            mark: { icon: "calendly", tint: "#006bff" },
            name: "Calendly",
            chips: ["synced calendar"],
            note: "free times copied, never live",
        },
        destacked: {
            label: "Booking",
            mark: { icon: "calendar", tint: "#c64a17" },
            name: "Calendar",
            chips: ["public"],
            note: "times straight from your calendar",
        },
    },
    {
        stacked: {
            label: "Waitlist",
            mark: { icon: "replit", tint: "#f26207" },
            name: "Replit",
            chips: ["own database"],
            note: "signups in a database of its own",
        },
        destacked: {
            label: "Waitlist",
            mark: { icon: "table", tint: "#4f8a5b" },
            name: "Waitlist",
            chips: ["built"],
            note: "signups in your existing tables",
        },
    },
];

/** One page the browser shows: its ledger row, its path in your space, who sees it once owned, and the rented site's address. */
export type Page = { row: number; path: string; viewers: readonly Viewer[]; site: string };

/** The pages the browser turns through, one per ledger row. */
export const pages: readonly Page[] = [
    { row: 1, path: "/launch", viewers: ["Me", "Agent"], site: "you.notion.site/Launch-plan" },
    { row: 2, path: "/tasks", viewers: ["Me"], site: "linear.app/launch/team/LCH/all" },
    {
        row: 3,
        path: "/chat",
        viewers: ["Me", "Agent"],
        site: "launchkit.slack.com/archives/launch",
    },
    { row: 4, path: "/code", viewers: ["Me", "Agent"], site: "github.com/you/launchkit" },
    { row: 5, path: "/", viewers: ["Me", "Agent", "Public"], site: "you.framer.website" },
    { row: 6, path: "/book", viewers: ["Me", "Agent", "Public"], site: "calendly.com/you/30min" },
    {
        row: 7,
        path: "/waitlist",
        viewers: ["Me", "Agent", "Public"],
        site: "you-waitlist.replit.app",
    },
];

/** The sites you rent, as the browser's tabs while rented: the ledger row, the icon and its tint, and the tab's title. */
const tabs: readonly (readonly [row: number, icon: string, tint: string, title: string])[] = [
    [1, "notion", "#191919", "Launch plan"],
    [2, "linear", "#5e6ad2", "Launch › All issues"],
    [3, "slack", "#4a154b", "launch (Channel) - LaunchKit - Slack"],
    [4, "github", "#181717", "you/launchkit"],
    [5, "framer", "#0a0a0a", "you · Portfolio"],
    [6, "calendly", "#006bff", "30 Minute Meeting | You | Calendly"],
    [7, "replit", "#f26207", "Join the waitlist"],
];

/** The sections of your own site: the ledger row, the name, and who sees it. */
const sections: readonly (readonly [
    row: number,
    name: string,
    icon: string,
    tint: string,
    seen: readonly Viewer[],
])[] = [
    [1, "Pages", "pages", "#3d6fb0", ["Me", "Agent"]],
    [2, "Tasks", "tasks", "#c64a17", ["Me"]],
    [3, "Chat", "chat", "#4f8a5b", ["Me", "Agent"]],
    [4, "Forge", "source", "#c64a17", ["Me", "Agent"]],
    [5, "Site", "user", "#2f7d8c", ["Me", "Agent", "Public"]],
    [6, "Calendar", "calendar", "#c64a17", ["Me", "Agent", "Public"]],
    [7, "Tables", "table", "#4f8a5b", ["Me", "Agent", "Public"]],
];

/**
 * Draw your corner of the cloud in a browser, one page at a time.
 *
 * While rented, every page is a different site in its own tab, with its own look, login and sharing.
 * Once owned, every page is a section of one site at your own address, shown to whoever looks as far as access allows.
 * Every page lays out at the width of a real window and shows at four fifths of its size.
 */
export function SpaceBrowser(properties: {
    isOpenAt: Stagger;
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
}) {
    // show the page when the viewer may see it, or say who may
    const isOpen = () => properties.isOpenAt(properties.page.row);
    const isAllowed = () => !isOpen() || properties.page.viewers.includes(properties.viewer);
    const host = () => properties.page.site.split("/")[0] ?? "";

    return (
        <div data-component="Browser" {...stylex.attrs(styles.browser)}>
            <div data-component="TabStrip" {...stylex.attrs(styles.strip)}>
                <span aria-hidden="true" {...stylex.attrs(styles.lights)} />
                {properties.isOpenAt(1) ? (
                    <span
                        data-component="Tab"
                        data-service="1"
                        {...stylex.attrs(styles.tab, styles.tabOn)}
                    >
                        <Favicon icon="user" tint="#2f7d8c" />
                        <span {...stylex.attrs(styles.tabTitle)}>you.dev</span>
                        <Glyph name="close" size={12} />
                    </span>
                ) : (
                    tabs.map(([row, icon, tint, title]) => (
                        <span
                            data-component="Tab"
                            data-service={String(row)}
                            title={title}
                            {...stylex.attrs(
                                styles.tab,
                                styles.tabEven,
                                row === properties.page.row && styles.tabOn,
                            )}
                        >
                            <Favicon icon={icon} tint={tint} />
                            <span {...stylex.attrs(styles.tabTitle)}>{title}</span>
                        </span>
                    ))
                )}
                <span {...stylex.attrs(styles.newTab)}>
                    <Glyph name="plus" />
                </span>
            </div>
            <div data-component="Toolbar" {...stylex.attrs(styles.toolbar)}>
                <span {...stylex.attrs(styles.tools)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...stylex.attrs(styles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    {isOpen() ? (
                        <span {...stylex.attrs(styles.url)}>
                            <span {...stylex.attrs(styles.host)}>you.dev</span>
                            {properties.page.path === "/" ? "" : properties.page.path}
                            <span data-service="1" {...stylex.attrs(styles.points)}>
                                → destack.app/@you
                            </span>
                        </span>
                    ) : (
                        <span {...stylex.attrs(styles.url)}>
                            <span {...stylex.attrs(styles.host)}>{host()}</span>
                            {properties.page.site.slice(host().length)}
                        </span>
                    )}
                    <Glyph name="star" />
                </span>
                <span {...stylex.attrs(styles.tools)}>
                    <Glyph name="puzzle" />
                    <span {...stylex.attrs(styles.profile)}>Y</span>
                    <Glyph name="menu" />
                </span>
            </div>
            <Fade
                isOpen={isOpen()}
                style={styles.viewport}
                turn={properties.page.row}
                isLarge
                component={isOpen() ? "SitePage" : "RentedPage"}
            >
                <div data-service={String(properties.page.row)} {...stylex.attrs(styles.window)}>
                    {isOpen() ? (
                        <div {...stylex.attrs(styles.site)}>
                            <AppRail page={properties.page} viewer={properties.viewer} />
                            <div {...stylex.attrs(styles.app)}>
                                <AppBar
                                    page={properties.page}
                                    viewer={properties.viewer}
                                    onViewer={properties.onViewer}
                                />
                                {isAllowed() ? (
                                    sectionOf(properties.page.row)
                                ) : (
                                    <p
                                        data-component="AccessNote"
                                        {...stylex.attrs(styles.private)}
                                    >
                                        <Glyph name="lock" size={20} />
                                        {properties.page.viewers.length === 1
                                            ? "Only you can see this page"
                                            : "Only you and your agent can see this page"}
                                    </p>
                                )}
                            </div>
                        </div>
                    ) : (
                        rentedPageOf(properties.page.row)
                    )}
                </div>
            </Fade>
        </div>
    );
}

/** Draw your space's icon rail: one standard app per section, the open one lit. */
function AppRail(properties: { page: Page; viewer: Viewer }) {
    return (
        <nav data-component="AppRail" data-service="1" {...stylex.attrs(styles.rail)}>
            <span {...stylex.attrs(styles.avatar)}>Y</span>
            {sections
                .filter(([, , , , seen]) => seen.includes(properties.viewer))
                .map(([row, name, icon, tint]) => (
                    <span
                        data-component="AppLink"
                        data-service={String(row)}
                        title={name}
                        style={{ "--tint": tint }}
                        {...stylex.attrs(
                            styles.railApp,
                            row === properties.page.row && styles.railAppOn,
                        )}
                    >
                        <Favicon
                            icon={icon}
                            tint={row === properties.page.row ? "#ffffff" : tint}
                            size={18}
                        />
                    </span>
                ))}
        </nav>
    );
}

/** Draw the bar every app in your space shares: its name and place, the search, and the preview of who is looking. */
function AppBar(properties: { page: Page; viewer: Viewer; onViewer: (viewer: Viewer) => void }) {
    const section = () => sections.find(([row]) => row === properties.page.row);

    return (
        <div data-component="AppBar" {...stylex.attrs(styles.bar)}>
            <span {...stylex.attrs(styles.crumbs)}>
                <b>{section()?.[1] ?? ""}</b>
                <span {...stylex.attrs(styles.quiet)}>
                    you.dev{properties.page.path === "/" ? "" : properties.page.path}
                </span>
            </span>
            <span {...stylex.attrs(styles.search)}>
                <Glyph name="search" size={14} />
                Search your space
                <span {...stylex.attrs(styles.kbd)}>⌘K</span>
            </span>
            <span data-component="ViewAs" {...stylex.attrs(styles.viewAs)}>
                <span {...stylex.attrs(styles.viewAsLabel)}>View as</span>
                {viewers.map((viewer) => (
                    <button
                        type="button"
                        aria-pressed={properties.viewer === viewer ? "true" : "false"}
                        onClick={() => properties.onViewer(viewer)}
                        {...stylex.attrs(
                            styles.viewer,
                            properties.viewer === viewer && styles.viewerOn,
                        )}
                    >
                        {viewer}
                    </button>
                ))}
            </span>
        </div>
    );
}

/** Draw a section of your own site by its ledger row. */
function sectionOf(row: number): JSX.Element {
    if (row === 1) {
        return <Launch />;
    } else if (row === 2) {
        return <Tasks />;
    } else if (row === 3) {
        return <Chat />;
    } else if (row === 4) {
        return <Code />;
    } else if (row === 5) {
        return <Writing />;
    } else if (row === 6) {
        return <Book />;
    }

    return <Waitlist />;
}

/** Draw a chip naming who can see a section. */
function Audience(properties: { children: string }) {
    return (
        <span data-component="AudienceChip" {...stylex.attrs(styles.audience)}>
            <Glyph name="key" size={14} />
            {properties.children}
        </span>
    );
}

/** Draw a section's title row the same way in every app: the title, who can see it, and the app's main action. */
function Heading(properties: { title: string; audience: string; action: string }) {
    return (
        <span {...stylex.attrs(styles.heading)}>
            <b {...stylex.attrs(styles.title)}>{properties.title}</b>
            <span {...stylex.attrs(styles.headingEnd)}>
                <Audience>{properties.audience}</Audience>
                <span {...stylex.attrs(styles.action)}>
                    <Glyph name="plus" size={14} />
                    {properties.action}
                </span>
            </span>
        </span>
    );
}

/** Draw your waitlist: the same page at your own address, its signups in the Tables you already use. */
function Waitlist() {
    return (
        <div data-component="Waitlist" {...stylex.attrs(styles.body)}>
            <Heading
                title="The launch tool you'll actually use"
                audience="Public"
                action="New field"
            />
            <span {...stylex.attrs(styles.lead)}>
                Pricing v4 opens on Friday. Leave your email and we'll send you an invite.
            </span>
            <span {...stylex.attrs(styles.signup)}>
                <span {...stylex.attrs(styles.input)}>you@company.com</span>
                <span {...stylex.attrs(styles.join)}>Join waitlist</span>
            </span>
            <ul {...stylex.attrs(styles.perks)}>
                {perks.map(([title, line]) => (
                    <li {...stylex.attrs(styles.perk)}>
                        <b>{title}</b>
                        <span {...stylex.attrs(styles.quiet)}>{line}</span>
                    </li>
                ))}
            </ul>
            <span {...stylex.attrs(styles.built)}>
                <Glyph name="grid" size={14} />
                1,840 signups in your Tables · 24 today · your agent sees them in Launch
            </span>
        </div>
    );
}

/** Draw your writing: the same posts on your own domain. */
function Writing() {
    return (
        <div data-component="Writing" {...stylex.attrs(styles.body)}>
            <Heading title="Notes on building small software" audience="Public" action="New post" />
            <span {...stylex.attrs(styles.lead)}>
                Product, pricing and the tools I run my work on.
            </span>
            <ol {...stylex.attrs(styles.postGrid)}>
                {posts.map(([title, topic, date, from, to], index) => (
                    <li {...stylex.attrs(styles.post)}>
                        <span
                            style={{
                                background: coverOf(index, from, to),
                            }}
                            {...stylex.attrs(styles.cover)}
                        />
                        <span {...stylex.attrs(styles.meta)}>
                            {topic} · {date}
                        </span>
                        <b {...stylex.attrs(styles.postTitle)}>{title}</b>
                    </li>
                ))}
            </ol>
        </div>
    );
}

/** Draw your booking page: free times taken from your own week. */
function Book() {
    return (
        <div data-component="Book" {...stylex.attrs(styles.body)}>
            <Heading title="Book a 30 minute call" audience="Public" action="New slot" />
            <div {...stylex.attrs(styles.booking)}>
                <div {...stylex.attrs(styles.monthPanel)}>
                    <b {...stylex.attrs(styles.panelTitle)}>October 2025</b>
                    <div {...stylex.attrs(styles.days)}>
                        {["S", "M", "T", "W", "T", "F", "S"].map((day) => (
                            <span {...stylex.attrs(styles.weekday)}>{day}</span>
                        ))}
                        {Array.from({ length: month.first }, () => (
                            <span />
                        ))}
                        {Array.from({ length: month.days }, (_, index) => index + 1).map((date) => (
                            <span
                                {...stylex.attrs(
                                    styles.day,
                                    month.open.includes(date) && styles.dayOpen,
                                    date === month.chosen && styles.dayChosen,
                                )}
                            >
                                {date}
                            </span>
                        ))}
                    </div>
                </div>
                <div {...stylex.attrs(styles.slots)}>
                    <b {...stylex.attrs(styles.panelTitle)}>Tuesday, October 28</b>
                    {times.map((time) => (
                        <span {...stylex.attrs(styles.slot)}>{time}</span>
                    ))}
                    <span {...stylex.attrs(styles.note)}>
                        Free times come from your Calendar, and a booking lands there with a prep
                        task.
                    </span>
                </div>
            </div>
        </div>
    );
}

/** Draw the launch plan: the same page, shared with your agent and edited together. */
function Launch() {
    return (
        <div data-component="Launch" {...stylex.attrs(styles.body)}>
            <Heading title="Launch plan" audience="You and your agent" action="New page" />
            <dl {...stylex.attrs(styles.properties)}>
                <dt {...stylex.attrs(styles.key)}>Status</dt>
                <dd {...stylex.attrs(styles.value)}>
                    <span {...stylex.attrs(styles.pill)}>In progress</span>
                </dd>
                <dt {...stylex.attrs(styles.key)}>Launch</dt>
                <dd {...stylex.attrs(styles.value)}>October 24, 2025</dd>
                <dt {...stylex.attrs(styles.key)}>Owner</dt>
                <dd {...stylex.attrs(styles.value)}>You</dd>
            </dl>
            <p {...stylex.attrs(styles.callout)}>
                Ship to the waitlist on Thursday, then open signups on Friday.
            </p>
            <ul {...stylex.attrs(styles.checklist)}>
                {launch.map(([item, isDone]) => (
                    <li {...stylex.attrs(styles.check)}>
                        <span {...stylex.attrs(styles.box, isDone && styles.boxDone)}>
                            {isDone ? <Glyph name="check" size={12} weight={3} /> : undefined}
                        </span>
                        <span {...stylex.attrs(isDone && styles.struck)}>{item}</span>
                        {item.startsWith("Record") ? (
                            <span {...stylex.attrs(styles.cursor)}>Agent</span>
                        ) : undefined}
                    </li>
                ))}
            </ul>
        </div>
    );
}

/** Draw the launch channel: one thread in your own Chat, with your agent in it by name. */
function Chat() {
    return (
        <div data-component="Chat" {...stylex.attrs(styles.body)}>
            <Heading title="# launch" audience="You and your agent" action="New thread" />
            <ul {...stylex.attrs(styles.messages)}>
                {messages.map(([author, time, text]) => (
                    <li {...stylex.attrs(styles.message)}>
                        <span
                            style={{
                                "background-color": author === "Agent" ? "#6b5ca5" : "#2f7d8c",
                            }}
                            {...stylex.attrs(styles.avatar)}
                        >
                            {author === "Agent" ? "A" : "Y"}
                        </span>
                        <span {...stylex.attrs(styles.messageBody)}>
                            <span>
                                <b>{author === "Agent" ? "Agent" : "You"}</b>
                                <span {...stylex.attrs(styles.quiet)}> {time}</span>
                            </span>
                            {text}
                        </span>
                    </li>
                ))}
            </ul>
            <span {...stylex.attrs(styles.composer)}>Message #launch, or ask your agent</span>
        </div>
    );
}

/** Draw the code behind your apps: every app's source in your own Forge. */
function Code() {
    return (
        <div data-component="Code" {...stylex.attrs(styles.body)}>
            <Heading title="launchkit" audience="You and your agent" action="New branch" />
            <span {...stylex.attrs(styles.lead)}>
                The waitlist, the site and My week, with their history.
            </span>
            <table {...stylex.attrs(styles.table)}>
                <tbody>
                    {sources.map(([name, change, when]) => (
                        <tr>
                            <td {...stylex.attrs(styles.td)}>
                                <span {...stylex.attrs(styles.fileName)}>
                                    <Glyph
                                        name={name.includes(".") ? "file" : "folder"}
                                        size={18}
                                    />
                                    {name}
                                </span>
                            </td>
                            <td {...stylex.attrs(styles.td, styles.quiet)}>{change}</td>
                            <td {...stylex.attrs(styles.td, styles.quiet, styles.figure)}>
                                {when}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}

/** Draw the launch tasks: your own list, seen by you alone. */
function Tasks() {
    return (
        <div data-component="Tasks" {...stylex.attrs(styles.body)}>
            <Heading title="Launch tasks" audience="Only you" action="New task" />
            <ul {...stylex.attrs(styles.taskList)}>
                {tasks.map(([id, title, state, owner, tint, due]) => (
                    <li {...stylex.attrs(styles.task)}>
                        <span
                            {...stylex.attrs(
                                styles.box,
                                state === "done" && styles.boxDone,
                                state === "started" && styles.boxStarted,
                            )}
                        >
                            {state === "done" ? (
                                <Glyph name="check" size={12} weight={3} />
                            ) : undefined}
                        </span>
                        <span {...stylex.attrs(styles.taskId)}>{id}</span>
                        <span
                            {...stylex.attrs(styles.taskTitle, state === "done" && styles.struck)}
                        >
                            {title}
                        </span>
                        <span {...stylex.attrs(styles.quiet)}>{due}</span>
                        <span
                            style={{ "background-color": tint }}
                            {...stylex.attrs(styles.avatar, styles.avatarSmall)}
                        >
                            {owner}
                        </span>
                    </li>
                ))}
            </ul>
        </div>
    );
}

/** The space styles: the browser in the page's units, and its pages in the pixels of a real window. */
const styles = stylex.create({
    browser: {
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxShadow: "0 1px 2px rgb(0 0 0 / 4%), 0 8px 24px rgb(0 0 0 / 5%)",
        color: color.cardForeground,
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        height: "100%",
        minHeight: 0,
        overflow: "hidden",
    },
    strip: {
        alignItems: "stretch",
        backgroundColor: color.muted,
        display: "flex",
        gap: "0.125rem",
        height: "2.5rem",
        minWidth: 0,
        paddingInline: "0.875rem 0.625rem",
        paddingTop: "0.375rem",
    },
    lights: {
        alignSelf: "stretch",
        backgroundImage:
            "radial-gradient(circle at 6px 50%, #ff5f57 5.5px, transparent 6px), radial-gradient(circle at 26px 50%, #febc2e 5.5px, transparent 6px), radial-gradient(circle at 46px 50%, #28c840 5.5px, transparent 6px)",
        backgroundRepeat: "no-repeat",
        flexShrink: 0,
        marginRight: "0.875rem",
        width: "52px",
    },
    tab: {
        alignItems: "center",
        borderRadius: "8px 8px 0 0",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        fontSize: "0.72rem",
        gap: "0.5rem",
        justifyContent: "center",
        minWidth: 0,
        paddingInline: "0.75rem",
        position: "relative",
    },
    tabEven: {
        flexBasis: 0,
        flexGrow: 1,
        flexShrink: 1,
        justifyContent: "flex-start",
        maxWidth: "12rem",
        transition: "background-color 160ms ease, color 160ms ease",
    },
    tabOn: {
        backgroundColor: color.card,
        color: color.foreground,
        "::before": {
            backgroundImage: `radial-gradient(circle at 0 0, transparent 8px, ${color.card} 8.5px)`,
            bottom: 0,
            content: "''",
            height: "8px",
            left: "-8px",
            position: "absolute",
            width: "8px",
        },
        "::after": {
            backgroundImage: `radial-gradient(circle at 100% 0, transparent 8px, ${color.card} 8.5px)`,
            bottom: 0,
            content: "''",
            height: "8px",
            position: "absolute",
            right: "-8px",
            width: "8px",
        },
    },
    tabTitle: {
        flexGrow: 1,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    newTab: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        paddingInline: "0.5rem",
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        gap: "0.875rem",
        height: "2.75rem",
        minWidth: 0,
        paddingInline: "0.875rem",
    },
    tools: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        gap: "0.875rem",
    },
    profile: {
        alignItems: "center",
        backgroundColor: "#2f7d8c",
        borderRadius: "50%",
        color: "#ffffff",
        display: "inline-flex",
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.25rem",
        justifyContent: "center",
        width: "1.25rem",
    },
    omnibox: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "999px",
        color: color.mutedForeground,
        display: "flex",
        flexGrow: 1,
        gap: "0.5rem",
        height: "1.875rem",
        minWidth: 0,
        paddingInline: "0.875rem",
    },
    url: {
        alignItems: "baseline",
        display: "flex",
        flexGrow: 1,
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    host: {
        color: color.foreground,
    },
    points: {
        color: tokens.signal,
        fontFamily: tokens.monoFont,
        fontSize: "0.68rem",
        marginLeft: "0.75rem",
    },
    viewport: {
        maskImage: "linear-gradient(to bottom, #000 calc(100% - 1rem), transparent)",
        minHeight: 0,
        overflow: "clip",
        position: "relative",
    },
    window: {
        height: "125%",
        left: 0,
        overflow: "clip",
        position: "absolute",
        top: 0,
        transform: "scale(0.8)",
        transformOrigin: "0 0",
        width: "125%",
    },
    site: {
        backgroundColor: color.card,
        color: color.foreground,
        display: "grid",
        gridTemplateColumns: "64px minmax(0, 1fr)",
        minHeight: "100%",
    },
    app: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minWidth: 0,
    },
    rail: {
        alignContent: "start",
        backgroundColor: color.muted,
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
        display: "grid",
        gap: "10px",
        justifyItems: "center",
        paddingBlock: "16px",
    },
    railApp: {
        alignItems: "center",
        borderRadius: "10px",
        display: "flex",
        height: "38px",
        justifyContent: "center",
        width: "38px",
    },
    railAppOn: {
        backgroundColor: "var(--tint)",
    },
    bar: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        fontSize: "15px",
        gap: "20px",
        height: "60px",
        paddingInline: "28px",
        whiteSpace: "nowrap",
    },
    crumbs: {
        alignItems: "baseline",
        display: "flex",
        gap: "10px",
    },
    search: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "8px",
        color: color.mutedForeground,
        display: "flex",
        flexGrow: 1,
        fontSize: "13px",
        gap: "8px",
        maxWidth: "280px",
        padding: "7px 10px",
    },
    kbd: {
        fontFamily: tokens.monoFont,
        fontSize: "11px",
        marginLeft: "auto",
    },
    headingEnd: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    action: {
        alignItems: "center",
        backgroundColor: color.foreground,
        borderRadius: "8px",
        color: color.background,
        display: "flex",
        fontSize: "13px",
        fontWeight: 600,
        gap: "6px",
        padding: "7px 12px",
    },
    composer: {
        borderColor: tokens.rule,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.mutedForeground,
        fontSize: "15px",
        marginTop: "8px",
        padding: "14px 16px",
    },
    viewAs: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "8px",
        display: "flex",
        gap: "2px",
        marginLeft: "auto",
        padding: "3px",
        flexShrink: 0,
    },
    viewAsLabel: {
        color: color.mutedForeground,
        fontSize: "13px",
        paddingInline: "8px",
    },
    viewer: {
        backgroundColor: "transparent",
        borderRadius: "6px",
        borderWidth: 0,
        color: color.mutedForeground,
        cursor: "pointer",
        fontFamily: "inherit",
        fontSize: "13px",
        paddingBlock: "4px",
        paddingInline: "10px",
    },
    viewerOn: {
        backgroundColor: color.card,
        boxShadow: "0 1px 2px rgb(0 0 0 / 10%)",
        color: color.foreground,
        fontWeight: 600,
    },
    avatar: {
        alignItems: "center",
        backgroundColor: "#2f7d8c",
        borderRadius: "50%",
        color: "#ffffff",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "13px",
        fontWeight: 700,
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    avatarSmall: {
        fontSize: "10px",
        height: "22px",
        width: "22px",
    },
    audience: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "999px",
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "13px",
        fontWeight: 500,
        gap: "6px",
        justifySelf: "start",
        paddingBlock: "4px",
        paddingInline: "12px",
        whiteSpace: "nowrap",
    },
    body: {
        alignContent: "start",
        display: "grid",
        gap: "20px",
        padding: "40px",
    },
    heading: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
        justifyContent: "space-between",
    },
    title: {
        fontSize: "36px",
        fontWeight: 700,
        letterSpacing: "-0.02em",
        lineHeight: 1.15,
    },
    lead: {
        color: color.mutedForeground,
        fontSize: "18px",
    },
    signup: {
        display: "flex",
        gap: "10px",
        maxWidth: "560px",
    },
    input: {
        borderColor: tokens.rule,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.mutedForeground,
        flexGrow: 1,
        fontSize: "16px",
        paddingBlock: "14px",
        paddingInline: "16px",
    },
    join: {
        backgroundColor: color.foreground,
        borderRadius: "10px",
        color: color.background,
        fontSize: "16px",
        fontWeight: 600,
        paddingBlock: "14px",
        paddingInline: "24px",
    },
    perks: {
        display: "grid",
        gap: "14px",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        listStyle: "none",
        margin: "8px 0 0",
        padding: 0,
    },
    perk: {
        borderColor: tokens.rule,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
        fontSize: "15px",
        gap: "6px",
        padding: "16px",
    },
    built: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "8px",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "14px",
        gap: "8px",
        marginTop: "12px",
        padding: "12px 14px",
    },
    postGrid: {
        display: "grid",
        gap: "24px",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        listStyle: "none",
        margin: "12px 0 0",
        padding: 0,
    },
    post: {
        alignContent: "start",
        display: "grid",
        gap: "10px",
    },
    cover: {
        aspectRatio: "4 / 3",
        borderRadius: "12px",
    },
    meta: {
        color: color.mutedForeground,
        fontSize: "13px",
    },
    postTitle: {
        fontSize: "19px",
        fontWeight: 600,
        lineHeight: 1.3,
    },
    booking: {
        borderColor: tokens.rule,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr) 240px",
    },
    monthPanel: {
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
        alignContent: "start",
        display: "grid",
        gap: "12px",
        padding: "20px 28px",
    },
    panelTitle: {
        fontSize: "17px",
        fontWeight: 600,
    },
    days: {
        display: "grid",
        gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
        justifyItems: "center",
        rowGap: "2px",
    },
    weekday: {
        color: color.mutedForeground,
        fontSize: "12px",
        textAlign: "center",
    },
    day: {
        alignItems: "center",
        borderRadius: "50%",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "15px",
        height: "38px",
        justifyContent: "center",
        width: "38px",
    },
    dayOpen: {
        backgroundColor: "rgb(60 143 88 / 12%)",
        color: "#3c8f58",
        fontWeight: 700,
    },
    dayChosen: {
        backgroundColor: "#3c8f58",
        color: "#ffffff",
    },
    slots: {
        alignContent: "start",
        display: "grid",
        gap: "10px",
        padding: "24px",
    },
    slot: {
        borderColor: "#3c8f58",
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "#3c8f58",
        fontSize: "16px",
        fontWeight: 700,
        paddingBlock: "12px",
        textAlign: "center",
    },
    note: {
        color: color.mutedForeground,
        fontSize: "13px",
        lineHeight: 1.45,
        marginTop: "6px",
    },
    properties: {
        display: "grid",
        fontSize: "15px",
        gridAutoRows: "32px",
        gridTemplateColumns: "140px minmax(0, 1fr)",
        margin: 0,
    },
    key: {
        alignSelf: "center",
        color: color.mutedForeground,
    },
    value: {
        alignSelf: "center",
        margin: 0,
    },
    pill: {
        backgroundColor: "rgb(217 162 27 / 18%)",
        borderRadius: "999px",
        color: "#8a6510",
        fontSize: "13px",
        fontWeight: 600,
        paddingBlock: "2px",
        paddingInline: "10px",
    },
    callout: {
        backgroundColor: color.muted,
        borderRadius: "8px",
        fontSize: "16px",
        margin: 0,
        padding: "16px 18px",
    },
    checklist: {
        display: "grid",
        fontSize: "16px",
        gap: "10px",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    check: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    box: {
        alignItems: "center",
        borderColor: color.mutedForeground,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: "#ffffff",
        display: "flex",
        flexShrink: 0,
        height: "18px",
        justifyContent: "center",
        width: "18px",
    },
    boxDone: {
        backgroundColor: "#3c8f58",
        borderColor: "#3c8f58",
    },
    boxStarted: {
        borderColor: "#b8862b",
    },
    struck: {
        opacity: 0.45,
        textDecorationLine: "line-through",
    },
    cursor: {
        backgroundColor: "#6b5ca5",
        borderRadius: "4px",
        color: "#ffffff",
        fontSize: "11px",
        fontWeight: 600,
        paddingBlock: "1px",
        paddingInline: "6px",
    },
    table: {
        borderCollapse: "collapse",
        fontSize: "15px",
        width: "100%",
    },
    td: {
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        height: "52px",
        whiteSpace: "nowrap",
    },
    figure: {
        textAlign: "right",
    },
    fileName: {
        alignItems: "center",
        display: "flex",
        fontWeight: 500,
        gap: "12px",
    },
    messages: {
        display: "grid",
        gap: "18px",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    message: {
        alignItems: "flex-start",
        display: "flex",
        gap: "12px",
    },
    messageBody: {
        display: "grid",
        fontSize: "15px",
        gap: "2px",
        lineHeight: 1.45,
    },
    taskList: {
        display: "grid",
        fontSize: "15px",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    task: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        gap: "14px",
        height: "52px",
    },
    taskId: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "13px",
        width: "64px",
    },
    taskTitle: {
        flexGrow: 1,
        fontWeight: 500,
    },
    private: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "18px",
        gap: "10px",
        justifyContent: "center",
        margin: 0,
        minHeight: "420px",
    },
    quiet: {
        color: color.mutedForeground,
    },
});
