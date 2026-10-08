import { Icon } from "@destack/icon";
import arrowSquareOut from "@destack/icon/phosphor/arrow-square-out";
import at from "@destack/icon/phosphor/at";
import checkCircle from "@destack/icon/phosphor/check-circle";
import circle from "@destack/icon/phosphor/circle";
import circleDashed from "@destack/icon/phosphor/circle-dashed";
import circleHalf from "@destack/icon/phosphor/circle-half";
import desktop from "@destack/icon/phosphor/desktop";
import deviceMobile from "@destack/icon/phosphor/device-mobile";
import dotsThree from "@destack/icon/phosphor/dots-three";
import fileText from "@destack/icon/phosphor/file-text";
import folder from "@destack/icon/phosphor/folder";
import funnel from "@destack/icon/phosphor/funnel";
import gitBranch from "@destack/icon/phosphor/git-branch";
import gitPullRequest from "@destack/icon/phosphor/git-pull-request";
import kanban from "@destack/icon/phosphor/kanban";
import listBullets from "@destack/icon/phosphor/list-bullets";
import magnifyingGlass from "@destack/icon/phosphor/magnifying-glass";
import paperPlaneRight from "@destack/icon/phosphor/paper-plane-right";
import paperclip from "@destack/icon/phosphor/paperclip";
import smiley from "@destack/icon/phosphor/smiley";
import * as style from "@destack/style";
import { color, font, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { AvatarGroup } from "@destack/ui/avatar";
import { Button } from "@destack/ui/button";
import { Checkbox } from "@destack/ui/checkbox";
import { InputGroup, InputGroupAddon, InputGroupInput } from "@destack/ui/input-group";
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemMedia,
    ItemTitle,
} from "@destack/ui/item";
import { Progress } from "@destack/ui/progress";
import { Separator } from "@destack/ui/separator";
import { Spinner } from "@destack/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@destack/ui/table";
import type { JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { AppHeader, appStyles, appText } from "../figure/app";
import {
    coverOf,
    launch,
    messages,
    posts,
    signups,
    sources,
    type TaskState,
    tasks,
    week,
} from "./corner";

/** The hours your week shows, from nine to five. */
const HOURS = [9, 10, 11, 12, 13, 14, 15, 16];

/** The weekdays your week shows, with their dates. */
const DAYS = ["Mon 20", "Tue 21", "Wed 22", "Thu 23", "Fri 24"];

/** The task states in the order the list groups them, with their names and marks. */
const STATES: readonly (readonly [state: TaskState, name: string])[] = [
    ["started", "In progress"],
    ["review", "In review"],
    ["todo", "Todo"],
    ["done", "Done"],
];

/** Draw a section of your own space by its ledger row. */
export function sectionOf(row: number): JSX.Element {
    if (row === 1) {
        return <Launch />;
    } else if (row === 2) {
        return <Tasks />;
    } else if (row === 3) {
        return <Chat />;
    } else if (row === 4) {
        return <Code />;
    } else if (row === 5) {
        return <Website />;
    } else if (row === 6) {
        return <Week />;
    }

    return <Signups />;
}

/** Lay out an app's page beside the shared rail, its header first. */
function Paned(properties: { children: JSX.Element }) {
    return (
        <article inert {...style.attrs(appStyles.page)}>
            {properties.children}
        </article>
    );
}

/** Draw a page's header, its title and actions, with its view controls as chips beneath. */
function Toolbar(properties: { title: string; actions: JSX.Element; children?: JSX.Element }) {
    return (
        <>
            <AppHeader title={properties.title}>{properties.actions}</AppHeader>
            {properties.children === undefined ? undefined : (
                <div {...style.attrs(appStyles.properties)}>{properties.children}</div>
            )}
        </>
    );
}

/** Draw a person's face: you in teal, your agent in violet, anyone else in their tint. */
function Face(properties: { owner: string; tint?: string; size?: "sm" | "default" }) {
    const tint = () =>
        properties.tint ?? (properties.owner === "A" ? palette.violet : palette.teal);

    return (
        <span
            style={{ "background-color": tint() }}
            {...style.attrs(appStyles.avatar, properties.size === "default" && styles.faceLarge)}
        >
            {properties.owner}
        </span>
    );
}

/** Draw who is in a page right now: you and your agent. */
function Present() {
    return (
        <AvatarGroup aria-label="In this page">
            <Face owner="F" />
            <Face owner="A" />
        </AvatarGroup>
    );
}

/** Draw a task's state: a dashed ring to do, a half ring started, a ring in review, a check done. */
function StateMark(properties: { state: TaskState }) {
    const marks = { todo: circleDashed, started: circleHalf, review: circle, done: checkCircle };
    const tints = {
        todo: color.mutedForeground,
        started: palette.amber,
        review: palette.green,
        done: palette.indigo,
    };

    return (
        <span style={{ color: tints[properties.state] }} {...style.attrs(styles.mark)}>
            <Icon
                icon={marks[properties.state]}
                weight={properties.state === "done" ? "fill" : "bold"}
            />
        </span>
    );
}

/** Draw the launch plan: a page you and your agent edit together, its tasks linked in place. */
function Launch() {
    return (
        <Paned>
            <Toolbar
                title="Launch plan"
                actions={
                    <>
                        <Present />
                        <span {...style.attrs(appStyles.action)}>Agent edited just now</span>
                        <span {...style.attrs(appStyles.primary)}>Share</span>
                    </>
                }
            />
            <article {...style.attrs(styles.document)}>
                <div {...style.attrs(appStyles.properties)}>
                    <span {...style.attrs(appStyles.property)}>
                        <StateMark state="started" />
                        In progress
                    </span>
                    <span {...style.attrs(appStyles.property)}>Fri 24 Oct</span>
                    <span {...style.attrs(appStyles.property)}>
                        <Progress value={66} aria-label="Tasks done" xstyle={styles.progress} />4 of
                        6 tasks
                    </span>
                    <span {...style.attrs(appStyles.property)}>Reminder Thu</span>
                </div>
                <p {...style.attrs(appText.body)}>
                    Ship to the waitlist on Thursday, then open signups on Friday. Pricing v4 goes
                    live with the launch post.
                </p>
                <ul {...style.attrs(styles.checklist)}>
                    {launch.map(([item, isDone]) => (
                        <li {...style.attrs(appText.body, styles.check)}>
                            <Checkbox checked={isDone} aria-label={item} />
                            <span {...style.attrs(isDone && styles.struck)}>{item}</span>
                            {item.startsWith("Record") ? (
                                <>
                                    <span {...style.attrs(appStyles.property)}>
                                        <StateMark state="started" />
                                        LCH-14 · Wed 11:00
                                    </span>
                                    <span {...style.attrs(styles.cursor)}>
                                        <span {...style.attrs(appText.meta, styles.cursorName)}>
                                            Agent
                                        </span>
                                    </span>
                                </>
                            ) : undefined}
                        </li>
                    ))}
                </ul>
                <Item variant="outline" size="sm" xstyle={styles.comment}>
                    <ItemMedia>
                        <Face owner="A" />
                    </ItemMedia>
                    <ItemContent>
                        <ItemTitle xstyle={appText.body}>
                            Agent <span {...style.attrs(styles.quiet)}>09:13</span>
                        </ItemTitle>
                        <ItemDescription xstyle={appText.body}>
                            Moved the demo to Wednesday so it lands before Thursday's invites.
                        </ItemDescription>
                    </ItemContent>
                </Item>
            </article>
        </Paned>
    );
}

/** Draw the launch tasks: your list grouped by state, your agent working through its share. */
function Tasks() {
    return (
        <Paned>
            <Toolbar
                title="Launch tasks"
                actions={<span {...style.attrs(appStyles.primary)}>+ New task</span>}
            >
                <span {...style.attrs(appStyles.property, styles.on)}>
                    <Icon icon={listBullets} />
                    List
                </span>
                <span {...style.attrs(appStyles.property)}>
                    <Icon icon={kanban} />
                    Board
                </span>
                <span {...style.attrs(appStyles.property)}>
                    <Icon icon={funnel} />
                    Due this week
                </span>
            </Toolbar>
            <div {...style.attrs(styles.list)}>
                {STATES.map(([state, name]) => (
                    <>
                        <div {...style.attrs(appText.body, styles.groupRow)}>
                            <StateMark state={state} />
                            <b>{name}</b>
                            <span {...style.attrs(styles.quiet)}>
                                {tasks.filter((task) => task[2] === state).length}
                            </span>
                        </div>
                        {tasks
                            .filter((task) => task[2] === state)
                            .map(([id, title, , owner, tint, due]) => (
                                <div {...style.attrs(appText.body, styles.taskRow)}>
                                    <span {...style.attrs(styles.quiet, styles.code)}>{id}</span>
                                    <StateMark state={state} />
                                    <span
                                        {...style.attrs(
                                            styles.taskTitle,
                                            state === "done" && styles.struck,
                                        )}
                                    >
                                        {title}
                                    </span>
                                    {id === "LCH-15" ? (
                                        <span {...style.attrs(styles.inline, styles.agentNote)}>
                                            <Spinner aria-label="Agent is writing" />
                                            writing…
                                        </span>
                                    ) : (
                                        <span {...style.attrs(appStyles.property)}>
                                            <span {...style.attrs(styles.dot, styles.signal)} />
                                            Launch
                                        </span>
                                    )}
                                    <span {...style.attrs(styles.quiet)}>{due}</span>
                                    <Face owner={owner} tint={tint} />
                                </div>
                            ))}
                    </>
                ))}
            </div>
        </Paned>
    );
}

/** Draw the launch channel: your thread with your agent in it, its changes linked to the apps they touched. */
function Chat() {
    return (
        <Paned>
            <Toolbar
                title="# launch"
                actions={
                    <>
                        <AvatarGroup aria-label="Members">
                            <Face owner="F" />
                            <Face owner="A" />
                        </AvatarGroup>
                        <span {...style.attrs(appStyles.action)}>Ship v4 on Friday</span>
                    </>
                }
            />
            <div {...style.attrs(appText.meta, styles.divider)}>
                <Separator />
                Today
                <Separator />
            </div>
            <div {...style.attrs(styles.messages)}>
                {messages.map(([author, time, line], index) => (
                    <div {...style.attrs(styles.message)}>
                        <Face owner={author === "Agent" ? "A" : "F"} size="default" />
                        <div {...style.attrs(styles.messageBody)}>
                            <span {...style.attrs(appText.body)}>
                                <b>{author}</b> <span {...style.attrs(styles.quiet)}>{time}</span>
                            </span>
                            <span {...style.attrs(appText.body)}>{line}</span>
                            {index === 1 ? (
                                <Item variant="outline" size="sm" xstyle={styles.attachment}>
                                    <ItemMedia>
                                        <StateMark state="started" />
                                    </ItemMedia>
                                    <ItemContent>
                                        <ItemTitle xstyle={appText.body}>
                                            LCH-14 Record the demo
                                        </ItemTitle>
                                        <ItemDescription xstyle={appText.meta}>
                                            Tasks · due moved to Wed 11:00
                                        </ItemDescription>
                                    </ItemContent>
                                </Item>
                            ) : undefined}
                            {index === 1 ? (
                                <span {...style.attrs(styles.inline)}>
                                    <span {...style.attrs(appStyles.property)}>👍 2</span>
                                    <span {...style.attrs(appStyles.property)}>🚀 1</span>
                                </span>
                            ) : undefined}
                            {index === 2 ? (
                                <span {...style.attrs(appText.body, styles.inline, styles.replies)}>
                                    <AvatarGroup aria-label="Replied">
                                        <Face owner="F" />
                                    </AvatarGroup>
                                    3 replies · last at 09:52
                                </span>
                            ) : undefined}
                        </div>
                    </div>
                ))}
            </div>
            <span {...style.attrs(appText.meta, styles.inline, styles.agentNote)}>
                <Spinner aria-label="Agent is typing" />
                Agent is typing…
            </span>
            <InputGroup>
                <InputGroupInput placeholder="Message #launch, or ask your agent" readonly />
                <InputGroupAddon align="inline-end">
                    <Icon icon={paperclip} />
                    <Icon icon={at} />
                    <Icon icon={smiley} />
                    <Icon icon={paperPlaneRight} />
                </InputGroupAddon>
            </InputGroup>
        </Paned>
    );
}

/** Draw the code behind your apps: the repository, its latest commit and your agent's open pull request. */
function Code() {
    return (
        <Paned>
            <Toolbar
                title="launchkit"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Preview branch</span>
                        <span {...style.attrs(appStyles.primary)}>Code ▾</span>
                    </>
                }
            >
                <span {...style.attrs(appStyles.property)}>
                    <Icon icon={gitBranch} />
                    main
                </span>
                <span {...style.attrs(appStyles.property)}>2 branches</span>
                <span {...style.attrs(appStyles.property)}>48 commits</span>
            </Toolbar>
            <Item variant="outline" size="sm" xstyle={styles.pull}>
                <ItemMedia xstyle={styles.green}>
                    <Icon icon={gitPullRequest} weight="bold" />
                </ItemMedia>
                <ItemContent>
                    <ItemTitle xstyle={appText.body}>Show a friend's free slots #12</ItemTitle>
                    <ItemDescription xstyle={appText.meta}>
                        Agent wants to merge friend-free-slots into main · checks passed
                    </ItemDescription>
                </ItemContent>
                <ItemActions>
                    <Button size="sm" variant="outline">
                        Review
                    </Button>
                </ItemActions>
            </Item>
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHead colspan={3}>
                            <span {...style.attrs(appText.body, styles.inline)}>
                                <Face owner="F" />
                                <b>Florian</b>
                                <span>Mail the first batch on Thursday</span>
                                <span {...style.attrs(styles.quiet, styles.code)}>a41f2c9</span>
                                <span {...style.attrs(appStyles.property)}>
                                    <Icon icon={checkCircle} weight="fill" />
                                    passed
                                </span>
                            </span>
                        </TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {sources.map(([name, change, when]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(styles.inline)}>
                                    <Icon icon={name.includes(".") ? fileText : folder} />
                                    {name}
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{change}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{when}</TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Paned>
    );
}

/** Draw your website: its pages in the editor, previewed as visitors see them at your own address. */
function Website() {
    return (
        <Paned>
            <Toolbar
                title="Notes"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Published 2 h ago</span>
                        <span {...style.attrs(appStyles.primary)}>Publish</span>
                    </>
                }
            >
                <span {...style.attrs(appStyles.property, styles.on)}>
                    <Icon icon={desktop} />
                    Desktop
                </span>
                <span {...style.attrs(appStyles.property)}>
                    <Icon icon={deviceMobile} />
                    Mobile
                </span>
                <span {...style.attrs(appStyles.property)}>
                    <span {...style.attrs(styles.dot, styles.live)} />
                    florian.dev
                    <Icon icon={arrowSquareOut} />
                </span>
            </Toolbar>
            <div {...style.attrs(styles.preview)}>
                <div {...style.attrs(appText.body, styles.siteBar)}>
                    <b>florian</b>
                    <span {...style.attrs(styles.siteNav)}>
                        <span>Notes</span>
                        <span>About</span>
                        <span>Waitlist</span>
                    </span>
                </div>
                <h3 {...style.attrs(appText.heading, styles.title)}>Notes</h3>
                <p {...style.attrs(appText.body, styles.quiet)}>
                    On building small software: product, pricing and the tools I run it on.
                </p>
                <ol {...style.attrs(styles.postGrid)}>
                    {posts.map(([title, topic, date, from, to], index) => (
                        <li {...style.attrs(styles.post)}>
                            <span
                                style={{ background: coverOf(index, from, to) }}
                                {...style.attrs(styles.cover)}
                            />
                            <span {...style.attrs(appText.meta, styles.quiet)}>
                                {topic} · {date}
                            </span>
                            <b {...style.attrs(appText.body)}>{title}</b>
                        </li>
                    ))}
                </ol>
            </div>
        </Paned>
    );
}

/** Draw your week: the events your apps put there and the free times your booking page offers. */
function Week() {
    return (
        <Paned>
            <Toolbar
                title="October 20 – 24"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Day</span>
                        <span {...style.attrs(appStyles.action, appStyles.actionStrong)}>Week</span>
                        <span {...style.attrs(appStyles.action)}>Month</span>
                        <span {...style.attrs(appStyles.primary)}>Share booking link</span>
                    </>
                }
            />
            <div {...style.attrs(appText.meta, styles.week)}>
                {DAYS.map((day, index) => (
                    <span
                        style={{ "grid-column": String(index + 2) }}
                        {...style.attrs(styles.dayName, index === 0 && styles.today)}
                    >
                        {day}
                    </span>
                ))}
                {HOURS.map((hour) => (
                    <span
                        style={{ "grid-row": `${rowOf(hour)} / span 2` }}
                        {...style.attrs(styles.hour)}
                    >
                        {hour}:00
                    </span>
                ))}
                {week.map(([day, start, end, title, isBookable]) => (
                    <span
                        style={{
                            "grid-column": String(day + 2),
                            "grid-row": `${rowOf(start)} / ${rowOf(end)}`,
                        }}
                        {...style.attrs(
                            styles.event,
                            isBookable
                                ? styles.bookable
                                : title.includes("·")
                                  ? styles.fromApps
                                  : styles.personal,
                        )}
                    >
                        {title}
                    </span>
                ))}
            </div>
        </Paned>
    );
}

/** Return the grid row a time of day starts at in the week: two rows per hour under the two of the day names. */
function rowOf(hour: number): number {
    return 3 + (hour - (HOURS[0] ?? 0)) * 2;
}

/** Draw your waitlist's signups: the table your public form fills, ready to invite the first batch. */
function Signups() {
    return (
        <Paned>
            <Toolbar
                title="Signups"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>24 joined today</span>
                        <span {...style.attrs(appStyles.primary)}>Invite 400</span>
                    </>
                }
            >
                <span {...style.attrs(appStyles.property)}>
                    <Icon icon={magnifyingGlass} />
                    Search 1,840 signups
                </span>
                <span {...style.attrs(appStyles.property, styles.on)}>
                    <Icon icon={funnel} />
                    Waiting
                </span>
                <span {...style.attrs(appStyles.property)}>Source: any</span>
            </Toolbar>
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHead xstyle={styles.select}>
                            <Checkbox aria-label="Select all" indeterminate />
                        </TableHead>
                        <TableHead>Email</TableHead>
                        <TableHead>Source</TableHead>
                        <TableHead>Joined</TableHead>
                        <TableHead>Status</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {signups.map(([email, source, joined, state], index) => (
                        <TableRow>
                            <TableCell xstyle={styles.select}>
                                <Checkbox checked={index < 4} aria-label={email} />
                            </TableCell>
                            <TableCell xstyle={appText.body}>{email}</TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>{source}</span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{joined}</TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>
                                    <span
                                        {...style.attrs(
                                            styles.dot,
                                            state === "invited" ? styles.live : styles.amber,
                                        )}
                                    />
                                    {state === "invited" ? "Invited" : "Waiting"}
                                </span>
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <span {...style.attrs(appText.meta, styles.inline, styles.quiet)}>
                <Icon icon={dotsThree} />
                1,833 more · filled by your public waitlist form
            </span>
        </Paned>
    );
}

/** The styles of your space's apps, drawn at the size of a real window. */
const styles = style.create({
    on: {
        backgroundColor: `color-mix(in srgb, ${color.primary} 14%, transparent)`,
        color: color.foreground,
    },
    faceLarge: {
        fontSize: "0.75rem",
        height: "1.75rem",
        width: "1.75rem",
    },
    quiet: {
        color: color.mutedForeground,
    },
    code: {
        fontFamily: font.code,
    },
    inline: {
        alignItems: "center",
        display: "inline-flex",
        gap: space[2],
    },
    title: {
        color: color.foreground,
        margin: 0,
    },
    document: {
        display: "grid",
        gap: space[3],
        maxWidth: "40rem",
    },
    progress: {
        width: "3rem",
    },
    dot: {
        borderRadius: "50%",
        display: "inline-block",
        flexShrink: 0,
        height: "0.5rem",
        width: "0.5rem",
    },
    amber: {
        backgroundColor: palette.amber,
    },
    signal: {
        backgroundColor: palette.signal,
    },
    live: {
        backgroundColor: palette.green,
    },
    green: {
        color: palette.green,
    },
    checklist: {
        display: "grid",
        gap: space[2],
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    check: {
        alignItems: "center",
        display: "flex",
        gap: space[3],
    },
    struck: {
        color: color.mutedForeground,
        textDecorationLine: "line-through",
    },
    cursor: {
        backgroundColor: palette.violet,
        height: "1.25em",
        position: "relative",
        width: "2px",
    },
    cursorName: {
        backgroundColor: palette.violet,
        borderRadius: radius[1],
        color: palette.cream,
        insetBlockEnd: "100%",
        insetInlineStart: 0,
        paddingInline: space[1],
        position: "absolute",
        whiteSpace: "nowrap",
    },
    comment: {
        maxWidth: "26rem",
    },
    mark: {
        display: "inline-flex",
    },
    list: {
        display: "grid",
    },
    groupRow: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: radius[2],
        display: "flex",
        gap: space[2],
        paddingBlock: space[1],
        paddingInline: space[3],
    },
    taskRow: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "grid",
        gap: space[3],
        gridTemplateColumns: "4.5rem auto minmax(0, 1fr) auto auto auto",
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    taskTitle: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    agentNote: {
        color: palette.violet,
    },
    divider: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "grid",
        gap: space[3],
        gridTemplateColumns: "1fr auto 1fr",
    },
    messages: {
        display: "grid",
        gap: space[4],
    },
    message: {
        alignItems: "start",
        display: "grid",
        gap: space[3],
        gridTemplateColumns: "auto minmax(0, 1fr)",
    },
    messageBody: {
        display: "grid",
        gap: space[1],
        justifyItems: "start",
    },
    attachment: {
        maxWidth: "22rem",
    },
    replies: {
        color: color.primary,
        fontWeight: weight.semibold,
    },
    pull: {
        borderColor: `color-mix(in srgb, ${palette.green} 40%, ${color.border})`,
    },
    preview: {
        backgroundColor: color.background,
        borderColor: color.border,
        borderRadius: radius[3],
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        gap: space[2],
        padding: space[5],
    },
    siteBar: {
        alignItems: "center",
        display: "flex",
        justifyContent: "space-between",
        paddingBlockEnd: space[3],
    },
    siteNav: {
        color: color.mutedForeground,
        display: "flex",
        gap: space[4],
    },
    postGrid: {
        display: "grid",
        gap: space[4],
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    post: {
        display: "grid",
        gap: space[1],
    },
    cover: {
        aspectRatio: "4 / 3",
        borderRadius: radius[2],
        display: "block",
    },
    week: {
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderRadius: radius[3],
        display: "grid",
        gridAutoRows: "1.25rem",
        gridTemplateColumns: "3rem repeat(5, minmax(0, 1fr))",
        overflow: "hidden",
        rowGap: 0,
    },
    dayName: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        fontWeight: weight.semibold,
        gridRow: "1 / span 2",
        paddingBlock: space[1],
        paddingInline: space[2],
    },
    today: {
        color: color.primary,
    },
    hour: {
        color: color.mutedForeground,
        gridColumn: "1",
        paddingInlineStart: space[2],
    },
    event: {
        borderRadius: radius[2],
        lineHeight: 1.25,
        marginInline: space[1],
        marginBlock: "1px",
        overflow: "hidden",
        paddingInline: space[2],
        paddingBlock: space[1],
    },
    personal: {
        backgroundColor: `color-mix(in srgb, ${palette.teal} 18%, ${color.card})`,
        color: color.foreground,
    },
    fromApps: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 22%, ${color.card})`,
        color: color.foreground,
        fontWeight: weight.semibold,
    },
    bookable: {
        borderColor: palette.green,
        borderStyle: "dashed",
        borderWidth: stroke.border,
        color: palette.green,
    },
    select: {
        width: "2.5rem",
    },
});
