import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { plate } from "../style/plate.stylex";
import { tokens } from "../style/tokens.stylex";
import { Fade } from "./fade";
import type { Stagger } from "./stagger";
import { Tile } from "./tile";

/** The engine's services the week view runs on, numbered as its parts mark them: reading, writing back, repeating and changing. */
export const services: readonly string[] = ["Database", "Access", "Workflows", "Forge"];

/** One event on the week: its title, its time, the objects it links to, and whether the agent just moved it. */
type Event = readonly [title: string, time: string, links?: string | undefined, isMoved?: boolean];

/** The days of the week in both states: the date, the events on it while rented and once owned, and when Kai is free once owned. */
const days: readonly (readonly [
    day: string,
    stacked: readonly Event[],
    destacked: readonly Event[],
    kaiFree?: string,
])[] = [
    ["Mon 20", [["Standup", "10:00"]], [["Standup", "10:00"]], "15–17"],
    ["Tue 21", [["Pricing review", "14:00"]], [["Pricing review", "14:00"]], "10–12"],
    [
        "Wed 22",
        [
            ["Demo with Kai", "11:00", undefined, true],
            ["Lunch with Ada", "12:30"],
        ],
        [
            ["Demo with Kai", "11:00", "LCH-14 · 1 page", true],
            ["Lunch with Ada", "12:30"],
        ],
    ],
    ["Thu 23", [["Waitlist invites", "09:00"]], [["Waitlist invites", "09:00", "LCH-16"]], "13–16"],
    ["Fri 24", [["Launch", "10:00"]], [["Launch", "10:00"]], "14–15"],
];

/** The day that is today. */
const today = "Tue 21";

/** The launch tasks due this week: title and due date. */
const tasks: readonly (readonly [title: string, due: string])[] = [
    ["Record the demo", "Wed 22"],
    ["Draft the launch post", "Thu 23"],
    ["Email the waitlist", "Fri 24"],
];

/** What moving the demo touches in each app: the app, the change, the chore it leaves while rented, and the connector that makes it while rented, if any. */
const ripple: readonly (readonly [app: string, done: string, chore: string, connector?: string])[] =
    [
        ["calendar", "Demo → Wed 11:00", "Move demo", "calendar"],
        ["tasks", "LCH-14 → Wed 22", "Update LCH-14", "linear"],
        ["pages", "Launch plan → Wed", "Edit launch plan"],
        ["pages", "Replied to Kai", "Reply to Kai"],
    ];

/** The threads waiting on you: who, the app it came from, and what they asked. */
const waiting: readonly (readonly [person: "Kai" | "Ada", app: string, text: string])[] = [
    ["Kai", "pages", "Can we move the demo to Wednesday?"],
    ["Ada", "mail", "Is pricing-v4.pdf final?"],
];

/** The tint of each person's avatar. */
const tints = { Kai: "#5b7f2e", Ada: "#6b5ca5" } as const;

/** The strokes of each glyph the chat and the app draw, on a 24-unit grid. */
const glyphs = {
    copy: ["M8 8h11a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1z", "M4 16V4a1 1 0 0 1 1-1h11"],
    like: [
        "M7 10v11",
        "M15 5.9 14 10h5.8a2 2 0 0 1 1.9 2.6l-2.3 8a2 2 0 0 1-1.9 1.4H4a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h2.8a2 2 0 0 0 1.8-1.1L12 2a3.1 3.1 0 0 1 3 3.9Z",
    ],
    dislike: [
        "M17 14V3",
        "M9 18.1 10 14H4.2a2 2 0 0 1-1.9-2.6l2.3-8A2 2 0 0 1 6.5 2H20a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-2.8a2 2 0 0 0-1.8 1.1L12 22a3.1 3.1 0 0 1-3-3.9Z",
    ],
    share: ["M12 3v12", "m8 7 4-4 4 4", "M5 14v5a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-5"],
    plus: ["M12 5v14", "M5 12h14"],
    microphone: [
        "M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z",
        "M19 10v2a7 7 0 0 1-14 0v-2",
        "M12 19v3",
    ],
    up: ["M12 19V5", "m5 12 7-7 7 7"],
    next: ["m9 18 6-6-6-6"],
    previous: ["m15 18-6-6 6-6"],
    down: ["m6 9 6 6 6-6"],
    check: ["M20 6 9 17l-5-5"],
    open: ["M7 17 17 7", "M7 7h10v10"],
} as const;

/** Draw a glyph in the current colour. */
function Glyph(properties: { name: keyof typeof glyphs }) {
    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            {...stylex.attrs(styles.glyph)}
        >
            {glyphs[properties.name].map((path) => (
                <path d={path} />
            ))}
        </svg>
    );
}

/** Draw a message of yours as a rounded pill on the right. */
function UserMessage(properties: { children: JSX.Element }) {
    return (
        <p data-component="UserMessage" {...stylex.attrs(styles.bubble)}>
            {properties.children}
        </p>
    );
}

/** Draw a reply as the chat sets it: how long the assistant worked, its text, and the actions under it. */
function Reply(properties: { time: string; children: JSX.Element }) {
    return (
        <div data-component="AssistantMessage" {...stylex.attrs(styles.reply)}>
            <p data-component="WorkSummary" {...stylex.attrs(styles.worked)}>
                Worked for {properties.time}
                <Glyph name="next" />
            </p>
            <div {...stylex.attrs(styles.replyText)}>{properties.children}</div>
            <p data-component="MessageActions" {...stylex.attrs(styles.actions)}>
                <Glyph name="copy" />
                <Glyph name="like" />
                <Glyph name="dislike" />
                <Glyph name="share" />
            </p>
        </div>
    );
}

/** Draw a person's initial on their round avatar. */
function Avatar(properties: { person: "Kai" | "Ada" }) {
    return (
        <span
            data-component="Avatar"
            style={{ "background-color": tints[properties.person] }}
            {...stylex.attrs(styles.avatar)}
        >
            {properties.person.charAt(0)}
        </span>
    );
}

/** Draw a connector the chat reads an app through. */
function Connector(properties: { icon: string; name: string }) {
    return (
        <span data-component="Connector" {...stylex.attrs(styles.connector)}>
            <span
                style={{ "mask-image": `url(/diagram/${properties.icon}.svg)` }}
                {...stylex.attrs(styles.mask)}
            />
            {properties.name}
        </span>
    );
}

/**
 * Draw a week view made by asking in one chat, with the conversation on the left and the view on the right.
 *
 * While rented, the view is a chat artifact: a snapshot of what the chat's connectors return, changed through each connector, with its schedule in the chat.
 * Once owned, the view is an app in your space: one live query over the objects every app shares, changing them all at once, with its schedule beside your apps.
 */
export function WeekApp(properties: { isOpenAt: Stagger }) {
    return (
        <div {...stylex.attrs(styles.app)}>
            <Fade
                isOpen={properties.isOpenAt(0)}
                style={styles.conversation}
                component={properties.isOpenAt(0) ? "AgentChat" : "Chat"}
            >
                <div {...stylex.attrs(styles.thread)}>
                    <p data-component="Timestamp" {...stylex.attrs(styles.date)}>
                        Mon 09:12
                    </p>
                    <UserMessage>
                        Make me a view of my week: meetings, launch tasks, and anything waiting on
                        me.
                    </UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply time="31s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Built <b {...stylex.attrs(styles.strong)}>My week</b> in your space.
                                It reads <code {...stylex.attrs(styles.code)}>Event</code>,{" "}
                                <code {...stylex.attrs(styles.code)}>Task</code> and{" "}
                                <code {...stylex.attrs(styles.code)}>Thread</code> from your apps,
                                so it stays current.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="52s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Here's <b {...stylex.attrs(styles.strong)}>My week</b>, built from
                                your Google Calendar, Linear and Gmail connectors as of 09:14. Ask
                                me to refresh it.
                            </p>
                        </Reply>
                    )}
                    <p data-component="Timestamp" {...stylex.attrs(styles.date)}>
                        Today 16:32
                    </p>
                    <UserMessage>
                        Kai wants the demo on Wednesday. Move it, keep everything in sync, and send
                        me a summary on Fridays.
                    </UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply time="48s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Moved <b {...stylex.attrs(styles.strong)}>Demo with Kai</b> to Wed
                                11:00. That also moved{" "}
                                <code {...stylex.attrs(styles.code)}>LCH-14</code>, the launch
                                plan's date and your reply to Kai. Your summary arrives Fridays at
                                17:00.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="1m 04s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Moved the event in Google Calendar and{" "}
                                <code {...stylex.attrs(styles.code)}>LCH-14</code> in Linear. Pages
                                has no connector, so edit the launch plan and reply to Kai there.
                            </p>
                            <p data-service="3" {...stylex.attrs(styles.paragraph)}>
                                Your summary is scheduled here for Fridays at 17:00.
                            </p>
                        </Reply>
                    )}
                    <UserMessage>
                        Also show Kai's free slots next to mine, so I can pick times myself.
                    </UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply time="1m 12s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Kai shares free and busy times with you, so I added his free slots
                                to each day on{" "}
                                <code {...stylex.attrs(styles.code)}>kai-free-slots</code>. Preview
                                it, then merge.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="1m 31s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Kai's calendar isn't one of your connectors. Paste his free times
                                and I'll add them. This is{" "}
                                <b {...stylex.attrs(styles.strong)}>version 4</b>, so the layout is
                                rebuilt.
                            </p>
                        </Reply>
                    )}
                </div>
                <div data-component="Composer" {...stylex.attrs(styles.composer)}>
                    <span {...stylex.attrs(styles.quiet)}>
                        {properties.isOpenAt(0) ? "Ask your agent" : "Ask anything"}
                    </span>
                    <span {...stylex.attrs(styles.controls)}>
                        <span {...stylex.attrs(styles.iconButton)}>
                            <Glyph name="plus" />
                        </span>
                        {properties.isOpenAt(0) ? (
                            <span data-component="AgentScope" {...stylex.attrs(styles.chip)}>
                                <Tile name="calendar" />
                                <Tile name="tasks" />
                                <Tile name="mail" />3 apps
                            </span>
                        ) : undefined}
                        <span
                            data-component="ModelPicker"
                            {...stylex.attrs(styles.picker, styles.pushed)}
                        >
                            Thinking
                            <Glyph name="down" />
                        </span>
                        <span {...stylex.attrs(styles.iconButton)}>
                            <Glyph name="microphone" />
                        </span>
                        <span {...stylex.attrs(styles.send)}>
                            <Glyph name="up" />
                        </span>
                    </span>
                </div>
            </Fade>
            <div
                data-component={properties.isOpenAt(5) ? "SpaceApp" : "Artifact"}
                {...stylex.attrs(styles.pane)}
            >
                <Fade isOpen={properties.isOpenAt(4)} style={styles.bar} component="ArtifactHeader">
                    <span data-component="ArtifactTitle" {...stylex.attrs(styles.title)}>
                        My week
                    </span>
                    {properties.isOpenAt(4) ? (
                        <span data-service="4" {...stylex.attrs(plate.plate)}>
                            kai-free-slots
                        </span>
                    ) : (
                        <span
                            data-component="VersionPicker"
                            data-service="4"
                            {...stylex.attrs(styles.picker)}
                        >
                            v4
                            <Glyph name="down" />
                        </span>
                    )}
                    <span
                        data-component={properties.isOpenAt(4) ? "ObjectList" : "ConnectorCount"}
                        data-service="1"
                        {...stylex.attrs(styles.sourcesNote)}
                    >
                        {properties.isOpenAt(4) ? (
                            <>
                                Reads <code {...stylex.attrs(styles.code)}>Event</code>,{" "}
                                <code {...stylex.attrs(styles.code)}>Task</code> and{" "}
                                <code {...stylex.attrs(styles.code)}>Thread</code>
                            </>
                        ) : (
                            "From 3 connectors"
                        )}
                    </span>
                    <span {...stylex.attrs(styles.end)}>
                        {properties.isOpenAt(4) ? (
                            <span
                                data-component="BranchControls"
                                data-service="4"
                                {...stylex.attrs(styles.pair)}
                            >
                                <span
                                    data-component="EditAppButton"
                                    {...stylex.attrs(styles.button, styles.wide)}
                                >
                                    Edit app
                                </span>
                                <span {...stylex.attrs(styles.button, styles.wide)}>Preview</span>
                                <span {...stylex.attrs(styles.button, styles.primary)}>Merge</span>
                            </span>
                        ) : (
                            <>
                                <span
                                    data-component="ArtifactTabs"
                                    {...stylex.attrs(styles.tabs, styles.wide)}
                                >
                                    <span {...stylex.attrs(styles.tab, styles.tabOn)}>Preview</span>
                                    <span {...stylex.attrs(styles.tab)}>Code</span>
                                </span>
                                <span {...stylex.attrs(styles.iconButton, styles.wide)}>
                                    <Glyph name="copy" />
                                </span>
                                <span
                                    data-component="PublishButton"
                                    {...stylex.attrs(styles.button, styles.dark)}
                                >
                                    Publish
                                </span>
                            </>
                        )}
                    </span>
                </Fade>
                <div data-component="WeekView" {...stylex.attrs(styles.body)}>
                    <div data-component="Toolbar" {...stylex.attrs(styles.toolbar)}>
                        <span data-component="WeekPicker" {...stylex.attrs(styles.week)}>
                            <Glyph name="previous" />
                            20 – 24 October
                            <Glyph name="next" />
                        </span>
                        <Fade
                            isOpen={properties.isOpenAt(1)}
                            style={styles.sources}
                            component={properties.isOpenAt(1) ? "SourceChip" : "ConnectorBar"}
                        >
                            {properties.isOpenAt(1) ? (
                                <span data-service="1" {...stylex.attrs(styles.live)}>
                                    <span {...stylex.attrs(styles.dot)} />
                                    Calendar, Tasks, Mail · live
                                </span>
                            ) : (
                                <span data-service="1" {...stylex.attrs(styles.connectors)}>
                                    <Connector icon="calendar" name="Google Calendar" />
                                    <Connector icon="linear" name="Linear" />
                                    <Connector icon="mail" name="Gmail" />
                                    <span {...stylex.attrs(styles.snapshot)}>as of 16:40</span>
                                </span>
                            )}
                        </Fade>
                    </div>
                    <ol data-component="WeekStrip" data-service="1" {...stylex.attrs(styles.days)}>
                        {days.map(([day, stacked, destacked, kaiFree], index) => (
                            <li
                                data-component="Day"
                                {...stylex.attrs(
                                    styles.day,
                                    day === today && styles.today,
                                    (index === 0 || index === days.length - 1) && styles.wide,
                                )}
                            >
                                <span
                                    {...stylex.attrs(
                                        styles.dayName,
                                        day === today && styles.todayName,
                                    )}
                                >
                                    {day}
                                </span>
                                <Fade isOpen={properties.isOpenAt(1)} style={styles.events}>
                                    {(properties.isOpenAt(1) ? destacked : stacked).map(
                                        ([title, time, links, isMoved]) => (
                                            <span
                                                data-component="CalendarEvent"
                                                {...stylex.attrs(
                                                    styles.event,
                                                    isMoved === true && styles.moved,
                                                )}
                                            >
                                                <b {...stylex.attrs(styles.eventTitle)}>{title}</b>
                                                <span {...stylex.attrs(styles.eventTime)}>
                                                    {isMoved === true ? `${time} · moved` : time}
                                                </span>
                                                {links === undefined ? undefined : (
                                                    <span
                                                        data-component="LinkedObjects"
                                                        {...stylex.attrs(styles.links)}
                                                    >
                                                        {links}
                                                    </span>
                                                )}
                                            </span>
                                        ),
                                    )}
                                    {properties.isOpenAt(1) && kaiFree !== undefined ? (
                                        <span
                                            data-component="FreeSlot"
                                            data-service="4"
                                            {...stylex.attrs(styles.free)}
                                        >
                                            <b {...stylex.attrs(styles.eventTitle)}>Kai free</b>
                                            <span {...stylex.attrs(styles.eventTime)}>
                                                {kaiFree}
                                            </span>
                                        </span>
                                    ) : undefined}
                                </Fade>
                            </li>
                        ))}
                    </ol>
                    <div {...stylex.attrs(styles.columns)}>
                        <section data-component="DueList" {...stylex.attrs(styles.panel)}>
                            <p {...stylex.attrs(styles.panelHead)}>
                                Due this week
                                <span {...stylex.attrs(styles.count)}>3</span>
                            </p>
                            <Fade isOpen={properties.isOpenAt(2)} style={styles.stack}>
                                {tasks.map(([title, due]) => (
                                    <p
                                        data-component="TaskRow"
                                        data-service="2"
                                        {...stylex.attrs(styles.row)}
                                    >
                                        <span
                                            data-component="Checkbox"
                                            {...stylex.attrs(
                                                styles.box,
                                                !properties.isOpenAt(2) && styles.boxLocked,
                                            )}
                                        />
                                        <span {...stylex.attrs(styles.rowTitle)}>{title}</span>
                                        <span {...stylex.attrs(styles.quiet)}>{due}</span>
                                    </p>
                                ))}
                            </Fade>
                            <Fade
                                isOpen={properties.isOpenAt(3)}
                                component={properties.isOpenAt(3) ? "SummaryCard" : "SummaryNote"}
                            >
                                {properties.isOpenAt(3) ? (
                                    <p
                                        data-service="3"
                                        {...stylex.attrs(styles.row, styles.summary)}
                                    >
                                        <Tile name="mail" />
                                        <span {...stylex.attrs(styles.rowTitle)}>
                                            Friday summary
                                        </span>
                                        <span {...stylex.attrs(styles.quiet)}>Fri 17:00</span>
                                    </p>
                                ) : (
                                    <p
                                        data-service="3"
                                        {...stylex.attrs(styles.row, styles.summary)}
                                    >
                                        <span
                                            style={{ "mask-image": "url(/diagram/chat.svg)" }}
                                            {...stylex.attrs(styles.mask, styles.quiet)}
                                        />
                                        <span {...stylex.attrs(styles.rowTitle)}>
                                            Friday summary
                                        </span>
                                        <span {...stylex.attrs(styles.quiet)}>in the chat</span>
                                    </p>
                                )}
                            </Fade>
                        </section>
                        <section
                            data-component="WaitingList"
                            {...stylex.attrs(styles.panel, styles.wide)}
                        >
                            <p {...stylex.attrs(styles.panelHead)}>
                                Waiting on you
                                <span {...stylex.attrs(styles.count)}>
                                    {properties.isOpenAt(2) ? "2" : "1"}
                                </span>
                            </p>
                            <Fade
                                isOpen={properties.isOpenAt(2)}
                                style={styles.threads}
                                component={properties.isOpenAt(2) ? "Threads" : "ConnectedThreads"}
                            >
                                {waiting
                                    .filter(([, app]) => properties.isOpenAt(2) || app === "mail")
                                    .map(([person, app, text]) => (
                                        <p
                                            data-component="ThreadRow"
                                            data-service="2"
                                            {...stylex.attrs(styles.row)}
                                        >
                                            <Avatar person={person} />
                                            <b {...stylex.attrs(styles.strong)}>{person}</b>
                                            <span {...stylex.attrs(styles.quiet, styles.clip)}>
                                                {text}
                                            </span>
                                            <span {...stylex.attrs(styles.source)}>
                                                <Tile name={app} />
                                            </span>
                                        </p>
                                    ))}
                                {properties.isOpenAt(2) ? undefined : (
                                    <p
                                        data-service="1"
                                        {...stylex.attrs(styles.row, styles.absent)}
                                    >
                                        Pages has no connector
                                    </p>
                                )}
                            </Fade>
                        </section>
                        <section data-component="RippleList" {...stylex.attrs(styles.panel)}>
                            <p {...stylex.attrs(styles.panelHead)}>
                                Moving the demo
                                <span
                                    {...stylex.attrs(
                                        styles.count,
                                        properties.isOpenAt(2)
                                            ? styles.countDone
                                            : styles.countLeft,
                                    )}
                                >
                                    {properties.isOpenAt(2) ? "1 change" : "2 by hand"}
                                </span>
                            </p>
                            <Fade isOpen={properties.isOpenAt(2)} style={styles.stack}>
                                {ripple.map(([app, done, chore, connector]) => (
                                    <p
                                        data-component={
                                            properties.isOpenAt(2)
                                                ? "AppliedChange"
                                                : connector === undefined
                                                  ? "Chore"
                                                  : "ConnectorChange"
                                        }
                                        data-service="2"
                                        {...stylex.attrs(styles.row)}
                                    >
                                        {properties.isOpenAt(2) ? (
                                            <>
                                                <Tile name={app} />
                                                <span {...stylex.attrs(styles.rowTitle)}>
                                                    {done}
                                                </span>
                                                <span {...stylex.attrs(styles.passed)}>
                                                    <Glyph name="check" />
                                                </span>
                                            </>
                                        ) : connector === undefined ? (
                                            <>
                                                <span
                                                    {...stylex.attrs(styles.box, styles.boxLocked)}
                                                />
                                                <span {...stylex.attrs(styles.rowTitle)}>
                                                    {chore}
                                                </span>
                                                <span {...stylex.attrs(styles.opens)}>
                                                    Pages
                                                    <Glyph name="open" />
                                                </span>
                                            </>
                                        ) : (
                                            <>
                                                <span
                                                    style={{
                                                        "mask-image": `url(/diagram/${connector}.svg)`,
                                                    }}
                                                    {...stylex.attrs(styles.mask, styles.quiet)}
                                                />
                                                <span {...stylex.attrs(styles.rowTitle)}>
                                                    {done}
                                                </span>
                                                <span {...stylex.attrs(styles.quiet)}>
                                                    <Glyph name="check" />
                                                </span>
                                            </>
                                        )}
                                    </p>
                                ))}
                            </Fade>
                        </section>
                    </div>
                </div>
            </div>
        </div>
    );
}

/** The week app styles. */
const styles = stylex.create({
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) minmax(0, 1.75fr)",
            "@media (max-width: 767px)": "minmax(0, 1fr)",
        },
        height: "100%",
        minHeight: 0,
    },
    glyph: {
        flexShrink: 0,
        height: "0.9375rem",
        width: "0.9375rem",
    },
    mask: {
        backgroundColor: "currentColor",
        flexShrink: 0,
        height: "0.8125rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.8125rem",
    },
    conversation: {
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr) auto",
        minHeight: 0,
        padding: "0.75rem 1.25rem 1rem",
    },
    thread: {
        alignContent: "end",
        display: "grid",
        gap: "1.25rem",
        maskImage: "linear-gradient(to bottom, transparent, #000 2rem)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
    },
    date: {
        color: color.mutedForeground,
        fontSize: "0.6875rem",
        margin: 0,
        textAlign: "center",
    },
    bubble: {
        backgroundColor: color.muted,
        borderRadius: "1.125rem",
        justifySelf: "end",
        lineHeight: 1.45,
        margin: 0,
        maxWidth: "80%",
        paddingBlock: "0.5rem",
        paddingInline: "0.875rem",
    },
    reply: {
        display: "grid",
        gap: "0.375rem",
    },
    worked: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: color.mutedForeground,
        display: "flex",
        gap: "0.25rem",
        margin: 0,
        paddingBottom: "0.375rem",
    },
    replyText: {
        display: "grid",
        gap: "0.5rem",
        lineHeight: 1.6,
    },
    paragraph: {
        margin: 0,
    },
    code: {
        backgroundColor: color.muted,
        borderRadius: "4px",
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        paddingInline: "0.25rem",
    },
    actions: {
        color: color.mutedForeground,
        display: "flex",
        gap: "0.75rem",
        margin: 0,
    },
    composer: {
        borderColor: tokens.rule,
        borderRadius: "1.25rem",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxShadow: "0 2px 10px rgb(0 0 0 / 6%)",
        display: "grid",
        gap: "0.625rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        marginTop: "1.25rem",
        padding: "0.75rem 0.5rem 0.5rem 1rem",
    },
    controls: {
        alignItems: "center",
        display: "flex",
        gap: "0.375rem",
        marginLeft: "-0.5rem",
        minWidth: 0,
    },
    iconButton: {
        alignItems: "center",
        borderRadius: "50%",
        color: color.mutedForeground,
        display: "inline-flex",
        flexShrink: 0,
        height: "1.75rem",
        justifyContent: "center",
        width: "1.75rem",
    },
    chip: {
        alignItems: "center",
        borderColor: tokens.rule,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "inline-flex",
        fontSize: "0.72rem",
        gap: "0.25rem",
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "0.1875rem",
        paddingInline: "0.25rem 0.625rem",
        whiteSpace: "nowrap",
    },
    picker: {
        alignItems: "center",
        borderRadius: "6px",
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.125rem",
        paddingInline: "0.375rem",
        whiteSpace: "nowrap",
    },
    pushed: {
        marginLeft: "auto",
    },
    send: {
        alignItems: "center",
        backgroundColor: color.foreground,
        borderRadius: "50%",
        color: color.background,
        display: "flex",
        flexShrink: 0,
        height: "1.875rem",
        justifyContent: "center",
        width: "1.875rem",
    },
    pane: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        minWidth: 0,
    },
    bar: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        gap: "0.625rem",
        height: "2.75rem",
        paddingInline: "1rem 0.75rem",
    },
    title: {
        fontWeight: 600,
        whiteSpace: "nowrap",
    },
    tabs: {
        backgroundColor: color.muted,
        borderRadius: "7px",
        display: "flex",
        padding: "0.125rem",
    },
    tab: {
        borderRadius: "5px",
        color: color.mutedForeground,
        fontSize: "0.75rem",
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
    },
    tabOn: {
        backgroundColor: color.card,
        boxShadow: "0 1px 2px rgb(0 0 0 / 10%)",
        color: color.foreground,
        fontWeight: 600,
    },
    end: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        marginLeft: "auto",
    },
    avatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: "#ffffff",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.375rem",
        justifyContent: "center",
        width: "1.375rem",
    },
    pair: {
        display: "flex",
        gap: "0.375rem",
    },
    button: {
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.foreground,
        flexShrink: 0,
        fontSize: "0.75rem",
        fontWeight: 600,
        paddingBlock: "0.1875rem",
        paddingInline: "0.625rem",
        whiteSpace: "nowrap",
    },
    primary: {
        backgroundColor: tokens.signal,
        borderColor: tokens.signal,
        color: tokens.signalInk,
    },
    dark: {
        backgroundColor: color.foreground,
        borderColor: color.foreground,
        color: color.background,
    },
    body: {
        alignContent: "start",
        display: "grid",
        gap: "1.25rem",
        gridTemplateRows: "auto auto minmax(10rem, 1fr)",
        maskImage: "linear-gradient(to bottom, #000 calc(100% - 1.5rem), transparent)",
        minHeight: 0,
        overflow: "hidden",
        padding: { default: "1.25rem 1.5rem", "@media (max-width: 767px)": "1rem" },
    },
    toolbar: {
        alignItems: "center",
        display: "flex",
        gap: "1rem",
    },
    week: {
        alignItems: "center",
        display: "flex",
        flexShrink: 0,
        fontSize: "0.9375rem",
        fontWeight: 700,
        gap: "0.375rem",
    },
    sources: {
        alignItems: "center",
        display: { default: "flex", "@media (max-width: 767px)": "none" },
        height: "1.375rem",
        marginLeft: "auto",
        minWidth: 0,
    },
    live: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        gap: "0.375rem",
        whiteSpace: "nowrap",
    },
    dot: {
        backgroundColor: "#4caf6e",
        borderRadius: "50%",
        height: "0.5rem",
        width: "0.5rem",
    },
    connectors: {
        display: "flex",
        gap: "0.375rem",
    },
    connector: {
        alignItems: "center",
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "inline-flex",
        fontSize: "0.72rem",
        gap: "0.3125rem",
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
        whiteSpace: "nowrap",
    },
    snapshot: {
        color: color.mutedForeground,
        fontSize: "0.72rem",
        whiteSpace: "nowrap",
    },
    days: {
        borderColor: tokens.rule,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
        gridTemplateColumns: {
            default: "repeat(5, minmax(0, 1fr))",
            "@media (max-width: 767px)": "repeat(3, minmax(0, 1fr))",
        },
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        padding: 0,
    },
    day: {
        alignContent: "start",
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: { default: tokens.hairline, ":last-child": 0 },
        display: "grid",
        minHeight: "8.9375rem",
        minWidth: 0,
    },
    today: {
        backgroundColor: "rgb(255 121 46 / 4%)",
    },
    dayName: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: color.mutedForeground,
        display: "flex",
        fontSize: "0.72rem",
        fontWeight: 600,
        height: "2.125rem",
        paddingInline: "0.625rem",
    },
    todayName: {
        color: tokens.signal,
    },
    events: {
        alignContent: "start",
        display: "grid",
        gap: "0.375rem",
        padding: "0.5rem",
    },
    event: {
        backgroundColor: color.muted,
        borderLeftColor: "#3d6fb0",
        borderLeftStyle: "solid",
        borderLeftWidth: "3px",
        borderRadius: "4px",
        display: "grid",
        minWidth: 0,
        paddingBlock: "0.25rem",
        paddingInline: "0.375rem",
    },
    moved: {
        backgroundColor: "rgb(255 121 46 / 12%)",
        borderLeftColor: tokens.signal,
    },
    eventTitle: {
        fontSize: "0.75rem",
        fontWeight: 600,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    eventTime: {
        color: color.mutedForeground,
        fontSize: "0.68rem",
        whiteSpace: "nowrap",
    },
    columns: {
        alignItems: "stretch",
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: {
            default: "minmax(0, 1.2fr) minmax(0, 1fr) minmax(0, 1fr)",
            "@media (max-width: 767px)": "minmax(0, 1fr)",
        },
    },
    links: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.625rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    summary: {
        borderTopColor: tokens.rule,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
    },
    absent: {
        color: color.mutedForeground,
        fontWeight: 500,
    },
    source: {
        display: "flex",
        flexShrink: 0,
        marginLeft: "auto",
    },
    passed: {
        color: "#3c8f58",
        display: "flex",
    },
    opens: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        fontSize: "0.72rem",
        gap: "0.125rem",
    },
    wide: {
        "@media (max-width: 767px)": { display: "none" },
    },
    sourcesNote: {
        color: color.mutedForeground,
        fontSize: "0.75rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        "@media (max-width: 1279px)": { display: "none" },
    },
    panel: {
        alignContent: "start",
        borderColor: tokens.rule,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
        minWidth: 0,
        overflow: "hidden",
    },
    panelHead: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        fontSize: "0.75rem",
        fontWeight: 600,
        gap: "0.5rem",
        height: "2.25rem",
        margin: 0,
        paddingInline: "0.75rem",
        whiteSpace: "nowrap",
    },
    count: {
        color: color.mutedForeground,
        fontWeight: 500,
        marginLeft: "auto",
    },
    countDone: {
        color: "#3c8f58",
        fontWeight: 600,
    },
    countLeft: {
        color: "#b03a2e",
        fontWeight: 600,
    },
    stack: {
        display: "grid",
    },
    threads: {
        display: "grid",
    },
    free: {
        backgroundImage:
            "repeating-linear-gradient(135deg, rgb(60 143 88 / 12%) 0 4px, transparent 4px 8px)",
        borderLeftColor: "#3c8f58",
        borderLeftStyle: "solid",
        borderLeftWidth: "3px",
        borderRadius: "4px",
        color: "#3c8f58",
        display: "grid",
        minWidth: 0,
        paddingBlock: "0.25rem",
        paddingInline: "0.375rem",
    },
    row: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: tokens.hairline, ":last-child": 0 },
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        minHeight: "2.375rem",
        minWidth: 0,
        paddingBlock: "0.3125rem",
        paddingInline: "0.75rem",
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
    boxLocked: {
        borderStyle: "dashed",
        opacity: 0.5,
    },
    rowTitle: {
        flexGrow: 1,
        fontWeight: 600,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    clip: {
        flexGrow: 1,
        flexShrink: 1,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    strong: {
        fontWeight: 600,
    },
    quiet: {
        color: color.mutedForeground,
        flexShrink: 0,
    },
});
