import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { plate } from "../style/plate.stylex";
import { tokens } from "../style/tokens.stylex";
import { Fade } from "./fade";
import { Glyph } from "./glyph";
import type { Stagger } from "./stagger";
import { Tile } from "./tile";

/** The engine's services the week view runs on, numbered as the view marks them: reading, writing back, repeating and changing. */
export const services: readonly string[] = ["Database", "Access", "Workflows", "Forge"];

/** One event on the week: its title, its time, the objects it links to, and whether the agent just moved it. */
type Event = readonly [title: string, time: string, links?: string | undefined, isMoved?: boolean];

/** The days of the week in both states: the date, the events on it while rented and once owned, and when Friend is free once owned. */
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
            ["Demo with a friend", "11:00", undefined, true],
            ["Lunch", "12:30"],
        ],
        [
            ["Demo with a friend", "11:00", "LCH-14 · 1 page", true],
            ["Lunch", "12:30"],
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

/** What moving the demo touches: the change, the chore it leaves while rented, and the connector that makes it while rented, if any. */
const ripple: readonly (readonly [done: string, chore: string, connector?: string])[] = [
    ["Demo → Wed 11:00", "Move demo", "calendar"],
    ["LCH-14 → Wed 22", "Update LCH-14", "linear"],
    ["Launch plan → Wed", "Edit launch plan"],
    ["Replied to your friend", "Reply to your friend"],
];

/** The threads waiting on you: who, the app it came from, and what they asked. */
const waiting: readonly (readonly [person: "Friend" | "Me", app: string, text: string])[] = [
    ["Friend", "pages", "Can we move the demo to Wednesday?"],
    ["Friend", "mail", "Is pricing-v4.pdf final?"],
];

/** The tint of each person's avatar. */
const tints = { Friend: "#5b7f2e", Me: "#2f7d8c" };

/** Draw what a rented part of the artifact fails to do, marked the same way everywhere. */
function Issue(properties: { children: string }) {
    return (
        <span data-component="Issue" {...stylex.attrs(styles.issue)}>
            {properties.children}
        </span>
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
                <Glyph size={15} name="forward" />
            </p>
            <div {...stylex.attrs(styles.replyText)}>{properties.children}</div>
            <p data-component="MessageActions" {...stylex.attrs(styles.actions)}>
                <Glyph size={15} name="copy" />
                <Glyph size={15} name="like" />
                <Glyph size={15} name="dislike" />
                <Glyph size={15} name="share" />
            </p>
        </div>
    );
}

/** Say what you can do with a thread: open it in the vendor's app while rented, or answer it here once destacked. */
function actionOf(app: string, isOpen: boolean) {
    if (!isOpen) {
        return "Open in Gmail ↗";
    } else if (app === "pages") {
        return "✓ Moved to Wed";
    }

    return "Reply here";
}

/** Draw a person's initial on their round avatar. */
function Avatar(properties: { person: "Friend" | "Me" }) {
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

/**
 * Draw a week view made by asking in one chat, with the conversation on the left and the view on the right.
 *
 * While rented, the view is a chat artifact: a snapshot of what the chat's connectors return, changed through each connector, with its schedule in the chat.
 * Once owned, the same chat builds an app in your space through Destack: one live query over the objects every app shares, changing them all at once, with its schedule beside your apps.
 */
export function WeekApp(properties: { isOpenAt: Stagger }) {
    return (
        <div {...stylex.attrs(styles.app)}>
            <Fade
                isOpen={properties.isOpenAt(0)}
                style={styles.conversation}
                isLarge
                component={properties.isOpenAt(0) ? "ConnectedChat" : "Chat"}
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
                                Built <b {...stylex.attrs(styles.strong)}>My week</b> in your
                                Destack space. It reads{" "}
                                <code {...stylex.attrs(styles.code)}>Event</code>,{" "}
                                <code {...stylex.attrs(styles.code)}>Task</code> and{" "}
                                <code {...stylex.attrs(styles.code)}>Thread</code> from your apps,
                                so it stays current.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="52s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Here's <b {...stylex.attrs(styles.strong)}>My week</b> as an
                                artifact. It's a snapshot from 09:14, so ask me to regenerate it
                                when things change.
                            </p>
                        </Reply>
                    )}
                    <p data-component="Timestamp" {...stylex.attrs(styles.date)}>
                        Today 16:32
                    </p>
                    <UserMessage>
                        Your friend wants the demo on Wednesday. Move it, keep everything in sync,
                        and send me a summary on Fridays.
                    </UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply time="48s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Moved <b {...stylex.attrs(styles.strong)}>Demo with a friend</b> to
                                Wed 11:00. That also moved{" "}
                                <code {...stylex.attrs(styles.code)}>LCH-14</code>, the launch
                                plan's date and your reply to your friend. Your summary arrives
                                Fridays at 17:00.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="1m 04s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Moved the demo. I regenerated the artifact to show it, so your
                                checked-off tasks and notes from Monday are gone.
                            </p>
                            <p data-service="3" {...stylex.attrs(styles.paragraph)}>
                                Your summary is scheduled here for Fridays at 17:00.
                            </p>
                        </Reply>
                    )}
                    <UserMessage>
                        Also show your friend's free slots next to mine, so I can pick times myself.
                    </UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply time="1m 12s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                Your friend shares their free and busy times with you in Destack, so
                                I added their free slots on{" "}
                                <code {...stylex.attrs(styles.code)}>friend-free-slots</code>.
                                Preview it, then merge.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="1m 31s">
                            <p {...stylex.attrs(styles.paragraph)}>
                                I can't edit this artifact in place, only rebuild it. Here's{" "}
                                <b {...stylex.attrs(styles.strong)}>version 4</b>, with a new
                                layout. Paste your friend's free times and I'll rebuild it again.
                            </p>
                        </Reply>
                    )}
                </div>
                <div data-component="Composer" {...stylex.attrs(styles.composer)}>
                    <span {...stylex.attrs(styles.quiet)}>Ask anything</span>
                    <span {...stylex.attrs(styles.controls)}>
                        <span {...stylex.attrs(styles.iconButton)}>
                            <Glyph size={15} name="plus" />
                        </span>
                        {properties.isOpenAt(0) ? (
                            <span data-component="AgentScope" {...stylex.attrs(styles.chip)}>
                                <Tile name="calendar" />
                                <Tile name="tasks" />
                                <Tile name="mail" />
                                Your space · live
                            </span>
                        ) : (
                            <span data-component="ConnectorScope" {...stylex.attrs(styles.chip)}>
                                3 connectors · as of 09:14
                            </span>
                        )}
                        <span
                            data-component="ModelPicker"
                            {...stylex.attrs(styles.picker, styles.pushed)}
                        >
                            Thinking
                            <Glyph size={15} name="caret" />
                        </span>
                        <span {...stylex.attrs(styles.iconButton)}>
                            <Glyph size={15} name="microphone" />
                        </span>
                        <span {...stylex.attrs(styles.send)}>
                            <Glyph size={15} name="up" />
                        </span>
                    </span>
                </div>
            </Fade>
            <div
                data-component={properties.isOpenAt(5) ? "SpaceApp" : "Artifact"}
                data-artifact={properties.isOpenAt(0) ? undefined : ""}
                {...stylex.attrs(styles.pane)}
            >
                <Fade isOpen={properties.isOpenAt(4)} style={styles.bar} component="ArtifactHeader">
                    <span data-component="ArtifactTitle" {...stylex.attrs(styles.title)}>
                        My week
                    </span>
                    {properties.isOpenAt(4) ? (
                        <span data-service="4" {...stylex.attrs(plate.plate)}>
                            friend-free-slots
                        </span>
                    ) : (
                        <span
                            data-component="VersionPicker"
                            data-service="4"
                            {...stylex.attrs(styles.picker)}
                        >
                            v4
                            <Glyph size={15} name="caret" />
                        </span>
                    )}
                    {properties.isOpenAt(4) ? undefined : (
                        <span
                            data-component="ConnectorCount"
                            data-service="1"
                            {...stylex.attrs(styles.sourcesNote)}
                        >
                            Frozen at v4
                        </span>
                    )}
                    <span {...stylex.attrs(styles.end)}>
                        {properties.isOpenAt(4) ? (
                            <span
                                data-component="BranchControls"
                                data-service="4"
                                {...stylex.attrs(styles.pair)}
                            >
                                <span {...stylex.attrs(styles.button, styles.wide)}>Discard</span>
                                <span {...stylex.attrs(styles.button, styles.dark)}>Merge</span>
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
                                    <Glyph size={15} name="copy" />
                                </span>
                                <span
                                    data-component="RegenerateButton"
                                    {...stylex.attrs(styles.button, styles.dark)}
                                >
                                    ↻ Regenerate
                                </span>
                            </>
                        )}
                    </span>
                </Fade>
                <div data-component="WeekView" {...stylex.attrs(styles.body)}>
                    <div data-component="Toolbar" {...stylex.attrs(styles.toolbar)}>
                        <span data-component="WeekPicker" {...stylex.attrs(styles.week)}>
                            <Glyph size={15} name="back" />
                            20 – 24 October
                            <Glyph size={15} name="forward" />
                        </span>
                        <Fade
                            isOpen={properties.isOpenAt(1)}
                            style={styles.sources}
                            component={properties.isOpenAt(1) ? "SourceChip" : "ConnectorBar"}
                        >
                            {properties.isOpenAt(1) ? (
                                <span data-service="1" {...stylex.attrs(styles.live)}>
                                    <span {...stylex.attrs(styles.dot)} />
                                    Live · built from your space's standard parts
                                </span>
                            ) : (
                                <span data-service="1" {...stylex.attrs(styles.connectors)}>
                                    <Issue>No access to your friend's calendar</Issue>
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
                                <Fade isOpen={properties.isOpenAt(1)} style={styles.events} isLarge>
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
                                            <b {...stylex.attrs(styles.eventTitle)}>Friend free</b>
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
                                {properties.isOpenAt(2) ? (
                                    <span {...stylex.attrs(styles.count)}>3</span>
                                ) : (
                                    <Issue>Checks reset</Issue>
                                )}
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
                                {properties.isOpenAt(2) ? (
                                    <span {...stylex.attrs(styles.count)}>2</span>
                                ) : (
                                    <Issue>Gmail only</Issue>
                                )}
                            </p>
                            <Fade
                                isOpen={properties.isOpenAt(2)}
                                style={styles.threads}
                                isLarge
                                component={properties.isOpenAt(2) ? "Threads" : "ConnectedThreads"}
                            >
                                {waiting
                                    .filter(([, app]) => properties.isOpenAt(2) || app === "mail")
                                    .map(([person, app, text]) => (
                                        <div
                                            data-component="ThreadRow"
                                            data-service="2"
                                            {...stylex.attrs(styles.waitingRow)}
                                        >
                                            <span {...stylex.attrs(styles.threadHead)}>
                                                <Avatar person={person} />
                                                <b {...stylex.attrs(styles.strong)}>{person}</b>
                                                <Tile name={app} />
                                                <span {...stylex.attrs(styles.threadAction)}>
                                                    {actionOf(app, properties.isOpenAt(2))}
                                                </span>
                                            </span>
                                            <span {...stylex.attrs(styles.threadText)}>{text}</span>
                                        </div>
                                    ))}
                            </Fade>
                        </section>
                        <section data-component="RippleList" {...stylex.attrs(styles.panel)}>
                            <p {...stylex.attrs(styles.panelHead)}>
                                Moving the demo
                                {properties.isOpenAt(2) ? (
                                    <span {...stylex.attrs(styles.count, styles.countDone)}>
                                        1 change
                                    </span>
                                ) : (
                                    <Issue>2 by hand</Issue>
                                )}
                            </p>
                            <Fade isOpen={properties.isOpenAt(2)} style={styles.stack}>
                                {ripple.map(([done, chore, connector]) => (
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
                                                <span {...stylex.attrs(styles.rowTitle)}>
                                                    {done}
                                                </span>
                                                <span {...stylex.attrs(styles.passed)}>
                                                    <Glyph size={15} name="check" />
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
                                                    <Glyph size={15} name="open" />
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
                                                    <Glyph size={15} name="check" />
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
        marginTop: "2rem",
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
    dark: {
        backgroundColor: color.foreground,
        borderColor: color.foreground,
        color: color.background,
    },
    body: {
        display: "grid",
        gap: "1.25rem",
        gridTemplateRows: "auto minmax(0, 1fr) minmax(0, 1.1fr)",
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
        gridTemplateRows: "auto minmax(0, 1fr)",
        listStyle: "none",
        margin: 0,
        minHeight: 0,
        overflow: "hidden",
        padding: 0,
    },
    day: {
        alignContent: "start",
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: { default: tokens.hairline, ":last-child": 0 },
        display: "grid",
        minHeight: 0,
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
        minHeight: 0,
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: {
            default: "minmax(0, 1.1fr) minmax(0, 1.15fr) minmax(0, 0.95fr)",
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
        height: "2rem",
        margin: 0,
        paddingInline: "0.75rem",
        whiteSpace: "nowrap",
    },
    count: {
        color: color.mutedForeground,
        fontWeight: 500,
        marginLeft: "auto",
    },
    issue: {
        alignItems: "center",
        color: "#a15c07",
        display: "inline-flex",
        fontSize: "0.72rem",
        fontWeight: 600,
        gap: "0.375rem",
        marginLeft: "auto",
        whiteSpace: "nowrap",
        "::before": {
            backgroundColor: "#e0a030",
            borderRadius: "50%",
            content: "''",
            height: "0.375rem",
            width: "0.375rem",
        },
    },
    countDone: {
        color: "#3c8f58",
        fontWeight: 600,
    },
    stack: {
        display: "grid",
        minWidth: 0,
    },
    threads: {
        display: "grid",
    },
    free: {
        backgroundColor: "rgb(60 143 88 / 7%)",
        borderLeftColor: "rgb(60 143 88 / 55%)",
        borderLeftStyle: "solid",
        borderLeftWidth: "2px",
        borderRadius: "4px",
        color: "#3c8f58",
        display: "grid",
        minWidth: 0,
        paddingBlock: "0.25rem",
        paddingInline: "0.375rem",
    },
    waitingRow: {
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: tokens.hairline, ":last-child": 0 },
        display: "grid",
        gap: "0.25rem",
        minWidth: 0,
        padding: "0.5rem 0.75rem",
    },
    threadHead: {
        minWidth: 0,
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
    },
    threadText: {
        color: color.foreground,
        fontSize: "0.78rem",
        lineHeight: 1.4,
    },
    threadAction: {
        marginLeft: "auto",
        whiteSpace: "nowrap",
        color: color.mutedForeground,
        fontSize: "0.72rem",
        fontWeight: 600,
    },
    row: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: tokens.hairline, ":last-child": 0 },
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        minHeight: "2.125rem",
        minWidth: 0,
        paddingBlock: "0.1875rem",
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
    strong: {
        fontWeight: 600,
    },
    quiet: {
        color: color.mutedForeground,
        flexShrink: 0,
    },
});
