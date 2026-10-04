import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal, onSettled, type JSX } from "@destack/view";

import { lattice } from "../style/lattice.stylex";
import { paper } from "../style/paper.stylex";
import { tokens } from "../style/tokens.stylex";
import { Entry, type Form } from "./entry";
import { Caption, Window } from "./window";

/** The media query for screens narrower than the desktop frame, where cells pair up. */
const narrow = "@media (min-width: 768px) and (max-width: 1099px)";
/** The media query for phone-width screens, where cells stack. */
const mobile = "@media (max-width: 767px)";

/** The adjective every app answers to. */
const destackable: Form = {
    syllables: ["de", "stack", "a", "ble"],
    pronunciation: "/diːˈstakəbl/",
    partOfSpeech: "adjective",
    senses: [
        {
            definition: "open to you, your friends and your agents",
            highlight: ["open"],
            sentence:
                "Every object and action in an app is reachable from the app, the CLI and any agent, through the permissions you grant.",
        },
        {
            definition: "open to change and remix, down to the source",
            highlight: ["remix"],
            sentence:
                "Every app installs with its source, and you or your agent change it on a branch.",
        },
    ],
};

/** The pause between one frame lighting and the next, in milliseconds. */
const stepTime = 900;

/** The notes the change runs on, with the due dates it adds. */
const notes: readonly (readonly [title: string, due: string])[] = [
    ["Launch plan", "Fri"],
    ["Hiring loop", "Mon"],
    ["Q4 budget", "31 Oct"],
];

/** The tasks on the board, by status: title, priority and due day, if any. */
const board: readonly (readonly [
    status: string,
    tasks: readonly (readonly [title: string, isUrgent: boolean, due: string | undefined])[],
])[] = [
    [
        "Open",
        [
            ["Launch post", true, "Fri"],
            ["Pricing page", false, "Mon"],
        ],
    ],
    ["Active", [["Due dates", false, "Today"]]],
    ["Done", [["Invite flow", false, undefined]]],
];

/** The first-party apps: what each holds, the objects and actions it opens, and who reaches it. */
const apps: readonly {
    name: string;
    description: string;
    objects: readonly string[];
    actions: readonly string[];
    members: string;
    screen: () => JSX.Element;
}[] = [
    {
        name: "Home",
        description: "Spaces, apps and notifications",
        objects: ["space", "host", "notification"],
        actions: ["invite", "enroll", "read"],
        members: "you · agent",
        screen: () => <HomeScreen />,
    },
    {
        name: "Notes",
        description: "Notes with sharing and history",
        objects: ["notebook", "note"],
        actions: ["share", "pin", "archive"],
        members: "you · Ada · agent",
        screen: () => <NotesScreen />,
    },
    {
        name: "Pages",
        description: "Documents built from blocks",
        objects: ["page", "block"],
        actions: ["publish", "move", "export"],
        members: "team · agent",
        screen: () => <PagesScreen />,
    },
    {
        name: "Tasks",
        description: "Projects for people and agents",
        objects: ["project", "task"],
        actions: ["assign", "complete", "comment"],
        members: "you · Ada · agents",
        screen: () => <TasksScreen />,
    },
];

/** Show the apps and who reaches them, then one change to an app from request to merge. */
export function Apps() {
    // hold whether the change is playing, and the section that starts it
    const [isPlaying, setIsPlaying] = createSignal(false);
    let section: HTMLElement | undefined;

    // play the change once, frame by frame, when the section first comes into view
    onSettled(() => {
        const watcher = new IntersectionObserver(
            ([entry]) => {
                if (entry?.isIntersecting === true) {
                    setIsPlaying(true);
                    watcher.disconnect();
                }
            },
            { threshold: 0.4 },
        );
        if (section) {
            watcher.observe(section);
        }

        return () => watcher.disconnect();
    });

    return (
        <section
            ref={section}
            data-universe
            style={{ "--step-time": `${stepTime}ms` }}
            {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.section)}
        >
            <Entry form={destackable} />
            {apps.map((app) => (
                <article {...stylex.attrs(styles.cell, styles.app)}>
                    <Window title={app.name} tag={app.members} style={styles.appWindow}>
                        {app.screen()}
                    </Window>
                    <h3 {...stylex.attrs(styles.appName)}>{app.name}</h3>
                    <p {...stylex.attrs(styles.appDescription)}>{app.description}</p>
                    <dl {...stylex.attrs(styles.terms)}>
                        <dt {...stylex.attrs(styles.term)}>objects</dt>
                        <dd {...stylex.attrs(styles.termValue)}>{app.objects.join(", ")}</dd>
                        <dt {...stylex.attrs(styles.term)}>actions</dt>
                        <dd {...stylex.attrs(styles.termValue)}>{app.actions.join(", ")}</dd>
                    </dl>
                </article>
            ))}
            <Frame isPlaying={isPlaying()} verb="Ask" rest=" for a change" title="Agent" step={0}>
                <div {...stylex.attrs(styles.chat)}>
                    <p {...stylex.attrs(styles.bubble, styles.mine)}>Add due dates to my notes</p>
                    <p {...stylex.attrs(styles.bubble, styles.theirs)}>
                        Adding a <code {...stylex.attrs(styles.code)}>due</code> field to{" "}
                        <code {...stylex.attrs(styles.code)}>note</code>, on a branch.
                    </p>
                    <p {...stylex.attrs(styles.bubble, styles.mine)}>And sort by it</p>
                    <p {...stylex.attrs(styles.bubble, styles.theirs)}>
                        Done. Three notes had dates in their text, so I filled those in.
                    </p>
                </div>
            </Frame>
            <Frame
                isPlaying={isPlaying()}
                verb="Edit"
                rest=" the app"
                title="note.ts"
                tag="notes"
                step={1}
            >
                <pre {...stylex.attrs(styles.diff)}>
                    {"fields: {\n  title: field.string(),\n  body: field.text(),\n"}
                    <ins {...stylex.attrs(styles.added)}>{"+ due: field.time().optional(),"}</ins>
                    {"\n},"}
                </pre>
            </Frame>
            <Frame
                isPlaying={isPlaying()}
                verb="Preview"
                rest=" locally"
                title="Notes"
                tag="due-dates"
                isBranch
                step={2}
            >
                <ul {...stylex.attrs(styles.rows)}>
                    {notes.map(([title, due]) => (
                        <li {...stylex.attrs(styles.row)}>
                            {title}
                            <span {...stylex.attrs(styles.due)}>{due}</span>
                        </li>
                    ))}
                </ul>
            </Frame>
            <Frame
                isPlaying={isPlaying()}
                verb="Keep"
                rest=", share or publish"
                title="Merge due-dates"
                step={3}
            >
                <div {...stylex.attrs(styles.merge)}>
                    <ul {...stylex.attrs(styles.changes)}>
                        <li {...stylex.attrs(styles.change)}>
                            Fields<b {...stylex.attrs(styles.count)}>+1 due</b>
                        </li>
                        <li {...stylex.attrs(styles.change)}>
                            Notes kept<b {...stylex.attrs(styles.count)}>3 of 3</b>
                        </li>
                        <li {...stylex.attrs(styles.change)}>
                            Conflicts<b {...stylex.attrs(styles.count)}>0</b>
                        </li>
                    </ul>
                    <span {...stylex.attrs(paper.key, paper.primary, paper.small, styles.mergeKey)}>
                        Merge into main
                    </span>
                </div>
            </Frame>
        </section>
    );
}

/** Frame one step of the change: a window titled by its file or app, and a caption under it. */
function Frame(properties: {
    isPlaying: boolean;
    verb: string;
    rest: string;
    title: string;
    tag?: string;
    isBranch?: boolean;
    step: number;
    children: JSX.Element;
}) {
    return (
        <article
            style={{ "--step": String(properties.step) }}
            {...stylex.attrs(styles.cell, styles.frame, properties.isPlaying && styles.playing)}
        >
            <Window
                title={properties.title}
                {...(properties.tag === undefined ? {} : { tag: properties.tag })}
                isBranch={properties.isBranch === true}
            >
                {properties.children}
            </Window>
            <Caption verb={properties.verb} rest={properties.rest} />
        </article>
    );
}

/** Draw an app's icon tile, as the stack figure draws them. */
function AppIcon(properties: { icon: string; tint: string }) {
    return (
        <span style={{ "background-color": properties.tint }} {...stylex.attrs(screen.icon)}>
            <span
                style={{ "mask-image": `url(/diagram/${properties.icon}.svg)` }}
                {...stylex.attrs(screen.glyph)}
            />
        </span>
    );
}

/** Draw Home: the space, its apps, the person's host and new notifications. */
function HomeScreen() {
    return (
        <div {...stylex.attrs(screen.pad)}>
            <p {...stylex.attrs(screen.caption)}>
                Ada's space<span {...stylex.attrs(screen.badge)}>3 new</span>
            </p>
            <span {...stylex.attrs(screen.apps)}>
                <span {...stylex.attrs(screen.app)}>
                    <AppIcon icon="file" tint="#b8862b" />
                    Notes
                </span>
                <span {...stylex.attrs(screen.app)}>
                    <AppIcon icon="tasks" tint="#c64a17" />
                    Tasks
                </span>
                <span {...stylex.attrs(screen.app)}>
                    <AppIcon icon="pages" tint="#3d6fb0" />
                    Pages
                </span>
            </span>
            <p {...stylex.attrs(screen.row)}>
                <span {...stylex.attrs(screen.online)} />
                MacBook Pro<span {...stylex.attrs(screen.quiet)}>online</span>
            </p>
        </div>
    );
}

/** Draw Notes: notebooks beside their notes, one pinned and one being edited. */
function NotesScreen() {
    return (
        <div {...stylex.attrs(screen.split)}>
            <ul {...stylex.attrs(screen.sidebar)}>
                <li {...stylex.attrs(screen.side)}>
                    Personal<span {...stylex.attrs(screen.quiet)}>12</span>
                </li>
                <li {...stylex.attrs(screen.side, screen.selected)}>
                    Work<span {...stylex.attrs(screen.quiet)}>4</span>
                </li>
            </ul>
            <ul {...stylex.attrs(screen.list)}>
                <li {...stylex.attrs(screen.row)}>
                    <span {...stylex.attrs(screen.pin)} />
                    Launch plan<span {...stylex.attrs(screen.avatar)}>A</span>
                </li>
                <li {...stylex.attrs(screen.row)}>Hiring loop</li>
                <li {...stylex.attrs(screen.row)}>Q4 budget</li>
            </ul>
        </div>
    );
}

/** Draw Pages: the page tree beside the open page. */
function PagesScreen() {
    return (
        <div {...stylex.attrs(screen.split)}>
            <ul {...stylex.attrs(screen.sidebar)}>
                <li {...stylex.attrs(screen.side)}>Handbook</li>
                <li {...stylex.attrs(screen.side, screen.nested, screen.selected)}>Onboarding</li>
                <li {...stylex.attrs(screen.side, screen.nested)}>Benefits</li>
                <li {...stylex.attrs(screen.side)}>Roadmap</li>
            </ul>
            <div {...stylex.attrs(screen.page)}>
                <b {...stylex.attrs(screen.title)}>Onboarding</b>
                <i {...stylex.attrs(screen.text)} />
                <i {...stylex.attrs(screen.text, screen.short)} />
                <i {...stylex.attrs(screen.callout)} />
                <i {...stylex.attrs(screen.text)} />
            </div>
        </div>
    );
}

/** Draw Tasks: a project's board from open to done. */
function TasksScreen() {
    return (
        <div {...stylex.attrs(screen.board)}>
            {board.map(([status, tasks]) => (
                <div {...stylex.attrs(screen.column)}>
                    <p {...stylex.attrs(screen.caption)}>
                        {status}
                        <span {...stylex.attrs(screen.quiet)}>{tasks.length}</span>
                    </p>
                    {tasks.map(([title, isUrgent, due]) => (
                        <p {...stylex.attrs(screen.task)}>
                            <span {...stylex.attrs(screen.priority, isUrgent && screen.urgent)} />
                            {title}
                            {due !== undefined && <span {...stylex.attrs(screen.due)}>{due}</span>}
                        </p>
                    ))}
                </div>
            ))}
        </div>
    );
}

/** A frame lighting up in its turn. */
const light = stylex.keyframes({
    from: { opacity: 0.2, transform: "translateY(0.5rem)" },
});

/** The section styles. */
const styles = stylex.create({
    section: {
        gridTemplateRows: `auto repeat(2, calc(${tokens.stage} * 3 - 0.875rem))`,
        "@media (max-width: 1099px)": { gridTemplateRows: "auto" },
    },
    cell: {
        alignContent: "center",
        borderBottomWidth: 0,
        borderColor: tokens.rule,
        borderLeftWidth: 0,
        borderRightWidth: {
            default: tokens.hairline,
            ":nth-of-type(4n)": 0,
            [narrow]: { default: tokens.hairline, ":nth-of-type(2n)": 0 },
            [mobile]: 0,
        },
        borderStyle: "solid",
        borderTopWidth: tokens.hairline,
        gridColumn: "span 3",
        minWidth: 0,
        paddingBottom: "3rem",
        paddingInline: tokens.inset,
        [narrow]: { gridColumn: "span 6" },
        [mobile]: { gridColumn: "1 / -1", paddingBottom: "2rem" },
    },
    playing: {
        animationDelay: "calc(var(--step) * var(--step-time))",
        animationDuration: "600ms",
        animationFillMode: "backwards",
        animationName: light,
        animationTimingFunction: "cubic-bezier(0.2, 0.8, 0.2, 1)",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    frame: {
        display: "grid",
        gap: "1.125rem",
        gridTemplateRows: `calc(${tokens.stage} * 3 - 7.875rem) auto`,
    },
    chat: {
        alignContent: "end",
        display: "grid",
        fontSize: "0.88rem",
        gap: "0.5rem",
        padding: "0.75rem",
    },
    bubble: {
        lineHeight: 1.35,
        margin: 0,
        maxWidth: "88%",
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    mine: {
        backgroundColor: tokens.space,
        color: tokens.cream,
        justifySelf: "end",
    },
    theirs: {
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "1.5px",
    },
    code: {
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 600,
    },
    diff: {
        fontFamily: tokens.monoFont,
        fontSize: "clamp(0.66rem, 0.95vw, 0.78rem)",
        lineHeight: 1.75,
        margin: 0,
        overflow: "hidden",
        paddingBlock: "0.875rem",
        paddingInline: "0.75rem",
    },
    added: {
        backgroundColor: "rgb(255 121 46 / 22%)",
        display: "block",
        fontWeight: 600,
        marginInline: "-0.75rem",
        paddingInline: "0.75rem",
        textDecoration: "none",
    },
    rows: {
        listStyle: "none",
        margin: 0,
        paddingBlock: "0.25rem",
        paddingInline: "0.75rem",
    },
    row: {
        alignItems: "center",
        borderBottomColor: "rgb(25 54 64 / 14%)",
        borderBottomStyle: { default: "solid", ":last-child": "none" },
        borderBottomWidth: "1px",
        display: "flex",
        fontSize: "0.92rem",
        height: "2.5rem",
        justifyContent: "space-between",
    },
    due: {
        backgroundColor: tokens.signal,
        fontFamily: tokens.monoFont,
        fontSize: "0.74rem",
        fontWeight: 600,
        paddingInline: "0.5rem",
    },
    merge: {
        display: "grid",
        gridTemplateRows: "1fr auto",
        paddingBottom: "0.75rem",
        paddingInline: "0.75rem",
    },
    changes: {
        fontSize: "0.88rem",
        listStyle: "none",
        margin: 0,
        padding: 0,
        paddingTop: "0.5rem",
    },
    change: {
        alignItems: "center",
        borderBottomColor: "rgb(25 54 64 / 30%)",
        borderBottomStyle: "dashed",
        borderBottomWidth: "1px",
        display: "flex",
        height: "1.875rem",
        justifyContent: "space-between",
    },
    count: {
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
    },
    mergeKey: {
        boxShadow: "none",
        width: "100%",
    },
    app: {
        display: "flex",
        flexDirection: "column",
        gap: "0.375rem",
        justifyContent: "center",
        paddingBottom: "2rem",
        paddingTop: "2rem",
    },
    appWindow: {
        height: `calc(${tokens.stage} * 3 - 12.875rem)`,
        marginBottom: "0.875rem",
    },
    appName: {
        fontSize: "1.05rem",
        fontWeight: 600,
        margin: 0,
    },
    appDescription: {
        color: color.mutedForeground,
        margin: 0,
    },
    terms: {
        columnGap: "0.75rem",
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.74rem",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        margin: 0,
        marginTop: "0.625rem",
        rowGap: "0.2rem",
    },
    term: {
        color: tokens.signal,
    },
    termValue: {
        margin: 0,
    },
});

/** The styles of the app screens inside the tiles. */
const screen = stylex.create({
    pad: {
        display: "grid",
        alignContent: "start",
        gap: "0.625rem",
        padding: "0.75rem",
    },
    caption: {
        alignItems: "center",
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.68rem",
        fontWeight: 600,
        gap: "0.375rem",
        justifyContent: "space-between",
        margin: 0,
    },
    badge: {
        backgroundColor: tokens.signal,
        fontSize: "0.62rem",
        paddingInline: "0.3rem",
    },
    apps: {
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
    },
    app: {
        alignItems: "center",
        display: "grid",
        fontSize: "0.7rem",
        gap: "0.25rem",
        justifyItems: "center",
    },
    icon: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        height: "2rem",
        justifyContent: "center",
        width: "2rem",
    },
    glyph: {
        backgroundColor: tokens.cream,
        height: "1.125rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "1.125rem",
    },
    row: {
        alignItems: "center",
        borderBottomColor: "rgb(25 54 64 / 14%)",
        borderBottomStyle: { default: "solid", ":last-child": "none" },
        borderBottomWidth: "1px",
        display: "flex",
        fontSize: "0.76rem",
        gap: "0.375rem",
        margin: 0,
        minHeight: "1.875rem",
    },
    online: {
        backgroundColor: "#4f8a5b",
        borderRadius: "50%",
        height: "0.4375rem",
        width: "0.4375rem",
    },
    quiet: {
        color: "rgb(25 54 64 / 55%)",
        fontFamily: tokens.monoFont,
        fontSize: "0.66rem",
        marginLeft: "auto",
    },
    split: {
        display: "grid",
        gridTemplateColumns: "38% minmax(0, 1fr)",
    },
    sidebar: {
        backgroundColor: "rgb(25 54 64 / 6%)",
        borderRightColor: "rgb(25 54 64 / 18%)",
        borderRightStyle: "solid",
        borderRightWidth: "1px",
        listStyle: "none",
        margin: 0,
        padding: "0.5rem",
    },
    side: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.72rem",
        gap: "0.25rem",
        minHeight: "1.625rem",
        paddingInline: "0.375rem",
    },
    nested: {
        paddingLeft: "1rem",
    },
    selected: {
        backgroundColor: tokens.cream,
        boxShadow: `inset 2px 0 0 ${tokens.signal}`,
        fontWeight: 600,
    },
    list: {
        listStyle: "none",
        margin: 0,
        paddingInline: "0.625rem",
    },
    pin: {
        backgroundColor: tokens.signal,
        borderRadius: "50%",
        height: "0.375rem",
        width: "0.375rem",
    },
    avatar: {
        alignItems: "center",
        backgroundColor: "#6b5ca5",
        borderRadius: "50%",
        color: tokens.cream,
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.56rem",
        fontWeight: 600,
        height: "1rem",
        justifyContent: "center",
        marginLeft: "auto",
        width: "1rem",
    },
    page: {
        alignContent: "start",
        display: "grid",
        gap: "0.4375rem",
        padding: "0.75rem",
    },
    title: {
        fontSize: "0.85rem",
        fontWeight: 600,
    },
    text: {
        backgroundColor: "rgb(25 54 64 / 30%)",
        height: "4px",
    },
    short: {
        width: "70%",
    },
    callout: {
        backgroundColor: "rgb(255 121 46 / 14%)",
        borderLeftColor: tokens.signal,
        borderLeftStyle: "solid",
        borderLeftWidth: "3px",
        height: "1.5rem",
    },
    board: {
        display: "grid",
        gap: "0.375rem",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        padding: "0.5rem",
    },
    column: {
        alignContent: "start",
        backgroundColor: "rgb(25 54 64 / 6%)",
        display: "grid",
        gap: "0.3rem",
        padding: "0.3rem",
    },
    task: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        borderColor: "rgb(25 54 64 / 40%)",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        flexWrap: "wrap",
        fontSize: "0.62rem",
        gap: "0.25rem",
        lineHeight: 1.25,
        margin: 0,
        padding: "0.3rem",
    },
    priority: {
        backgroundColor: "rgb(25 54 64 / 35%)",
        borderRadius: "50%",
        height: "0.3125rem",
        width: "0.3125rem",
    },
    urgent: {
        backgroundColor: "#c64a17",
    },
    due: {
        backgroundColor: tokens.signal,
        fontFamily: tokens.monoFont,
        fontSize: "0.56rem",
        fontWeight: 600,
        paddingInline: "0.2rem",
    },
});
