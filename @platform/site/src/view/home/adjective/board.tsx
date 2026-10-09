import { color, font, shadow, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";
import { media } from "@destack/style/media.stylex";

import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import type { Stagger } from "../figure/stagger";
import { Tile } from "../figure/tile";
import { palette } from "../../palette.stylex";

/** The engine's services the launch board runs on, numbered as the board marks them: changing the app, reading the other apps, keeping it current, and sharing it. */
export const services: readonly string[] = ["Forge", "Database", "Workflows", "Access"];

/** The launch tasks still open: the identifier, the title and the due date. */
const tasks: readonly (readonly [id: string, title: string, due: string])[] = [
    ["DST-14", "Record the demo", "Wed"],
    ["DST-15", "Write the launch post", "Thu"],
    ["DST-16", "Mail the first invites", "Thu"],
    ["DST-13", "Turn on pricing", "Fri"],
    ["DST-17", "Open signups", "Fri"],
];

/** The latest waitlist signups: the address and when they joined. */
const signups: readonly (readonly [email: string, joined: string])[] = [
    ["maya@northwind.io", "2 min"],
    ["jonas@kettle.dev", "14 min"],
    ["tara@fieldnotes.co", "1 h"],
    ["leo@brightpath.app", "3 h"],
    ["sam@quietly.so", "5 h"],
];

/** The demos booked this week: the day and time, and who booked it. */
const demos: readonly (readonly [time: string, person: string])[] = [
    ["Wed 11:00", "Friend"],
    ["Thu 15:00", "Jonas"],
    ["Fri 09:30", "Maya"],
    ["Fri 11:30", "Ines"],
    ["Fri 16:30", "Leo"],
];

/** Draw what a rented part of the board fails to do, marked the same way everywhere. */
function Issue(properties: { children: string }) {
    return (
        <span data-component="Issue" {...style.attrs(styles.issue)}>
            {properties.children}
        </span>
    );
}

/** Draw a message of yours as a rounded pill on the right. */
function UserMessage(properties: { children: JSX.Element }) {
    return (
        <p data-component="UserMessage" {...style.attrs(styles.bubble)}>
            {properties.children}
        </p>
    );
}

/** Draw a reply as the chat sets it: how long the agent worked, its text, and the actions under it. */
function Reply(properties: { time: string; children: JSX.Element }) {
    return (
        <div data-component="AgentMessage" {...style.attrs(styles.reply)}>
            <p data-component="WorkSummary" {...style.attrs(styles.worked)}>
                Worked for {properties.time}
                <Glyph size={15} name="forward" />
            </p>
            <div {...style.attrs(styles.replyText)}>{properties.children}</div>
            <p data-component="MessageActions" {...style.attrs(styles.actions)}>
                <Glyph size={15} name="copy" />
                <Glyph size={15} name="like" />
                <Glyph size={15} name="dislike" />
                <Glyph size={15} name="share" />
            </p>
        </div>
    );
}

/** Draw one column of the board: its mark and name, what it says about itself, and its rows. */
function Column(properties: {
    mark: JSX.Element;
    name: string;
    note: JSX.Element;
    service: number;
    children: JSX.Element;
}) {
    return (
        <section
            data-component="BoardColumn"
            data-service={String(properties.service)}
            {...style.attrs(styles.panel)}
        >
            <p {...style.attrs(styles.panelHead)}>
                {properties.mark}
                {properties.name}
                {properties.note}
            </p>
            <div {...style.attrs(styles.stack)}>{properties.children}</div>
        </section>
    );
}

/** Draw a row a column cannot fill while rented, with what it would take. */
function Missing(properties: { children: string }) {
    return (
        <p data-component="MissingRows" {...style.attrs(styles.row, styles.missing)}>
            {properties.children}
        </p>
    );
}

/**
 * Draw a launch board asked of your agent, with the conversation on the left and the board on the right.
 *
 * While rented, the agent can only ask the vendor for the feature and glue a copy together: a vote on a feature request, and an hourly copy into a document.
 * Once owned, the agent changes your own Tasks app: a board view merged in Forge that reads the waitlist and the calendar live and is shared through your space's access.
 */
export function LaunchBoard(properties: { isOpenAt: Stagger }) {
    return (
        <div {...style.attrs(styles.app)}>
            {/* talk to your agent */}
            <Fade
                isOpen={properties.isOpenAt(0)}
                xstyle={styles.conversation}
                isLarge
                component={properties.isOpenAt(0) ? "SpaceChat" : "Chat"}
            >
                <div {...style.attrs(styles.thread)}>
                    <UserMessage>
                        Two days to launch. Make me a launch board: open tasks, waitlist signups and
                        booked demos, in one place.
                    </UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply time="1m 12s">
                            <p {...style.attrs(styles.paragraph)}>
                                Added a <b {...style.attrs(styles.strong)}>Board</b> view to your
                                Tasks app. It reads{" "}
                                <code {...style.attrs(styles.code)}>Signup</code> from Waitlist and{" "}
                                <code {...style.attrs(styles.code)}>Event</code> from Calendar, so
                                it stays current. Preview it, then merge.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="2m 40s">
                            <p {...style.attrs(styles.paragraph)}>
                                Linear can't show other apps' data, so I upvoted their request for
                                custom dashboards, and a Zapier zap now copies your open tasks into
                                Notion hourly. Signups and demos aren't connected.
                            </p>
                        </Reply>
                    )}
                    <UserMessage>Ship it, and share it with the team.</UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply time="9s">
                            <p {...style.attrs(styles.paragraph)}>
                                Merged. Your team sees the board in Tasks, and I can read it too.
                            </p>
                        </Reply>
                    ) : (
                        <Reply time="21s">
                            <p {...style.attrs(styles.paragraph)}>
                                Shared the Notion page by link. Anyone with the link can see it.
                            </p>
                        </Reply>
                    )}
                </div>
                <div data-component="Composer" {...style.attrs(styles.composer)}>
                    <span {...style.attrs(styles.quiet)}>Ask anything</span>
                    <span {...style.attrs(styles.controls)}>
                        <span {...style.attrs(styles.iconButton)}>
                            <Glyph size={15} name="plus" />
                        </span>
                        {properties.isOpenAt(0) ? (
                            <span data-component="AgentScope" {...style.attrs(styles.chip)}>
                                <Tile name="tasks" />
                                <Tile name="calendar" />
                                Your space · live
                            </span>
                        ) : (
                            <span data-component="ConnectorScope" {...style.attrs(styles.chip)}>
                                3 connectors
                            </span>
                        )}
                        <span {...style.attrs(styles.iconButton, styles.pushed)}>
                            <Glyph size={15} name="microphone" />
                        </span>
                        <span {...style.attrs(styles.send)}>
                            <Glyph size={15} name="up" />
                        </span>
                    </span>
                </div>
            </Fade>

            {/* show the board: glued together from rented parts, or a view of your own app */}
            <div
                data-component={properties.isOpenAt(1) ? "SpaceApp" : "RentedBoard"}
                {...style.attrs(styles.pane)}
            >
                {/* name where the board lives */}
                <Fade isOpen={properties.isOpenAt(1)} xstyle={styles.bar}>
                    {properties.isOpenAt(1) ? (
                        <>
                            <Tile name="tasks" />
                            <span {...style.attrs(styles.title)}>Tasks › Launch board</span>
                        </>
                    ) : (
                        <>
                            <Favicon icon="notion" tint="#191919" />
                            <span {...style.attrs(styles.title)}>Notion › Launch board</span>
                        </>
                    )}
                    <span {...style.attrs(styles.end)}>
                        {properties.isOpenAt(3) ? (
                            <span data-service="3" {...style.attrs(styles.live)}>
                                <span {...style.attrs(styles.dot)} />
                                Live
                            </span>
                        ) : (
                            <span data-service="3" {...style.attrs(styles.quiet)}>
                                <Favicon icon="zapier" tint="#ff4f00" size={12} /> Synced 58 min ago
                            </span>
                        )}
                    </span>
                </Fade>

                <div data-component="Board" {...style.attrs(styles.body)}>
                    {/* change the app itself, or ask its vendor to */}
                    <Fade
                        isOpen={properties.isOpenAt(1)}
                        component={properties.isOpenAt(1) ? "ForgeChange" : "FeatureRequest"}
                    >
                        {properties.isOpenAt(1) ? (
                            <p data-service="1" {...style.attrs(styles.change)}>
                                <Glyph size={15} name="check" />
                                <code {...style.attrs(styles.code)}>tasks/board.tsx</code>
                                <span {...style.attrs(styles.added)}>+38</span>
                                <span {...style.attrs(styles.quiet)}>Merged by you in Forge</span>
                                <span {...style.attrs(styles.pushed, styles.button)}>
                                    View change
                                </span>
                            </p>
                        ) : (
                            <p data-service="1" {...style.attrs(styles.change)}>
                                <Favicon icon="linear" tint="#5e6ad2" />
                                <b {...style.attrs(styles.strong)}>Custom dashboards</b>
                                <span {...style.attrs(styles.quiet, styles.wide)}>
                                    Feature request · 4,212 votes
                                </span>
                                <Issue>Under review</Issue>
                                <span {...style.attrs(styles.button)}>▲ Upvoted</span>
                            </p>
                        )}
                    </Fade>

                    {/* read every column from where it lives */}
                    <div {...style.attrs(styles.columns)}>
                        <Column
                            service={2}
                            mark={
                                properties.isOpenAt(2) ? (
                                    <Tile name="tasks" />
                                ) : (
                                    <Favicon icon="linear" tint="#5e6ad2" />
                                )
                            }
                            name="Open tasks"
                            note={
                                properties.isOpenAt(2) ? (
                                    <span {...style.attrs(styles.count)}>5</span>
                                ) : (
                                    <Issue>Copied hourly</Issue>
                                )
                            }
                        >
                            {tasks.map(([id, title, due]) => (
                                <p data-component="TaskRow" {...style.attrs(styles.row)}>
                                    <span {...style.attrs(styles.id)}>{id}</span>
                                    <span {...style.attrs(styles.rowTitle)}>{title}</span>
                                    <span {...style.attrs(styles.quiet)}>{due}</span>
                                </p>
                            ))}
                        </Column>
                        <Column
                            service={2}
                            mark={
                                properties.isOpenAt(2) ? (
                                    <Favicon icon="table" tint="#4f8a5b" />
                                ) : (
                                    <Favicon icon="replit" tint="#f26207" />
                                )
                            }
                            name="Signups"
                            note={
                                properties.isOpenAt(2) ? (
                                    <span {...style.attrs(styles.count)}>1,840 · +12 today</span>
                                ) : (
                                    <Issue>Not connected</Issue>
                                )
                            }
                        >
                            {properties.isOpenAt(2) ? (
                                signups.map(([email, joined]) => (
                                    <p data-component="SignupRow" {...style.attrs(styles.row)}>
                                        <span {...style.attrs(styles.rowTitle)}>{email}</span>
                                        <span {...style.attrs(styles.quiet)}>{joined}</span>
                                    </p>
                                ))
                            ) : (
                                <Missing>Paste the waitlist export</Missing>
                            )}
                        </Column>
                        <Column
                            service={2}
                            mark={
                                properties.isOpenAt(2) ? (
                                    <Tile name="calendar" />
                                ) : (
                                    <Favicon icon="calendly" tint="#006bff" />
                                )
                            }
                            name="Demos booked"
                            note={
                                properties.isOpenAt(2) ? (
                                    <span {...style.attrs(styles.count)}>5</span>
                                ) : (
                                    <Issue>Not connected</Issue>
                                )
                            }
                        >
                            {properties.isOpenAt(2) ? (
                                demos.map(([time, person]) => (
                                    <p data-component="DemoRow" {...style.attrs(styles.row)}>
                                        <span {...style.attrs(styles.rowTitle)}>{person}</span>
                                        <span {...style.attrs(styles.quiet)}>{time}</span>
                                    </p>
                                ))
                            ) : (
                                <Missing>Paste the bookings export</Missing>
                            )}
                        </Column>
                    </div>

                    {/* share the board through your space, or by link */}
                    <Fade
                        isOpen={properties.isOpenAt(4)}
                        component={properties.isOpenAt(4) ? "SpaceAccess" : "PublicLink"}
                    >
                        {properties.isOpenAt(4) ? (
                            <p data-service="4" {...style.attrs(styles.access)}>
                                <Glyph size={15} name="person" />
                                Shared with your team · your agent can read it
                            </p>
                        ) : (
                            <p data-service="4" {...style.attrs(styles.access)}>
                                <Glyph size={15} name="link" />
                                Shared by link
                                <Issue>Anyone with the link</Issue>
                            </p>
                        )}
                    </Fade>
                </div>
            </div>
        </div>
    );
}

/** The launch board styles. */
const styles = style.create({
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) minmax(0, 1.75fr)",
            [media.maxMd]: "minmax(0, 1fr)",
        },
        height: "100%",
        minHeight: 0,
    },
    conversation: {
        borderRightColor: color.border,
        borderRightStyle: "solid",
        borderRightWidth: stroke.border,
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr) auto",
        minHeight: 0,
        paddingBlockStart: "0.75rem",
        paddingBlockEnd: "1rem",
        paddingInline: "1.25rem",
    },
    thread: {
        alignContent: "end",
        display: "grid",
        gap: "1rem",
        maskImage: "linear-gradient(to bottom, transparent, black 2rem)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
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
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
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
        fontFamily: font.code,
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
        borderColor: color.border,
        borderRadius: "1.25rem",
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        display: "grid",
        gap: "0.625rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        marginTop: "2rem",
        paddingBlockStart: "0.75rem",
        paddingBlockEnd: "0.5rem",
        paddingInlineStart: "1rem",
        paddingInlineEnd: "0.5rem",
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
        borderColor: color.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "inline-flex",
        fontSize: "0.72rem",
        gap: "0.25rem",
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "0.1875rem",
        paddingInlineStart: "0.25rem",
        paddingInlineEnd: "0.625rem",
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
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "0.5rem",
        height: "2.75rem",
        paddingInlineStart: "1rem",
        paddingInlineEnd: "1rem",
    },
    title: {
        fontWeight: 600,
        whiteSpace: "nowrap",
    },
    end: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        marginLeft: "auto",
    },
    live: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        gap: "0.375rem",
        whiteSpace: "nowrap",
    },
    dot: {
        backgroundColor: palette.lightGreen,
        borderRadius: "50%",
        height: "0.5rem",
        width: "0.5rem",
    },
    body: {
        display: "grid",
        gap: "1rem",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minHeight: 0,
        overflow: "hidden",
        paddingBlock: { default: "1.25rem", [media.maxMd]: "1rem" },
        paddingInline: { default: "1.5rem", [media.maxMd]: "1rem" },
    },
    change: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        minHeight: "2.75rem",
        minWidth: 0,
        paddingInline: "0.875rem",
        whiteSpace: "nowrap",
    },
    added: {
        color: palette.green,
        fontFamily: font.code,
        fontSize: "0.75rem",
        fontWeight: 600,
    },
    button: {
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        flexShrink: 0,
        fontSize: "0.75rem",
        fontWeight: 600,
        paddingBlock: "0.1875rem",
        paddingInline: "0.625rem",
        whiteSpace: "nowrap",
    },
    columns: {
        alignItems: "stretch",
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: {
            default: "repeat(3, minmax(0, 1fr))",
            [media.maxMd]: "minmax(0, 1fr)",
        },
        minHeight: 0,
    },
    panel: {
        alignContent: "start",
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        minWidth: 0,
        overflow: "hidden",
    },
    panelHead: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
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
    issue: {
        alignItems: "center",
        color: palette.rust,
        display: "inline-flex",
        fontSize: "0.72rem",
        fontWeight: 600,
        gap: "0.375rem",
        marginLeft: "auto",
        whiteSpace: "nowrap",
        "::before": {
            backgroundColor: palette.gold,
            borderRadius: "50%",
            content: "''",
            height: "0.375rem",
            width: "0.375rem",
        },
    },
    stack: {
        display: "grid",
        minWidth: 0,
    },
    row: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: stroke.border, ":last-child": 0 },
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        minHeight: "2.375rem",
        minWidth: 0,
        paddingInline: "0.75rem",
    },
    missing: {
        color: color.mutedForeground,
        fontStyle: "italic",
        justifyContent: "center",
        minHeight: "9.5rem",
    },
    id: {
        color: color.mutedForeground,
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: "0.6875rem",
    },
    rowTitle: {
        flexGrow: 1,
        fontWeight: 600,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    access: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        whiteSpace: "nowrap",
    },
    wide: { display: { default: null, [media.maxMd]: "none" } },
    strong: {
        fontWeight: 600,
    },
    quiet: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "inline-flex",
        flexShrink: 0,
        gap: "0.25rem",
    },
});
