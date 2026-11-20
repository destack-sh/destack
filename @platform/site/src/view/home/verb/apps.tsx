import { Icon } from "@destack/ui/icon";
import archive from "@destack/icon/phosphor/archive";
import arrowBendDoubleUpLeft from "@destack/icon/phosphor/arrow-bend-double-up-left";
import arrowBendUpLeft from "@destack/icon/phosphor/arrow-bend-up-left";
import arrowBendUpRight from "@destack/icon/phosphor/arrow-bend-up-right";
import flag from "@destack/icon/phosphor/flag";
import folderSimple from "@destack/icon/phosphor/folder-simple";
import funnelSimple from "@destack/icon/phosphor/funnel-simple";
import notePencil from "@destack/icon/phosphor/note-pencil";
import trash from "@destack/icon/phosphor/trash";
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
    commitments,
    type CommitmentState,
    mails,
    messages,
    people,
    type ReplyState,
    runs,
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

/** The hour your week opens at. */
const OPEN = 9;

/** The hour your week closes at. */
const CLOSE = 17;

/** The hours your week labels, each on its rule after the first. */
const HOURS = Array.from({ length: CLOSE - OPEN - 1 }, (_, index) => OPEN + 1 + index);

/** The time now on today, Monday, as an hour of the day: 10:36. */
const NOW = 10.6;

/** The weekdays your week shows, with their dates. */
const DAYS = ["Mon 20", "Tue 21", "Wed 22", "Thu 23", "Fri 24"];

/** The task states in the order the list groups them, with their names and marks. */
const STATES: readonly (readonly [state: TaskState, name: string])[] = [
    ["started", "In progress"],
    ["review", "In review"],
    ["todo", "Todo"],
    ["done", "Done"],
];

/** The tasks you do for Ines, which her page lists. */
const INES_TASKS: ReadonlySet<string> = new Set(["T-12", "T-14", "T-13"]);

/** The object in another app each task points at, by the app's icon and the object's live value. */
const taskLinks: Readonly<Record<string, readonly [icon: string, text: string]>> = {
    "T-12": ["source", "PR #48 · merged"],
    "T-14": ["calendar", "Review · Wed 11:00"],
    "T-16": ["mail", "29 of 31 matched"],
    "T-13": ["table", "3 clients"],
    "T-15": ["mail", "Saturday?"],
    "T-17": ["calendar", "Thu 08:30"],
    "T-18": ["table", "Payments · Oct 12"],
    "T-19": ["table", "4 unused"],
};

/** The last you heard from the person behind each commitment, by person. */
const lastHeard: Readonly<Record<string, string>> = {
    "Ines Ferreira": "Mail · 10:14",
    "Jonas Weber": "Mail · 09:31",
    "Mia Sørensen": "Mail · 08:52",
    "Kai Brandt": "Mail · Oct 19",
};

/** The meeting coming up with the person behind each commitment, by person. */
const nextMeeting: Readonly<Record<string, string>> = {
    "Ines Ferreira": "Wed 11:00",
    "Jonas Weber": "Tue 11:30",
    "Mia Sørensen": "Fri 13:00",
};

/** Draw a section of your space by its row's job, changed by a call when one lands on it. */
export function sectionOf(job: Job, source: Source, isRemixed: boolean): JSX.Element {
    if (job === "documents") {
        return <Person isRemixed={isRemixed} />;
    } else if (job === "tables") {
        return <People isRemixed={isRemixed} />;
    } else if (job === "tasks") {
        return <Tasks isRemixed={isRemixed} />;
    } else if (job === "calendar") {
        return <Week isRemixed={isRemixed} />;
    } else if (job === "chat") {
        return <Chat isRemixed={isRemixed} />;
    } else if (job === "mail") {
        return <Mail isRemixed={isRemixed} />;
    } else if (job === "canvas") {
        return <Board isRemixed={isRemixed} />;
    } else if (job === "workflows") {
        return <Flows isRemixed={isRemixed} />;
    } else if (job === "builder") {
        return <Commitments isRemixed={isRemixed} />;
    } else if (job === "tracker") {
        return <Tracker isRemixed={isRemixed} />;
    } else if (job === "changelog") {
        return <Changelog isRemixed={isRemixed} />;
    }

    return <Connection source={source} isRemixed={isRemixed} />;
}

/** A change your agent makes to an app of yours: what you ask for, and what it did. */
type Remix = { ask: string; done: string };

/** Return the change your agent makes to the app that does a job. */
export function remixOf(job: Job, source: Source): Remix {
    if (job === "documents") {
        return { ask: "Show when I last talked to her", done: "Added Last talked to her page" };
    } else if (job === "tables") {
        return { ask: "Flag anyone I owe a reply for 3 days", done: "Added Owe a reply to People" };
    } else if (job === "tasks") {
        return { ask: "Add a priority to every task", done: "Added Priority to Tasks" };
    } else if (job === "calendar") {
        return { ask: "Keep my mornings free for focus", done: "Blocked 9–11 every weekday" };
    } else if (job === "chat") {
        return { ask: "Pin what Ines and I agreed", done: "Pinned the agreed points" };
    } else if (job === "mail") {
        return { ask: "Attach the estimate when I reply", done: "Attached estimate-phase-2.pdf" };
    } else if (job === "canvas") {
        return { ask: "Draw what waits on what", done: "Drew the waiting arrows" };
    } else if (job === "workflows") {
        return { ask: "Tell me in Chat when a run fails", done: "Added failure alerts" };
    } else if (job === "builder") {
        return { ask: "Show when I last heard from each", done: "Added Last heard to Monday" };
    } else if (job === "tracker") {
        return { ask: "Link each milestone to its task", done: "Linked milestones to Tasks" };
    } else if (job === "changelog") {
        return { ask: "Post each release to Ines's channel", done: "Added Posted to Changelog" };
    }

    return { ask: `Chart my ${source.data} by week`, done: `Charted ${source.table} by week` };
}

/** The priority your agent gives each task once you ask it to add one. */
const priorities: Readonly<Record<string, string>> = {
    "T-14": "P1",
    "T-16": "P1",
    "T-13": "P1",
    "T-15": "P2",
    "T-17": "P2",
    "T-12": "P3",
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
            {/* show the table's headline figures */}
            <div {...style.attrs(styles.figures)}>
                {properties.source.figures.map(([label, value]) => (
                    <span {...style.attrs(styles.figure)}>
                        <span {...style.attrs(appText.meta, styles.quiet)}>{label}</span>
                        <b {...style.attrs(styles.figureValue)}>{value}</b>
                    </span>
                ))}
            </div>

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
function Toolbar(properties: { title: string; actions: JSX.Element; children: JSX.Element }) {
    return (
        <>
            <AppHeader title={properties.title}>{properties.actions}</AppHeader>
            <div {...style.attrs(appStyles.properties)}>{properties.children}</div>
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

/** Draw your page on Ines: notes you and your agent keep, made of your other apps' parts. */
function Person(properties: { isRemixed: boolean }) {
    // count her tasks done
    const herTasks = tasks.filter(([id]) => INES_TASKS.has(id));
    const done = herTasks.filter(([, , state]) => state === "done").length;

    return (
        <Paned>
            <AppHeader title="Ines Ferreira">
                <Present />
                <span {...style.attrs(appStyles.action, styles.wide)}>Agent edited just now</span>
                <span {...style.attrs(appStyles.primary)}>Share</span>
            </AppHeader>
            <article {...style.attrs(styles.document)}>
                <div {...style.attrs(appStyles.properties)}>
                    <span {...style.attrs(appStyles.property)}>
                        <StateMark state="started" />
                        Client
                    </span>
                    <span {...style.attrs(appStyles.property)}>Booking app</span>
                    <span data-xray="Analytics" {...style.attrs(appStyles.property, styles.wide)}>
                        14 mails this month
                    </span>
                    <span
                        data-xray="Jobs"
                        data-source="Calendar event"
                        {...style.attrs(appStyles.property)}
                    >
                        Review · Wed 11:00
                    </span>
                    {properties.isRemixed ? (
                        <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                            Last talked Mon 11:30
                        </span>
                    ) : undefined}
                </div>
                <p {...style.attrs(appText.body)}>
                    Runs a physio practice with three rooms. Phase 2 adds card payments and moves
                    the app to her own domain before December.
                </p>
                <div {...style.attrs(styles.annotated)}>
                    {/* her tasks, a live view of the Tasks app */}
                    <div
                        data-xray="Database"
                        data-source="Tasks list"
                        {...style.attrs(styles.embed)}
                    >
                        <span {...style.attrs(appText.meta, styles.embedHead)}>
                            <b>Tasks for Ines</b>
                            <span {...style.attrs(styles.quiet)}>
                                {done} of {herTasks.length} done
                            </span>
                            <Progress
                                value={(done * 100) / herTasks.length}
                                aria-label="Tasks done"
                                xstyle={styles.progress}
                            />
                        </span>
                        {herTasks.map(([id, title, state, owner, tint, due]) => (
                            <span {...style.attrs(appText.body, styles.embedRow)}>
                                <StateMark state={state} />
                                <span
                                    {...style.attrs(
                                        styles.clip,
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
                            She asked about card payments, so I linked it here and in T-14.
                        </span>
                        <span {...style.attrs(appText.meta, styles.quiet)}>Reply</span>
                    </aside>
                </div>

                {/* this week, a live view of the Calendar app */}
                <div
                    data-xray="Database"
                    data-source="Calendar week"
                    {...style.attrs(styles.embed)}
                >
                    <span {...style.attrs(appText.meta, styles.embedHead)}>
                        <b>This week</b>
                        <span {...style.attrs(styles.quiet)}>Oct 20 – 24</span>
                    </span>
                    <span {...style.attrs(styles.days)}>
                        {DAYS.map((day, index) => (
                            <span {...style.attrs(styles.day)}>
                                <span {...style.attrs(appText.meta, styles.quiet)}>{day}</span>
                                {week
                                    .filter(
                                        ([weekday, , , title]) =>
                                            weekday === index && title.includes(" · "),
                                    )
                                    .map(([, start, , title]) => (
                                        <span
                                            {...style.attrs(
                                                appText.meta,
                                                styles.dayEvent,
                                                styles.fromApps,
                                            )}
                                        >
                                            <span {...style.attrs(styles.clip)}>
                                                {title.split(" · ")[0]}
                                            </span>
                                            <span {...style.attrs(styles.quiet)}>
                                                {timeOf(start)}
                                            </span>
                                        </span>
                                    ))}
                            </span>
                        ))}
                    </span>
                </div>

                {/* the estimate draft, attached */}
                <span {...style.attrs(styles.attachment)}>
                    <Icon icon={fileText} />
                    <span>estimate-phase-2.pdf</span>
                    <span {...style.attrs(appText.meta, styles.quiet)}>180 KB</span>
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

/** Report whether a task carries the agent's comment. */
function isCommented(item: string): boolean {
    return item.startsWith("Send Ines");
}

/** Draw your tasks: your list grouped by state, your agent working through its share. */
function Tasks(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Tasks"
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
                    When done, tell whoever asked
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
                                                styles.clip,
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
                                            {id === "T-15" ? (
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

/** Draw the channel you share with Ines: its members, a day of messages, and the composer. */
function Chat(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <div {...style.attrs(styles.channelHead)}>
                <b {...style.attrs(appText.heading)}># booking-app</b>
                <span {...style.attrs(appText.meta, styles.quiet, styles.wide)}>
                    Phase 2 before December
                </span>
                <span data-xray="Identity" {...style.attrs(styles.members)}>
                    <AvatarGroup aria-label="Members">
                        <Face owner="F" />
                        <Face owner="A" />
                        <Face owner="I" tint={palette.berry} />
                    </AvatarGroup>
                    <span {...style.attrs(appText.meta, styles.quiet)}>3</span>
                </span>
            </div>
            {/* the checklist your agent pins when asked */}
            {properties.isRemixed ? (
                <div {...style.attrs(appText.meta, styles.pinned, styles.changed)}>
                    <b>Pinned</b> Agreed · cards at booking, her domain by December
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
                        <Face
                            owner={author === "Agent" ? "A" : author.slice(0, 1)}
                            tint={author === "Ines" ? palette.berry : palette.teal}
                            size="default"
                        />
                        <div {...style.attrs(styles.messageBody)}>
                            <span {...style.attrs(appText.body)}>
                                <b>{author}</b> <span {...style.attrs(styles.quiet)}>{time}</span>
                            </span>
                            <span {...style.attrs(appText.body)}>{line}</span>
                            {index === 1 ? (
                                <span {...style.attrs(styles.linked)}>
                                    <StateMark state="started" />
                                    <span {...style.attrs(styles.linkedText)}>
                                        <b {...style.attrs(appText.meta)}>
                                            T-14 Send Ines the estimate
                                        </b>
                                        <span {...style.attrs(appText.meta, styles.quiet)}>
                                            Tasks · due Wed 11:00
                                        </span>
                                    </span>
                                </span>
                            ) : undefined}
                            {index === 1 ? (
                                <span {...style.attrs(styles.inline)}>
                                    <span {...style.attrs(styles.reaction, styles.reactionOn)}>
                                        👍 1
                                    </span>
                                    <span {...style.attrs(styles.reaction)}>🙏 1</span>
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
                <span {...style.attrs(appText.body, styles.quiet)}>Message #booking-app</span>
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

/** Draw the people you deal with, their mail and meetings read live from Mail and Calendar. */
function People(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="People"
                actions={
                    <>
                        <span {...style.attrs(appStyles.action)}>Grid</span>
                        <span {...style.attrs(appStyles.primary)}>New row</span>
                    </>
                }
            >
                <span data-xray="Database" {...style.attrs(appStyles.property)}>
                    <Icon icon={magnifyingGlass} />
                    Search {people.length} people
                </span>
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        Owe a reply after 3 days
                    </span>
                ) : undefined}
            </Toolbar>
            <Table data-xray="Database">
                <TableHeader>
                    <TableRow>
                        <TableHead>Name</TableHead>
                        <TableHead>Who</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead>
                            <span {...style.attrs(styles.joined)}>
                                <ServiceGlyph icon="mail" />
                                Last mail
                            </span>
                        </TableHead>
                        <TableHead xstyle={properties.isRemixed && styles.changed}>
                            {properties.isRemixed ? (
                                "Owe a reply"
                            ) : (
                                <span {...style.attrs(styles.joined)}>
                                    <ServiceGlyph icon="calendar" />
                                    Next
                                </span>
                            )}
                        </TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {people.map(([name, relation, state, last, tint]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(styles.inline)}>
                                    <Face owner={name.slice(0, 1)} tint={tint} />
                                    {name}
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{relation}</TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>
                                    <span
                                        {...style.attrs(
                                            styles.dot,
                                            state === "clear"
                                                ? styles.live
                                                : state === "waiting"
                                                  ? styles.idle
                                                  : styles.amber,
                                        )}
                                    />
                                    {replyStates[state]}
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{last}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {nextOf(name, state, properties.isRemixed)}
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Paned>
    );
}

/** Return a person's last column: a reply you owe once remixed, else your next meeting. */
function nextOf(name: string, state: ReplyState, isRemixed: boolean): string {
    // flag the replies you owe once remixed
    if (isRemixed) {
        return state === "owe" ? "Today" : "–";
    }

    // read the next meeting from Calendar
    return nextMeeting[name] ?? "–";
}

/** The name each reply state shows. */
const replyStates: Readonly<Record<ReplyState, string>> = {
    owe: "Reply owed",
    waiting: "Waiting on them",
    clear: "Up to date",
};

/** Draw your week as a board of notes, each task's note live from Tasks. */
function Board(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="This week"
                actions={
                    <>
                        <Present />
                        <span {...style.attrs(appStyles.primary)}>Share</span>
                    </>
                }
            >
                {properties.isRemixed ? (
                    <span {...style.attrs(appStyles.property, styles.on, styles.changed)}>
                        What waits on what, drawn as arrows
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

/** Draw the workflows of your space and their runs. */
function Flows(properties: { isRemixed: boolean }) {
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
                    {workflows.length} workflows
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
                    {workflows.map(([name, trigger, total, last]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(styles.inline)}>
                                    <ServiceGlyph icon="sync" />
                                    <b {...style.attrs(styles.code)}>{name}</b>
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet, styles.code]}>
                                {trigger}
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{total}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{last}</TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>

            {/* list the latest runs, each step recorded once */}
            <div {...style.attrs(styles.runs)}>
                <span {...style.attrs(appText.meta, styles.runsHead)}>
                    <b>Recent runs</b>
                    <span {...style.attrs(styles.quiet)}>
                        Each step recorded once, retried on failure
                    </span>
                </span>
                {runs.map(([workflow, subject, steps, duration, when]) => (
                    <span {...style.attrs(appText.meta, styles.run)}>
                        <span {...style.attrs(styles.dot, styles.live)} />
                        <b {...style.attrs(styles.code)}>{workflow}</b>
                        <span {...style.attrs(styles.clip)}>{subject}</span>
                        <span {...style.attrs(styles.quiet)}>
                            {steps} {steps === 1 ? "step" : "steps"} · {duration}
                        </span>
                        <span {...style.attrs(styles.quiet, styles.runWhen)}>{when}</span>
                    </span>
                ))}
            </div>
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
                {/* name each day, today's date marked */}
                {DAYS.map((day, index) => {
                    const [name, date] = day.split(" ");

                    return (
                        <span
                            style={{ "grid-column": String(index + 2) }}
                            {...style.attrs(styles.dayName)}
                        >
                            {name}
                            <b {...style.attrs(styles.date, index === 0 && styles.today)}>{date}</b>
                        </span>
                    );
                })}

                {/* run the month's books across the whole week */}
                <span {...style.attrs(styles.allDayName)}>all day</span>
                <span {...style.attrs(styles.allDay)}>Books close Friday</span>

                {/* rule the hours, and part the days */}
                <span
                    style={{ "grid-row": `${rowOf(OPEN)} / ${rowOf(CLOSE)}` }}
                    {...style.attrs(styles.grid)}
                />
                {DAYS.map((_, index) => (
                    <span
                        style={{
                            "grid-column": String(index + 2),
                            "grid-row": `1 / ${rowOf(CLOSE)}`,
                        }}
                        {...style.attrs(styles.column, index === 0 && styles.columnToday)}
                    />
                ))}
                {HOURS.map((hour) => (
                    <span style={{ "grid-row": String(rowOf(hour)) }} {...style.attrs(styles.hour)}>
                        {hour}:00
                    </span>
                ))}

                {/* place each event, the free times your booking page offers among them */}
                {week.map(([day, start, end, title, isBookable]) => (
                    <span
                        style={{
                            "grid-column": String(day + 2),
                            "grid-row": `${rowOf(start)} / ${rowOf(end)}`,
                        }}
                        {...style.attrs(
                            styles.event,
                            end - start < 1 && styles.eventShort,
                            isBookable
                                ? styles.bookable
                                : title.includes("·")
                                  ? styles.fromApps
                                  : styles.personal,
                        )}
                    >
                        {isBookable ? (
                            <span {...style.attrs(styles.eventName)}>
                                <span {...style.attrs(styles.clip)}>Open slot</span>
                            </span>
                        ) : (
                            <EventTitle title={title} />
                        )}
                        <span {...style.attrs(styles.eventTime)}>
                            {end - start < 1 ? timeOf(start) : `${timeOf(start)} – ${timeOf(end)}`}
                        </span>
                    </span>
                ))}

                {/* mark the time now on today */}
                <span
                    style={{
                        "grid-row": String(rowOf(Math.floor(NOW * 2) / 2)),
                        "margin-top": `${((NOW * 2) % 1) * 1.25}rem`,
                    }}
                    {...style.attrs(styles.now)}
                />
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
        <span {...style.attrs(styles.eventName)}>
            {link === undefined ? undefined : (
                <ServiceGlyph icon={link.startsWith("T-") ? "tasks" : "mail"} />
            )}
            <span {...style.attrs(styles.clip)}>{name}</span>
        </span>
    );
}

/** Return the grid row a time of day starts at in the week. */
function rowOf(hour: number): number {
    return 3 + (hour - OPEN) * 2;
}

/** Draw your Mail app after Apple Mail: the inbox beside Ines's open message, her card under it. */
function Mail(properties: { isRemixed: boolean }) {
    return (
        <article inert {...style.attrs(styles.mail)}>
            {/* the inbox: its title, its categories, and its messages, Ines's open */}
            <div data-xray="Database" data-source="Mail inbox" {...style.attrs(styles.mailList)}>
                <span {...style.attrs(styles.mailListHead)}>
                    <span {...style.attrs(styles.fromText)}>
                        <b {...style.attrs(appText.body)}>Inbox</b>
                        <span {...style.attrs(appText.meta, styles.quiet)}>
                            Primary · {mails.length} messages
                        </span>
                    </span>
                    <span {...style.attrs(styles.mailTools)}>
                        <Icon icon={funnelSimple} />
                        <Icon icon={dotsThree} />
                    </span>
                </span>
                {mails.slice(0, 7).map(([sender, , subject, preview, time, isUnread], index) => (
                    <span
                        {...style.attrs(
                            styles.thread,
                            index === 0 && styles.threadOn,
                            index === 1 && styles.threadAfter,
                        )}
                    >
                        <span {...style.attrs(styles.threadTop)}>
                            <b {...style.attrs(appText.body, styles.clip)}>
                                {isUnread ? (
                                    <span {...style.attrs(styles.dot, styles.unread)} />
                                ) : undefined}{" "}
                                {sender}
                            </b>
                            <span {...style.attrs(appText.meta, styles.threadTime)}>{time}</span>
                        </span>
                        <span {...style.attrs(appText.meta, styles.clip)}>{subject}</span>
                        <span {...style.attrs(appText.meta, styles.preview)}>{preview}</span>
                    </span>
                ))}
            </div>

            {/* the open message under the toolbar */}
            <div {...style.attrs(styles.reading)}>
                <span {...style.attrs(styles.mailToolbar)}>
                    <Icon icon={notePencil} />
                    <span {...style.attrs(styles.mailToolGroup)}>
                        <Icon icon={arrowBendUpLeft} />
                        <Icon icon={arrowBendDoubleUpLeft} />
                        <Icon icon={arrowBendUpRight} />
                    </span>
                    <span {...style.attrs(styles.mailToolGroup)}>
                        <Icon icon={archive} />
                        <Icon icon={trash} />
                    </span>
                    <span {...style.attrs(styles.mailToolGroup, styles.wide)}>
                        <Icon icon={folderSimple} />
                        <Icon icon={flag} />
                    </span>
                    <span {...style.attrs(appText.meta, styles.mailSearch, styles.wide)}>
                        <Icon icon={magnifyingGlass} />
                        Search
                    </span>
                </span>
                <div {...style.attrs(styles.mailMessage)}>
                    <span data-xray="Messaging" {...style.attrs(styles.from)}>
                        <Face owner="I" tint={palette.berry} size="default" />
                        <span {...style.attrs(styles.fromText)}>
                            <b {...style.attrs(appText.body)}>Ines Ferreira</b>
                            <span {...style.attrs(appText.meta)}>Re: Phase 2</span>
                            <span {...style.attrs(appText.meta, styles.quiet)}>To: Florian</span>
                        </span>
                        <span {...style.attrs(styles.fromEnd)}>
                            <span {...style.attrs(appText.meta, styles.quiet)}>10:14</span>
                            <span {...style.attrs(appText.meta, styles.details)}>Details</span>
                        </span>
                    </span>

                    {/* her card, the same one People shows, from Notes, Calendar and Tasks */}
                    <span
                        data-xray="Database"
                        data-source="Person card"
                        {...style.attrs(styles.personCard)}
                    >
                        {card.map(([label, value]) => (
                            <span {...style.attrs(styles.cardField)}>
                                <span {...style.attrs(appText.meta, styles.quiet)}>{label}</span>
                                <span {...style.attrs(appText.body)}>{value}</span>
                            </span>
                        ))}
                    </span>
                    <p {...style.attrs(appText.body)}>Hi Florian,</p>
                    <p {...style.attrs(appText.body)}>
                        Wednesday at 11 works. Can you bring the estimate? I'd like to decide on
                        card payments then.
                    </p>
                    <p {...style.attrs(appText.body)}>Thanks, Ines</p>
                    {properties.isRemixed ? (
                        <span {...style.attrs(styles.attachment, styles.changed)}>
                            <Icon icon={paperclip} />
                            <span>estimate-phase-2.pdf</span>
                            <span {...style.attrs(appText.meta, styles.quiet)}>180 KB</span>
                        </span>
                    ) : undefined}
                </div>
            </div>
        </article>
    );
}

/** Ines's card in Mail: each line's label and its live value from Notes, Calendar and Tasks. */
const card: readonly (readonly [label: string, value: string])[] = [
    ["Notes", "4 notes"],
    ["Last talked", "Mon 11:30"],
    ["Next", "Wed 11:00"],
    ["You owe her", "Estimate"],
];

/** Draw your Monday page: what you said you'd do in mail and chat, and to whom. */
function Commitments(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Monday"
                actions={
                    <>
                        <span data-xray="Analytics" {...style.attrs(appStyles.action)}>
                            3 open
                        </span>
                        <span {...style.attrs(appStyles.primary)}>Plan my week</span>
                    </>
                }
            >
                <span data-xray="Database" {...style.attrs(appStyles.property)}>
                    <Icon icon={magnifyingGlass} />
                    Search {commitments.length} commitments
                </span>
                <span {...style.attrs(appStyles.property, styles.on)}>
                    <Icon icon={funnel} />
                    Open
                </span>
                <span {...style.attrs(appStyles.property)}>From mail and chat</span>
            </Toolbar>
            <Table data-xray="Database">
                <TableHeader>
                    <TableRow>
                        <TableHead xstyle={styles.select}>
                            <Checkbox aria-label="Select all" indeterminate />
                        </TableHead>
                        <TableHead>To</TableHead>
                        <TableHead>What you said</TableHead>
                        <TableHead xstyle={properties.isRemixed && styles.changed}>
                            {properties.isRemixed ? (
                                <span {...style.attrs(styles.joined)}>
                                    <ServiceGlyph icon="mail" />
                                    Last heard
                                </span>
                            ) : (
                                "Said"
                            )}
                        </TableHead>
                        <TableHead>
                            <span {...style.attrs(styles.joined)}>
                                <ServiceGlyph icon="calendar" />
                                Next
                            </span>
                        </TableHead>
                        <TableHead>Status</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {commitments.map(([person, what, said, state]) => (
                        <TableRow>
                            <TableCell xstyle={styles.select}>
                                <Checkbox checked={state === "kept"} aria-label={what} />
                            </TableCell>
                            <TableCell xstyle={appText.body}>{person}</TableCell>
                            <TableCell xstyle={appText.body}>{what}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {properties.isRemixed ? (lastHeard[person] ?? "–") : said}
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {nextMeeting[person] ?? "–"}
                            </TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>
                                    <span
                                        {...style.attrs(
                                            styles.dot,
                                            state === "kept"
                                                ? styles.live
                                                : state === "waiting"
                                                  ? styles.idle
                                                  : styles.amber,
                                        )}
                                    />
                                    {commitmentStates[state]}
                                </span>
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <span {...style.attrs(appText.meta, styles.inline, styles.quiet)}>
                <Icon icon={dotsThree} />
                Read from your mail and chat as they come in
            </span>
        </Paned>
    );
}

/** The name each commitment state shows. */
const commitmentStates: Readonly<Record<CommitmentState, string>> = {
    open: "Open",
    waiting: "Waiting",
    kept: "Done",
};

/** Draw Ines's booking app project as an app of your space, its dates from Calendar. */
function Tracker(properties: { isRemixed: boolean }) {
    return (
        <Paned>
            <Toolbar
                title="Booking app"
                actions={
                    <>
                        <Present />
                        <span data-xray="Analytics" {...style.attrs(appStyles.action, styles.wide)}>
                            38 bookings today
                        </span>
                        <span data-xray="Messaging" {...style.attrs(appStyles.primary)}>
                            Update Ines
                        </span>
                    </>
                }
            >
                <span {...style.attrs(appStyles.property, styles.on)}>Phase 2</span>
                <span data-xray="Jobs" {...style.attrs(appStyles.property)}>
                    Status to Ines · Fri 16:00
                </span>
                <span data-xray="Observability" {...style.attrs(appStyles.property)}>
                    0 errors today
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
                    {milestones.map(([title, due, isDone, task]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>{title}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{due}</TableCell>
                            <TableCell xstyle={appText.body}>
                                {properties.isRemixed ? (
                                    <span {...style.attrs(appStyles.property)}>{task}</span>
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
            <AppHeader title="Changelog">
                <span {...style.attrs(appStyles.primary)}>New release</span>
            </AppHeader>
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
                    {releases.map(([version, date, title, notes]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(appStyles.property)}>v{version}</span>
                            </TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(styles.release)}>
                                    <b>{title}</b>
                                    <span {...style.attrs(appText.meta, styles.quiet, styles.clip)}>
                                        {notes.join(" · ")}
                                    </span>
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{date}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {properties.isRemixed ? "#booking-app" : "Ines"}
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
    more: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        paddingBlock: "0.375rem",
        paddingInline: "0.75rem",
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
    mail: {
        display: "grid",
        gridTemplateColumns: { default: "15rem minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
    },
    mailList: {
        alignContent: "start",
        backgroundColor: color.card,
        borderInlineEndColor: color.border,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: stroke.border,
        display: { default: "grid", [NARROW]: "none" },
        maskImage: "linear-gradient(to bottom, black calc(100% - 3rem), transparent)",
        minHeight: 0,
        overflow: "hidden",
        paddingInline: "0.5rem",
    },
    mailListHead: {
        alignItems: "center",
        display: "flex",
        gap: space[2],
        paddingBlock: "0.75rem",
        paddingInline: "0.5rem",
    },
    mailTools: {
        color: color.mutedForeground,
        display: "inline-flex",
        gap: space[3],
    },
    thread: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        display: "grid",
        gap: "0.0625rem",
        minWidth: 0,
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    threadOn: {
        backgroundColor: `color-mix(in srgb, ${color.primary} 14%, transparent)`,
        borderRadius: "8px",
        borderTopColor: "transparent",
    },
    threadAfter: {
        borderTopColor: "transparent",
    },
    threadTop: {
        alignItems: "baseline",
        display: "grid",
        gap: space[2],
        gridTemplateColumns: "minmax(0, 1fr) auto",
    },
    threadTime: {
        opacity: 0.7,
    },
    preview: {
        WebkitBoxOrient: "vertical",
        WebkitLineClamp: 2,
        display: "-webkit-box",
        opacity: 0.7,
        overflow: "hidden",
    },
    unread: {
        backgroundColor: palette.indigo,
        marginInlineEnd: "0.25rem",
    },
    reading: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
    },
    mailToolbar: {
        alignItems: "center",
        borderBlockEndColor: color.border,
        borderBlockEndStyle: "solid",
        borderBlockEndWidth: stroke.border,
        color: color.mutedForeground,
        display: "flex",
        gap: space[4],
        paddingBlock: "0.625rem",
        paddingInline: "1rem",
    },
    mailToolGroup: {
        display: "inline-flex",
        gap: space[3],
    },
    mailSearch: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: radius.full,
        gap: "0.25rem",
        marginInlineStart: "auto",
        paddingBlock: "0.25rem",
        paddingInline: "0.625rem",
    },
    mailMessage: {
        alignContent: "start",
        display: "grid",
        gap: space[3],
        gridAutoRows: "max-content",
        maskImage: "linear-gradient(to bottom, black calc(100% - 3rem), transparent)",
        minHeight: 0,
        overflow: "hidden",
        paddingBlock: "1rem",
        paddingInline: { default: "1.25rem", [NARROW]: "1rem" },
    },
    from: {
        alignItems: "start",
        display: "flex",
        gap: space[3],
    },
    fromText: {
        display: "grid",
        flexGrow: 1,
        minWidth: 0,
    },
    fromEnd: {
        display: "grid",
        justifyItems: "end",
    },
    details: {
        color: palette.indigo,
    },
    personCard: {
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        columnGap: space[4],
        display: "grid",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [NARROW]: "repeat(2, minmax(0, 1fr))",
        },
        paddingBlock: "0.625rem",
        paddingInline: "0.875rem",
        rowGap: space[2],
    },
    cardField: {
        display: "grid",
        gap: "0.125rem",
        minWidth: 0,
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
    release: {
        display: "grid",
        gap: "1px",
        maxWidth: "18rem",
    },
    clip: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    runs: {
        borderColor: color.border,
        borderRadius: radius[3],
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        overflow: "hidden",
    },
    runsHead: {
        alignItems: "baseline",
        backgroundColor: color.muted,
        display: "flex",
        gap: space[2],
        justifyContent: "space-between",
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    run: {
        alignItems: "center",
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        display: "grid",
        gap: space[2],
        gridTemplateColumns: "auto 7.5rem minmax(0, 1fr) auto 4.5rem",
        paddingBlock: "0.375rem",
        paddingInline: space[3],
    },
    runWhen: {
        textAlign: "end",
    },
    figures: {
        display: "grid",
        gap: space[2],
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
    },
    figure: {
        borderColor: color.border,
        borderRadius: radius[3],
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        gap: "2px",
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    figureValue: {
        fontSize: "1.125rem",
        fontVariantNumeric: "tabular-nums",
        fontWeight: weight.semibold,
        letterSpacing: "-0.01em",
    },
    week: {
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderRadius: radius[3],
        display: "grid",
        gridAutoRows: "1.25rem",
        gridTemplateColumns: "2.75rem repeat(5, minmax(0, 1fr))",
        gridTemplateRows: "auto auto",
        overflow: "hidden",
        position: "relative",
    },
    dayName: {
        alignItems: "baseline",
        color: color.mutedForeground,
        display: "flex",
        gap: space[1],
        gridRow: "1",
        paddingBlock: space[2],
        paddingInline: space[2],
        zIndex: 1,
    },
    date: {
        alignItems: "center",
        borderRadius: "999px",
        color: color.foreground,
        display: "inline-flex",
        fontSize: "0.9375rem",
        fontVariantNumeric: "tabular-nums",
        fontWeight: weight.semibold,
        height: "1.5rem",
        justifyContent: "center",
        minWidth: "1.5rem",
    },
    today: {
        backgroundColor: color.primary,
        color: color.primaryForeground,
    },
    allDayName: {
        borderBlockColor: color.border,
        borderBlockStyle: "solid",
        borderBlockWidth: stroke.border,
        color: color.mutedForeground,
        fontSize: "0.625rem",
        gridColumn: "1",
        gridRow: "2",
        paddingBlock: space[1],
        paddingInlineStart: space[2],
        zIndex: 1,
    },
    allDay: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 16%, ${color.card})`,
        borderInlineStartColor: palette.signal,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        borderRadius: radius[2],
        fontWeight: weight.semibold,
        gridColumn: "2 / -1",
        gridRow: "2",
        marginBlock: space[1],
        marginInline: space[1],
        paddingBlock: "1px",
        paddingInline: space[2],
        zIndex: 1,
    },
    grid: {
        // rule each hour, and each half hour more faintly, one tile per hour
        backgroundImage: [
            `linear-gradient(${color.border} 1px, transparent 1px)`,
            `linear-gradient(transparent 1.25rem, color-mix(in srgb, ${color.border} 45%, transparent) 1.25rem, color-mix(in srgb, ${color.border} 45%, transparent) calc(1.25rem + 1px), transparent calc(1.25rem + 1px))`,
        ].join(", "),
        backgroundSize: "100% 2.5rem",
        gridColumn: "1 / -1",
    },
    column: {
        borderInlineStartColor: color.border,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: stroke.border,
    },
    columnToday: {
        backgroundColor: `color-mix(in srgb, ${color.primary} 5%, transparent)`,
    },
    hour: {
        color: color.mutedForeground,
        fontSize: "0.625rem",
        fontVariantNumeric: "tabular-nums",
        gridColumn: "1",
        paddingInlineEnd: space[2],
        textAlign: "end",
        transform: "translateY(-0.45rem)",
    },
    event: {
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        borderRadius: radius[2],
        display: "flex",
        flexDirection: "column",
        gap: "1px",
        lineHeight: 1.2,
        marginBlock: "1px",
        marginInline: "2px",
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "2px",
        paddingInline: space[1],
        zIndex: 1,
    },
    eventShort: {
        flexDirection: "row",
        gap: space[1],
    },
    eventName: {
        alignItems: "center",
        display: "flex",
        flexShrink: 0,
        fontWeight: weight.semibold,
        gap: "3px",
        minWidth: 0,
    },
    eventTime: {
        color: color.mutedForeground,
        flexShrink: 0,
        fontWeight: weight.regular,
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
    },
    personal: {
        backgroundColor: `color-mix(in srgb, ${palette.teal} 16%, ${color.card})`,
        borderInlineStartColor: palette.teal,
        color: color.foreground,
    },
    fromApps: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 20%, ${color.card})`,
        borderInlineStartColor: palette.signal,
        color: color.foreground,
        fontWeight: weight.semibold,
    },
    bookable: {
        backgroundImage: `repeating-linear-gradient(-45deg, color-mix(in srgb, ${palette.green} 16%, transparent) 0 2px, transparent 2px 6px)`,
        borderInlineStartColor: palette.green,
        color: palette.green,
    },
    now: {
        alignSelf: "start",
        backgroundColor: color.primary,
        gridColumn: "2",
        height: "2px",
        marginInlineStart: "-4px",
        position: "relative",
        zIndex: 2,
        "::before": {
            backgroundColor: color.primary,
            borderRadius: "999px",
            content: '""',
            height: "8px",
            left: "-3px",
            position: "absolute",
            top: "-3px",
            width: "8px",
        },
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
});
