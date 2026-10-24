import { Icon } from "@destack/icon";
import at from "@destack/icon/phosphor/at";
import checkCircle from "@destack/icon/phosphor/check-circle";
import circle from "@destack/icon/phosphor/circle";
import circleDashed from "@destack/icon/phosphor/circle-dashed";
import circleHalf from "@destack/icon/phosphor/circle-half";
import dotsThree from "@destack/icon/phosphor/dots-three";
import fileText from "@destack/icon/phosphor/file-text";
import funnel from "@destack/icon/phosphor/funnel";
import kanban from "@destack/icon/phosphor/kanban";
import listBullets from "@destack/icon/phosphor/list-bullets";
import magnifyingGlass from "@destack/icon/phosphor/magnifying-glass";
import paperPlaneRight from "@destack/icon/phosphor/paper-plane-right";
import paperclip from "@destack/icon/phosphor/paperclip";
import smiley from "@destack/icon/phosphor/smiley";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { color, font, motion, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { AvatarGroup } from "@destack/ui/avatar";
import { Checkbox } from "@destack/ui/checkbox";
import { Progress } from "@destack/ui/progress";
import { Separator } from "@destack/ui/separator";
import { Spinner } from "@destack/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@destack/ui/table";
import type { JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { AppHeader, appStyles, appText } from "../figure/app";
import { AgentAvatar } from "../figure/agent";
import {
    board,
    messages,
    press,
    type PressState,
    signups,
    type TaskState,
    tasks,
    week,
    workflows,
} from "./corner";
import type { Job } from "./space";
import { milestones } from "../project/tracker";
import { releases } from "../project/changelog";
import { ServiceGlyph } from "../figure/tile";
import type { Source } from "./source";
import { Favicon } from "../figure/glyph";

/** An app narrower than a phone screen, which then drops what only fits wide. */
const NARROW = "@container (max-width: 479px)";

/** The hours your week shows, from nine to five. */
const HOURS = [9, 10, 11, 12, 13, 14, 15, 16];

/** The weekdays your week shows, with their dates. */
const DAYS = ["Mon 20", "Tue 21", "Wed 22", "Thu 23", "Fri 24"];

/** The workflow your agent adds in fig. 4, lit once it lands. */
const ASKED_WORKFLOW = "asked-to-board";

/** How far your agent has got in fig. 4: not yet, its board built, or its workflow added. */
export type Change = "start" | "boarded" | "automated";

/** The task states in the order the list groups them, with their names and marks. */
const STATES: readonly (readonly [state: TaskState, name: string])[] = [
    ["started", "In progress"],
    ["review", "In review"],
    ["todo", "Todo"],
    ["done", "Done"],
];

/** The object in another app each task points at, by the app's icon and the object's live value. */
const taskLinks: Readonly<Record<string, readonly [icon: string, text: string]>> = {
    "DST-12": ["pages", "Pricing · published"],
    "DST-14": ["calendar", "Demo · Wed 11:00"],
    "DST-16": ["mail", "400 of 400 sent"],
    "DST-13": ["sync", "Runs Fri 09:00"],
    "DST-15": ["pages", "Launch post"],
    "DST-17": ["table", "1,840 waiting"],
    "DST-18": ["sync", "Zapier · owner unknown"],
    "DST-19": ["table", "9 subscriptions"],
};

/** The messages each signup wrote in Chat, by address. */
const signupMessages: Readonly<Record<string, string>> = {
    "maya@northwind.io": "1 · #launch",
    "noor@papertrail.fm": "3 · DM",
};

/** The invite each signup got from Mail, by address. */
const signupInvites: Readonly<Record<string, string>> = {
    "sam@quietly.so": "Opened",
    "noor@papertrail.fm": "Clicked",
    "ines@orbitlabs.eu": "Sent",
};

/** Draw a section of your space by its row's job, changed by a call when one lands on it. */
export function sectionOf(
    job: Job,
    change: Change,
    source: Source,
    isRemixed: boolean,
): JSX.Element {
    if (job === "documents") {
        return <Launch isRemixed={isRemixed} />;
    } else if (job === "tables") {
        return <Press isRemixed={isRemixed} />;
    } else if (job === "tasks") {
        return <Tasks isRemixed={isRemixed} />;
    } else if (job === "calendar") {
        return <Week isRemixed={isRemixed} />;
    } else if (job === "chat") {
        return <Chat isRemixed={isRemixed} />;
    } else if (job === "mail") {
        return <Mail isRemixed={isRemixed} />;
    } else if (job === "canvas") {
        return <Plan isRemixed={isRemixed} />;
    } else if (job === "workflows") {
        return <Flows change={change} isRemixed={isRemixed} />;
    } else if (job === "builder") {
        return <Signups change={change} isRemixed={isRemixed} />;
    } else if (job === "tracker") {
        return <Tracker isRemixed={isRemixed} />;
    } else if (job === "changelog") {
        return <Changelog isRemixed={isRemixed} />;
    }

    return <Connection source={source} isRemixed={isRemixed} />;
}

/** Your workflows once your agent adds the one that puts new askers on the board. */
const askedWorkflows = [
    [ASKED_WORKFLOW, "on message.create in #launch", "0", "just now"] as const,
    ...workflows,
];

/** A change your agent makes to an app of yours: what you ask for, and what it did. */
type Remix = { ask: string; done: string };

/** Return the change your agent makes to the app that does a job. */
export function remixOf(job: Job, source: Source): Remix {
    if (job === "documents") {
        return { ask: "Count down to the launch", done: "Added a countdown to the launch page" };
    } else if (job === "tables") {
        return { ask: "Flag press to follow up after 3 days", done: "Added Follow up to Press" };
    } else if (job === "tasks") {
        return { ask: "Add a priority to every task", done: "Added Priority to Tasks" };
    } else if (job === "calendar") {
        return { ask: "Keep my mornings free for focus", done: "Blocked 9–11 every weekday" };
    } else if (job === "chat") {
        return { ask: "Pin the launch checklist in #launch", done: "Pinned the checklist" };
    } else if (job === "mail") {
        return {
            ask: "Add a P.S. with each person's referral link",
            done: "Added a referral P.S.",
        };
    } else if (job === "canvas") {
        return { ask: "Draw how the tasks depend on each other", done: "Drew task arrows" };
    } else if (job === "workflows") {
        return { ask: "Tell me in Chat when a run fails", done: "Added failure alerts" };
    } else if (job === "builder") {
        return { ask: "Show who referred each signup", done: "Added Referred by to Signups" };
    } else if (job === "tracker") {
        return { ask: "Link each milestone to its task", done: "Linked milestones to Tasks" };
    } else if (job === "changelog") {
        return { ask: "Post each release to #launch", done: "Added Posted to Changelog" };
    }

    return { ask: `Chart my ${source.data} by week`, done: `Charted ${source.table} by week` };
}

/** The priority your agent gives each task once you ask it to add one. */
const priorities: Readonly<Record<string, string>> = {
    "DST-14": "P1",
    "DST-16": "P1",
    "DST-13": "P2",
    "DST-15": "P2",
    "DST-17": "P3",
    "DST-12": "P3",
};

/** Who referred each signup, which your agent reads from the launch post's links. */
const referrals: Readonly<Record<string, string>> = {
    "maya@northwind.io": "sam@quietly.so",
    "tara@fieldnotes.co": "noor@papertrail.fm",
    "sam@quietly.so": "Launch post",
};

/** The weekly totals of a connected app's chart, as shares of the busiest week. */
const weeks: readonly number[] = [0.45, 0.7, 0.35, 0.85, 0.6, 1, 0.75, 0.55];

/** Draw a personal app's data synced into a table of yours, its latest rows first. */
function Connection(properties: { source: Source; isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title={properties.source.table}
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Synced 4 min ago</span>
                        <span {...style.attrs(appStyles.primary)}>Chart it</span>
                    </>
                }
            >
                <span {...style.attrs(appStyles.property, styles.on)}>
                    <Favicon icon={properties.source.icon} tint={properties.source.tint} />
                    From {properties.source.name}
                </span>
                <span {...style.attrs(appStyles.property)}>
                    <ServiceGlyph icon="sync" />
                    Every 15 minutes
                </span>
                <span {...style.attrs(appStyles.property)}>Connected by your agent</span>
            </Toolbar>
            {/* the weekly chart your agent adds when asked */}
            {properties.isRemixed ? (
                <div {...style.attrs(styles.chart, styles.changed)}>
                    {weeks.map((share) => (
                        <span
                            style={{
                                height: `${String(share * 100)}%`,
                                "background-color": properties.source.tint,
                            }}
                            {...style.attrs(styles.bar)}
                        />
                    ))}
                </div>
            ) : undefined}
            <Table>
                <TableHeader>
                    <TableRow>
                        {properties.source.columns.map((column) => (
                            <TableHead>{column}</TableHead>
                        ))}
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {properties.source.rows.map((cells) => (
                        <TableRow>
                            {cells.map((cell) => (
                                <TableCell xstyle={appText.body}>{cell}</TableCell>
                            ))}
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <span {...style.attrs(appText.meta, styles.inline, styles.quiet)}>
                <Icon icon={dotsThree} />
                {`A year of your ${properties.source.data}, in your space, for every app and agent`}
            </span>
        </Paned>
    );
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

/** Draw a person's face: you in teal, anyone else in their tint, and your agent as its avatar. */
function Face(properties: { owner: string; tint?: string; size?: "sm" | "default" }) {
    return (
        <span
            style={{
                "background-color": isAgentOwner(properties.owner)
                    ? undefined
                    : (properties.tint ?? palette.teal),
            }}
            {...style.attrs(appStyles.avatar, properties.size === "default" && styles.faceLarge)}
        >
            {isAgentOwner(properties.owner) ? <AgentAvatar size="fill" /> : properties.owner}
        </span>
    );
}

/** Report whether a face belongs to your agent, marked `A` in the corner data. */
function isAgentOwner(owner: string): boolean {
    return owner === "A";
}

/** Draw who is in a page right now: you and your agent. */
function Present() {
    return (
        <AvatarGroup data-xray="Identity" aria-label="In this page">
            <Face owner="F" />
            <Face owner="A" />
        </AvatarGroup>
    );
}

/** Draw a task's state. */
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

/** Draw the launch plan: a page you and your agent edit, made of your other apps' parts. */
function Launch(properties: { isRemixed: boolean }) {
    // count the launch tasks done
    const launchTasks = tasks.slice(0, 4);
    const done = launchTasks.filter(([, , state]) => state === "done").length;

    return (
        <Paned>
            <Toolbar
                title="Destack launch"
                actions={
                    <>
                        <Present />
                        <span {...style.attrs(appStyles.action, styles.wide)}>
                            Agent edited just now
                        </span>
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
                    <span data-xray="Analytics" {...style.attrs(appStyles.property, styles.wide)}>
                        312 views
                    </span>
                    <span
                        data-xray="Jobs"
                        data-source="Calendar event"
                        {...style.attrs(appStyles.property)}
                    >
                        Demo · Wed 11:00
                    </span>
                    {properties.isRemixed ? (
                        <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                            3 days to launch
                        </span>
                    ) : undefined}
                </div>
                <p {...style.attrs(appText.body)}>
                    Ship to the waitlist on Thursday, then open signups on Friday. Pricing goes live
                    with the launch post.
                </p>
                <div {...style.attrs(styles.annotated)}>
                    {/* the launch tasks, a live view of the Tasks app */}
                    <div
                        data-xray="Database"
                        data-source="Tasks list"
                        {...style.attrs(styles.embed)}
                    >
                        <span {...style.attrs(appText.meta, styles.embedHead)}>
                            <b>Launch tasks</b>
                            <span {...style.attrs(styles.quiet)}>{done} of 4 done</span>
                            <Progress
                                value={done * 25}
                                aria-label="Tasks done"
                                xstyle={styles.progress}
                            />
                        </span>
                        {launchTasks.map(([id, title, state, owner, tint, due]) => (
                            <span {...style.attrs(appText.body, styles.embedRow)}>
                                <StateMark state={state} />
                                <span
                                    {...style.attrs(
                                        styles.embedTitle,
                                        state === "done" && styles.struck,
                                        isCommented(title) && styles.highlight,
                                    )}
                                >
                                    {title}
                                </span>
                                <span {...style.attrs(appText.meta, styles.quiet, styles.wide)}>
                                    {id}
                                </span>
                                <Face owner={owner} tint={tint} size="sm" />
                                <span {...style.attrs(appText.meta, styles.quiet)}>{due}</span>
                            </span>
                        ))}
                    </div>

                    {/* the agent's comment in the margin, a thread from Chat */}
                    <aside
                        data-xray="Messaging"
                        data-source="Chat thread"
                        {...style.attrs(styles.comment)}
                    >
                        <span {...style.attrs(styles.commentHead)}>
                            <Face owner="A" />
                            <b {...style.attrs(appText.meta)}>Agent</b>
                            <span {...style.attrs(appText.meta, styles.quiet)}>09:13</span>
                        </span>
                        <span {...style.attrs(appText.meta)}>
                            Ines booked the demo for Wednesday, so I linked it here and in DST-14.
                        </span>
                        <span {...style.attrs(appText.meta, styles.quiet)}>Reply</span>
                    </aside>
                </div>

                {/* the launch week, a live view of the Calendar app */}
                <div
                    data-xray="Database"
                    data-source="Calendar week"
                    {...style.attrs(styles.embed)}
                >
                    <span {...style.attrs(appText.meta, styles.embedHead)}>
                        <b>Launch week</b>
                        <span {...style.attrs(styles.quiet)}>Oct 20 – 24</span>
                    </span>
                    <span {...style.attrs(styles.days)}>
                        {DAYS.map((day, index) => (
                            <span {...style.attrs(styles.day)}>
                                <span {...style.attrs(appText.meta, styles.quiet)}>{day}</span>
                                {week
                                    .filter(
                                        ([weekday, , , , isBookable]) =>
                                            weekday === index && !isBookable,
                                    )
                                    .map(([, start, , title]) => (
                                        <span
                                            {...style.attrs(
                                                appText.meta,
                                                styles.dayEvent,
                                                title.includes("·")
                                                    ? styles.fromApps
                                                    : styles.personal,
                                            )}
                                        >
                                            {title}
                                            <span {...style.attrs(styles.quiet)}>
                                                {timeOf(start)}
                                            </span>
                                        </span>
                                    ))}
                            </span>
                        ))}
                    </span>
                </div>

                {/* the pricing draft, attached */}
                <span {...style.attrs(styles.attachment)}>
                    <Icon icon={fileText} />
                    <span>pricing-v3.pdf</span>
                    <span {...style.attrs(appText.meta, styles.quiet)}>2.4 MB</span>
                </span>
            </article>
        </Paned>
    );
}

/** Write an hour of the day as the calendar shows it, such as 9:30. */
function timeOf(hour: number): string {
    const minutes = Math.round((hour % 1) * 60);

    return `${String(Math.floor(hour))}:${String(minutes).padStart(2, "0")}`;
}

/** Report whether a launch item carries the agent's comment. */
function isCommented(item: string): boolean {
    return item.startsWith("Record");
}

/** Draw the launch tasks: your list grouped by state, your agent working through its share. */
function Tasks(properties: { isRemixed: boolean }) {
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
                <span data-xray="Database" {...style.attrs(appStyles.property)}>
                    <Icon icon={funnel} />
                    Due this week
                </span>
                <span data-xray="Jobs" {...style.attrs(appStyles.property, styles.wide)}>
                    <ServiceGlyph icon="sync" />
                    When done, post in #launch
                </span>
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        By priority
                    </span>
                ) : undefined}
            </Toolbar>
            <div data-xray="Database" {...style.attrs(styles.list)}>
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
                            .map(([id, title, , owner, tint, due]) => {
                                return (
                                    <div {...style.attrs(appText.body, styles.taskRow)}>
                                        <span {...style.attrs(styles.quiet, styles.taskId)}>
                                            {id}
                                        </span>
                                        <StateMark state={state} />
                                        <span
                                            {...style.attrs(
                                                styles.taskTitle,
                                                state === "done" && styles.struck,
                                            )}
                                        >
                                            {properties.isRemixed ? (
                                                <span
                                                    {...style.attrs(
                                                        styles.priority,
                                                        styles.changed,
                                                    )}
                                                >
                                                    {priorities[id] ?? "P3"}
                                                </span>
                                            ) : undefined}
                                            {title}
                                            {id === "DST-15" ? (
                                                <span
                                                    data-xray="Jobs"
                                                    {...style.attrs(styles.agentNote)}
                                                >
                                                    {" "}
                                                    · agent writing…
                                                </span>
                                            ) : undefined}
                                        </span>
                                        <LinkChip id={id} />
                                        <span {...style.attrs(styles.quiet)}>{due}</span>
                                        <Face owner={owner} tint={tint} />
                                    </div>
                                );
                            })}
                    </>
                ))}
            </div>
        </Paned>
    );
}

/** Draw the launch channel: its topic and members, a day of messages, and the composer. */
function Chat(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <div {...style.attrs(styles.channelHead)}>
                <b {...style.attrs(appText.heading)}># launch</b>
                <span {...style.attrs(appText.meta, styles.quiet, styles.wide)}>
                    Ship v4 on Friday
                </span>
                <span data-xray="Identity" {...style.attrs(styles.members)}>
                    <AvatarGroup aria-label="Members">
                        <Face owner="F" />
                        <Face owner="A" />
                        <Face owner="M" tint={palette.rust} />
                    </AvatarGroup>
                    <span {...style.attrs(appText.meta, styles.quiet)}>4</span>
                </span>
            </div>
            {/* the checklist your agent pins when asked */}
            {properties.isRemixed ? (
                <div {...style.attrs(appText.meta, styles.pinned, styles.changed)}>
                    <b>Pinned</b> Launch checklist · 4 of 6 done
                </div>
            ) : undefined}
            <div {...style.attrs(appText.meta, styles.divider)}>
                <Separator />
                Today
                <Separator />
            </div>
            <div {...style.attrs(styles.messages)}>
                {messages.slice(0, 3).map(([author, time, line], index) => (
                    <div
                        data-xray={index === 0 ? "Database" : undefined}
                        {...style.attrs(styles.message)}
                    >
                        <Face owner={author === "Agent" ? "A" : "F"} size="default" />
                        <div {...style.attrs(styles.messageBody)}>
                            <span {...style.attrs(appText.body)}>
                                <b>{author}</b> <span {...style.attrs(styles.quiet)}>{time}</span>
                            </span>
                            <span {...style.attrs(appText.body)}>{line}</span>
                            {index === 1 ? (
                                <span {...style.attrs(styles.linked)}>
                                    <StateMark state="started" />
                                    <span {...style.attrs(styles.linkedText)}>
                                        <b {...style.attrs(appText.meta)}>DST-14 Record the demo</b>
                                        <span {...style.attrs(appText.meta, styles.quiet)}>
                                            Tasks · due Wed 11:00
                                        </span>
                                    </span>
                                </span>
                            ) : undefined}
                            {index === 1 ? (
                                <span {...style.attrs(styles.inline)}>
                                    <span {...style.attrs(styles.reaction, styles.reactionOn)}>
                                        👍 2
                                    </span>
                                    <span {...style.attrs(styles.reaction)}>🚀 1</span>
                                </span>
                            ) : undefined}
                            {index === 2 ? (
                                <span
                                    data-xray="Messaging"
                                    {...style.attrs(appText.meta, styles.inline, styles.replies)}
                                >
                                    <AvatarGroup aria-label="Replied">
                                        <Face owner="F" />
                                    </AvatarGroup>
                                    3 replies
                                    <span {...style.attrs(styles.quiet)}>Last reply 09:52</span>
                                </span>
                            ) : undefined}
                        </div>
                    </div>
                ))}
            </div>
            <div {...style.attrs(styles.composer)}>
                <span {...style.attrs(appText.body, styles.quiet)}>Message #launch</span>
                <span {...style.attrs(styles.composerTools)}>
                    <Icon icon={paperclip} />
                    <Icon icon={at} />
                    <Icon icon={smiley} />
                    <span {...style.attrs(styles.sendButton)}>
                        <Icon icon={paperPlaneRight} />
                    </span>
                </span>
            </div>
            <span {...style.attrs(appText.meta, styles.typing)}>
                <Spinner aria-label="Agent is typing" />
                Agent is typing…
            </span>
        </Paned>
    );
}

/** Draw the press list as a table, its mail and calls read live from Mail and Calendar. */
function Press(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Press"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Grid</span>
                        <span {...style.attrs(appStyles.primary)}>New row</span>
                    </>
                }
            >
                <span data-xray="Database" {...style.attrs(appStyles.property)}>
                    <Icon icon={magnifyingGlass} />
                    Search 5 contacts
                </span>
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        Follow up after 3 days
                    </span>
                ) : undefined}
            </Toolbar>
            <Table data-xray="Database">
                <TableHeader>
                    <TableRow>
                        <TableHead>Name</TableHead>
                        <TableHead>Outlet</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead>
                            <span {...style.attrs(styles.joined)}>
                                <ServiceGlyph icon="mail" />
                                Last mail
                            </span>
                        </TableHead>
                        <TableHead xstyle={properties.isRemixed && styles.changed}>
                            {properties.isRemixed ? (
                                "Follow up"
                            ) : (
                                <span {...style.attrs(styles.joined)}>
                                    <ServiceGlyph icon="calendar" />
                                    Call
                                </span>
                            )}
                        </TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {press.map(([name, outlet, state, last, tint]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(styles.inline)}>
                                    <Face owner={name.slice(0, 1)} tint={tint} />
                                    {name}
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{outlet}</TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>
                                    <span
                                        {...style.attrs(
                                            styles.dot,
                                            state === "scheduled"
                                                ? styles.live
                                                : state === "replied"
                                                  ? styles.amber
                                                  : styles.idle,
                                        )}
                                    />
                                    {pressStates[state]}
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{last}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {callOf(state, properties.isRemixed)}
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Paned>
    );
}

/** Return a press row's last column: its follow-up once remixed, or else its call's time. */
function callOf(state: PressState, isRemixed: boolean): string {
    // follow up on the pitched once remixed
    if (isRemixed) {
        return state === "pitched" ? "Due tomorrow" : "–";
    }

    // read the call from the demo booking
    return state === "scheduled" ? "Wed 11:00" : "–";
}

/** The name each press state shows. */
const pressStates: Readonly<Record<PressState, string>> = {
    pitched: "Pitched",
    replied: "Replied",
    scheduled: "Call booked",
};

/** Draw the launch plan as a board of notes, each task's note live from Tasks. */
function Plan(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Launch plan"
                actions={
                    <>
                        <Present />
                        <span {...style.attrs(appStyles.primary)}>Share</span>
                    </>
                }
            >
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        Tasks drawn as arrows
                    </span>
                ) : undefined}
            </Toolbar>
            <div data-xray="Database" {...style.attrs(styles.board)}>
                {board.map(([column, notes]) => (
                    <div {...style.attrs(styles.boardColumn)}>
                        <span {...style.attrs(appText.meta, styles.quiet)}>{column}</span>
                        {notes.map((note) => {
                            const task = tasks.find(([id]) => id === note);

                            return task === undefined ? (
                                <span {...style.attrs(appText.body, styles.sticky)}>{note}</span>
                            ) : (
                                <span
                                    {...style.attrs(appText.body, styles.sticky, styles.stickyTask)}
                                >
                                    <span {...style.attrs(styles.inline)}>
                                        <StateMark state={task[2]} />
                                        <b {...style.attrs(appText.meta)}>{task[0]}</b>
                                    </span>
                                    {task[1]}
                                    <span {...style.attrs(appText.meta, styles.quiet)}>
                                        Tasks · due {task[5]}
                                    </span>
                                </span>
                            );
                        })}
                    </div>
                ))}
            </div>
        </Paned>
    );
}

/** Draw the workflows of your space and their runs, a new one first once your agent adds it. */
function Flows(properties: { change: Change; isRemixed: boolean }) {
    // list your agent's workflow first once it adds it
    const isAutomated = () => properties.change === "automated";

    return (
        <Paned>
            <Toolbar
                title="Workflows"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Runs</span>
                        <span {...style.attrs(appStyles.primary)}>New workflow</span>
                    </>
                }
            >
                <span data-xray="Jobs" {...style.attrs(appStyles.property)}>
                    {isAutomated() ? "5 workflows" : "4 workflows"}
                </span>
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        Alert me in Chat when a run fails
                    </span>
                ) : undefined}
            </Toolbar>
            <Table data-xray="Jobs">
                <TableHeader>
                    <TableRow>
                        <TableHead>Workflow</TableHead>
                        <TableHead>Starts</TableHead>
                        <TableHead>Runs</TableHead>
                        <TableHead>Last run</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {(isAutomated() ? askedWorkflows : workflows).map(
                        ([name, trigger, runs, last]) => (
                            <TableRow
                                xstyle={isAutomated() && name === ASKED_WORKFLOW && styles.changed}
                            >
                                <TableCell xstyle={appText.body}>
                                    <span {...style.attrs(styles.inline)}>
                                        <ServiceGlyph icon="sync" />
                                        <b {...style.attrs(styles.code)}>{name}</b>
                                    </span>
                                </TableCell>
                                <TableCell xstyle={[appText.body, styles.quiet, styles.code]}>
                                    {trigger}
                                </TableCell>
                                <TableCell xstyle={[appText.body, styles.quiet]}>{runs}</TableCell>
                                <TableCell xstyle={[appText.body, styles.quiet]}>{last}</TableCell>
                            </TableRow>
                        ),
                    )}
                </TableBody>
            </Table>
            <span {...style.attrs(appText.meta, styles.inline, styles.quiet)}>
                <Icon icon={dotsThree} />
                Each step recorded once, retried on failure
            </span>
        </Paned>
    );
}

/** Draw your week: the events your apps put there and the free times your booking page offers. */
function Week(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="October 20 – 24"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Day</span>
                        <span {...style.attrs(appStyles.action, appStyles.actionStrong)}>Week</span>
                        <span {...style.attrs(appStyles.action)}>Month</span>
                        <span data-xray="Messaging" {...style.attrs(appStyles.primary)}>
                            Share booking link
                        </span>
                    </>
                }
            >
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        Focus 9–11 every weekday
                    </span>
                ) : undefined}
            </Toolbar>
            <div data-xray="Database" {...style.attrs(appText.meta, styles.week)}>
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
                        <EventTitle title={title} />
                    </span>
                ))}
            </div>
        </Paned>
    );
}

/** Draw a task's link to an object in another app, with that object's live value. */
function LinkChip(properties: { id: string }) {
    const link = () => present(taskLinks[properties.id], "task link");

    return (
        <span {...style.attrs(appStyles.property, styles.link)}>
            <ServiceGlyph icon={link()[0]} />
            {link()[1]}
        </span>
    );
}

/** Draw an event's name, and the task or mail it comes from in another app. */
function EventTitle(properties: { title: string }) {
    const [name, link] = properties.title.split(" · ");

    return (
        <>
            <span>{name}</span>
            {link === undefined ? undefined : <EventLink link={link} />}
        </>
    );
}

/** Draw the task or mail an event comes from. */
function EventLink(properties: { link: string }) {
    return (
        <span {...style.attrs(styles.eventLink)}>
            <ServiceGlyph icon={properties.link.startsWith("DST-") ? "tasks" : "mail"} />
            {properties.link}
        </span>
    );
}

/** Return the grid row a time of day starts at in the week. */
function rowOf(hour: number): number {
    return 3 + (hour - (HOURS[0] ?? 0)) * 2;
}

/** Draw the launch mail. */
function Mail(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Destack opens Friday"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action, styles.wide)}>
                            Agent drafted 09:40
                        </span>
                        <span data-xray="Jobs" {...style.attrs(appStyles.primary)}>
                            Send Fri 09:00
                        </span>
                    </>
                }
            >
                <span data-xray="Database" {...style.attrs(appStyles.property, styles.on)}>
                    <Icon icon={at} />
                    To Signups · Waiting
                </span>
                <span {...style.attrs(appStyles.property)}>1,840 people</span>
                <span data-xray="Messaging" {...style.attrs(appStyles.property)}>
                    From hello@destack.sh
                </span>
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        P.S. with referral link
                    </span>
                ) : undefined}
            </Toolbar>
            {/* the agent's send, held for your approval until it goes */}
            <div {...style.attrs(styles.approval)}>
                <Face owner="A" />
                <span {...style.attrs(appText.meta)}>
                    Claude wants to send this to 1,840 people
                </span>
                <span {...style.attrs(styles.approvalButtons)}>
                    <span {...style.attrs(appStyles.action)}>Decline</span>
                    <span {...style.attrs(appStyles.primary)}>Approve</span>
                </span>
            </div>
            <article {...style.attrs(styles.document)}>
                <p {...style.attrs(appText.body)}>Hi Maya,</p>
                <p {...style.attrs(appText.body)}>
                    Destack opens on Friday, and you're in. Your invite link works from 09:00, and
                    early pricing holds for everyone on the waitlist.
                </p>
                <p {...style.attrs(appText.body)}>See you there, Florian</p>
                <aside {...style.attrs(styles.note)}>
                    <span {...style.attrs(styles.commentHead)}>
                        <Face owner="A" />
                        <b {...style.attrs(appText.meta)}>Agent</b>
                        <span {...style.attrs(appText.meta, styles.quiet)}>09:40</span>
                    </span>
                    <span {...style.attrs(appText.meta)}>
                        Reads Signups at send time, so Thursday's joiners get it too.
                    </span>
                </aside>

                {/* the recipients, a live view of the Signups table */}
                <div data-xray="Database" data-source="Tables view" {...style.attrs(styles.embed)}>
                    <span {...style.attrs(appText.meta, styles.embedHead)}>
                        <b>Signups</b>
                        <span {...style.attrs(styles.quiet)}>Waiting · 1,840</span>
                    </span>
                    {signups
                        .filter(([, , , state]) => state === "waiting")
                        .map(([email, source, joined]) => (
                            <span {...style.attrs(appText.body, styles.embedRow, styles.recipient)}>
                                <span {...style.attrs(styles.embedTitle)}>{email}</span>
                                <span {...style.attrs(appText.meta, styles.quiet)}>{source}</span>
                                <span {...style.attrs(appText.meta, styles.quiet)}>{joined}</span>
                            </span>
                        ))}
                    <span {...style.attrs(appText.meta, styles.quiet, styles.more)}>
                        and 1,836 more
                    </span>
                </div>
            </article>
        </Paned>
    );
}

/** Draw your waitlist's signups. */
function Signups(properties: { change: Change; isRemixed: boolean }) {
    // show your agent's board as a view of the signups once it builds it
    const isBoarded = () => properties.change !== "start";

    return (
        <Paned>
            <Toolbar
                title="Signups"
                actions={
                    <>
                        <span data-xray="Analytics" {...style.attrs(appStyles.action)}>
                            24 joined today
                        </span>
                        <span {...style.attrs(appStyles.primary)}>Invite 400</span>
                    </>
                }
            >
                <span data-xray="Database" {...style.attrs(appStyles.property)}>
                    <Icon icon={magnifyingGlass} />
                    Search 1,840 signups
                </span>
                <span {...style.attrs(appStyles.property, styles.on)}>
                    <Icon icon={funnel} />
                    Waiting
                </span>
                <span {...style.attrs(appStyles.property)}>Source: any</span>
                {isBoarded() ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        Launch askers · 9 · shared with Jonas
                    </span>
                ) : undefined}
            </Toolbar>
            <Table data-xray="Database">
                <TableHeader>
                    <TableRow>
                        <TableHead xstyle={styles.select}>
                            <Checkbox aria-label="Select all" indeterminate />
                        </TableHead>
                        <TableHead>Email</TableHead>
                        <TableHead>Source</TableHead>
                        <TableHead xstyle={properties.isRemixed && styles.changed}>
                            {properties.isRemixed ? (
                                "Referred by"
                            ) : (
                                <span {...style.attrs(styles.joined)}>
                                    <ServiceGlyph icon="chat" />
                                    Messages
                                </span>
                            )}
                        </TableHead>
                        <TableHead>
                            <span {...style.attrs(styles.joined)}>
                                <ServiceGlyph icon="mail" />
                                Invite
                            </span>
                        </TableHead>
                        <TableHead>Status</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {signups.map(([email, source, , state], index) => (
                        <TableRow>
                            <TableCell xstyle={styles.select}>
                                <Checkbox checked={index < 4} aria-label={email} />
                            </TableCell>
                            <TableCell xstyle={appText.body}>{email}</TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>{source}</span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {properties.isRemixed
                                    ? (referrals[email] ?? "–")
                                    : (signupMessages[email] ?? "–")}
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {signupInvites[email] ?? (index < 4 ? "Batch 1" : "Next batch")}
                            </TableCell>
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

/** Draw your launch tracker as an app of your space, its dates from Calendar. */
function Tracker(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Launch tracker"
                actions={
                    <>
                        <Present />
                        <span data-xray="Analytics" {...style.attrs(appStyles.action, styles.wide)}>
                            24 signups today
                        </span>
                        <span data-xray="Messaging" {...style.attrs(appStyles.primary)}>
                            Email the waitlist
                        </span>
                    </>
                }
            >
                <span {...style.attrs(appStyles.property, styles.on)}>3 days to go</span>
                <span data-xray="Jobs" {...style.attrs(appStyles.property)}>
                    Reminder · Fri 09:00
                </span>
            </Toolbar>
            <Table data-xray="Database">
                <TableHeader>
                    <TableRow>
                        <TableHead>Milestone</TableHead>
                        <TableHead>
                            <span {...style.attrs(styles.joined)}>
                                <ServiceGlyph icon="calendar" />
                                Due
                            </span>
                        </TableHead>
                        <TableHead xstyle={properties.isRemixed && styles.changed}>
                            {properties.isRemixed ? (
                                <span {...style.attrs(styles.joined)}>
                                    <ServiceGlyph icon="tasks" />
                                    Task
                                </span>
                            ) : (
                                "Status"
                            )}
                        </TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {milestones.map(([title, due, isDone], index) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>{title}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{due}</TableCell>
                            <TableCell xstyle={appText.body}>
                                {properties.isRemixed ? (
                                    <span {...style.attrs(appStyles.property)}>
                                        DST-{12 + index}
                                    </span>
                                ) : (
                                    <span {...style.attrs(appStyles.property)}>
                                        <span
                                            {...style.attrs(
                                                styles.dot,
                                                isDone ? styles.live : styles.amber,
                                            )}
                                        />
                                        {isDone ? "Done" : "Open"}
                                    </span>
                                )}
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Paned>
    );
}

/** Draw your changelog as an app of your space, public to anyone you share it with. */
function Changelog(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Changelog"
                actions={<span {...style.attrs(appStyles.primary)}>New release</span>}
            />
            <Table data-xray="Database">
                <TableHeader>
                    <TableRow>
                        <TableHead>Version</TableHead>
                        <TableHead>Release</TableHead>
                        <TableHead>Date</TableHead>
                        <TableHead xstyle={properties.isRemixed && styles.changed}>
                            {properties.isRemixed ? (
                                <span {...style.attrs(styles.joined)}>
                                    <ServiceGlyph icon="chat" />
                                    Posted
                                </span>
                            ) : (
                                <span {...style.attrs(styles.joined)}>
                                    <ServiceGlyph icon="mail" />
                                    Sent
                                </span>
                            )}
                        </TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {releases.map(([version, date, title]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>v{version}</span>
                            </TableCell>
                            <TableCell xstyle={appText.body}>{title}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{date}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {properties.isRemixed ? "#launch" : "1,840"}
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Paned>
    );
}

/** The styles of your space's apps, drawn at the size of a real window. */
const styles = style.create({
    wide: {
        display: { default: "inline-flex", [NARROW]: "none" },
    },
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
    live: {
        backgroundColor: palette.green,
    },
    embed: {
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        overflow: "hidden",
    },
    embedHead: {
        alignItems: "center",
        backgroundColor: color.muted,
        display: "flex",
        gap: "0.5rem",
        paddingBlock: "0.375rem",
        paddingInline: "0.75rem",
    },
    embedRow: {
        alignItems: "center",
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        columnGap: "0.625rem",
        display: "grid",
        gridTemplateColumns: {
            default: "auto minmax(0, 1fr) auto auto 3rem",
            [NARROW]: "auto minmax(0, 1fr) auto 3rem",
        },
        paddingBlock: "0.375rem",
        paddingInline: "0.75rem",
    },
    recipient: {
        gridTemplateColumns: "minmax(0, 1fr) 6rem 5rem",
    },
    more: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        paddingBlock: "0.375rem",
        paddingInline: "0.75rem",
    },
    embedTitle: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    days: {
        display: "grid",
        gridTemplateColumns: "repeat(5, minmax(0, 1fr))",
    },
    day: {
        alignContent: "start",
        borderInlineStartColor: color.border,
        borderInlineStartStyle: { default: "solid", ":first-child": "none" },
        borderInlineStartWidth: stroke.border,
        display: "grid",
        gap: "0.25rem",
        minHeight: "4.5rem",
        paddingBlock: "0.375rem",
        paddingInline: "0.5rem",
    },
    dayEvent: {
        borderRadius: "4px",
        display: "grid",
        overflow: "hidden",
        paddingBlock: "0.125rem",
        paddingInline: "0.375rem",
        whiteSpace: "nowrap",
    },
    attachment: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "inline-flex",
        fontSize: "0.8125rem",
        gap: "0.5rem",
        justifySelf: "start",
        paddingBlock: "0.3125rem",
        paddingInline: "0.625rem",
    },
    struck: {
        color: color.mutedForeground,
        textDecorationLine: "line-through",
    },
    annotated: {
        alignItems: "start",
        display: "grid",
        gap: "1.25rem",
        gridTemplateColumns: { default: "minmax(0, 1fr) 12rem", [NARROW]: "minmax(0, 1fr)" },
    },
    highlight: {
        backgroundColor: `color-mix(in srgb, ${palette.amber} 28%, transparent)`,
        borderRadius: "2px",
        boxShadow: `0 2px 0 ${palette.amber}`,
    },
    comment: {
        backgroundColor: color.card,
        borderRadius: "8px",
        boxShadow: `0 0 0 1px ${color.border}, 0 2px 8px color-mix(in srgb, ${palette.ink} 8%, transparent)`,
        display: { default: "grid", [NARROW]: "none" },
        gap: "0.25rem",
        marginTop: "1.75rem",
        paddingBlock: "0.625rem",
        paddingInline: "0.75rem",
    },
    note: {
        backgroundColor: color.card,
        borderRadius: "8px",
        boxShadow: `0 0 0 1px ${color.border}`,
        display: "grid",
        gap: "0.25rem",
        justifySelf: "start",
        maxWidth: "24rem",
        paddingBlock: "0.625rem",
        paddingInline: "0.75rem",
    },
    commentHead: {
        alignItems: "center",
        display: "flex",
        gap: "0.375rem",
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
    pinned: {
        borderRadius: "6px",
        display: "flex",
        gap: "0.5rem",
        paddingBlock: "0.5rem",
        paddingInline: "0.75rem",
    },
    priority: {
        borderRadius: "4px",
        fontSize: "0.6875rem",
        fontWeight: 600,
        marginInlineEnd: "0.375rem",
        paddingInline: "0.25rem",
    },
    chart: {
        alignItems: "end",
        borderRadius: "6px",
        display: "flex",
        gap: "0.375rem",
        height: "5rem",
        padding: "0.5rem",
    },
    bar: {
        borderRadius: "3px",
        flexGrow: 1,
    },
    idle: {
        backgroundColor: color.mutedForeground,
    },
    board: {
        display: "grid",
        gap: space[3],
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        minHeight: 0,
        overflow: "hidden",
    },
    boardColumn: {
        alignContent: "start",
        display: "grid",
        gap: space[2],
    },
    sticky: {
        backgroundColor: `color-mix(in srgb, ${palette.amber} 22%, ${color.background})`,
        borderRadius: radius["2"],
        boxShadow: `0 1px 2px color-mix(in srgb, ${color.foreground} 12%, transparent)`,
        display: "grid",
        gap: space[1],
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    stickyTask: {
        backgroundColor: color.background,
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: stroke.border,
    },
    changed: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 14%, transparent)`,
        transitionDuration: motion.durationShort,
        transitionProperty: "background-color",
        transitionTimingFunction: motion.easingStandard,
    },
    approval: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, ${palette.amber} 14%, transparent)`,
        borderRadius: radius[2],
        display: "flex",
        gap: space[2],
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    approvalButtons: {
        display: "flex",
        gap: space[2],
        marginLeft: "auto",
    },
    taskId: {
        fontVariantNumeric: "tabular-nums",
    },
    channelHead: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: space[3],
        paddingBottom: space[3],
    },
    members: {
        alignItems: "center",
        display: "flex",
        gap: space[2],
        marginLeft: "auto",
    },
    linked: {
        alignItems: "center",
        borderColor: color.border,
        borderInlineStartColor: palette.amber,
        borderInlineStartWidth: "3px",
        borderRadius: radius[2],
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "flex",
        gap: space[2],
        marginTop: space[1],
        maxWidth: "20rem",
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    linkedText: {
        display: "grid",
    },
    reaction: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: space[1],
        paddingBlock: "0.0625rem",
        paddingInline: space[2],
    },
    reactionOn: {
        backgroundColor: `color-mix(in srgb, ${color.primary} 10%, transparent)`,
        borderColor: `color-mix(in srgb, ${color.primary} 45%, transparent)`,
    },
    composer: {
        borderColor: color.border,
        borderRadius: radius[3],
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        gap: space[2],
        paddingBlock: space[3],
        paddingInline: space[3],
    },
    composerTools: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        gap: space[3],
    },
    typing: {
        alignItems: "center",
        color: palette.violet,
        display: "flex",
        gap: space[2],
        marginTop: `calc(${space[2]} * -1)`,
        paddingInline: space[3],
    },
    sendButton: {
        alignItems: "center",
        backgroundColor: color.foreground,
        borderRadius: radius[2],
        color: color.background,
        display: "inline-flex",
        height: "1.75rem",
        justifyContent: "center",
        width: "1.75rem",
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
    replies: {
        color: color.primary,
        fontWeight: weight.semibold,
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
    link: {
        backgroundColor: "transparent",
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
    },
    joined: {
        alignItems: "center",
        display: "inline-flex",
        gap: space[1],
    },
    eventLink: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        fontWeight: weight.regular,
        gap: space[1],
        marginTop: "2px",
    },
});
