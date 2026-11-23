import type { SwatchName } from "@destack/color";
import type { IconBodies } from "@destack/icon";
import { state } from "@destack/theme/state";
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
import plus from "@destack/icon/phosphor/plus";
import smiley from "@destack/icon/phosphor/smiley";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { color, font, motion, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { Avatar, AvatarFallback, AvatarGroup } from "@destack/ui/avatar";
import { Badge } from "@destack/ui/badge";
import {
    Board,
    BoardColumn,
    BoardColumnContent,
    BoardColumnList,
    BoardColumnHeader,
    BoardItem,
} from "@destack/ui/board";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
    InputGroupTextarea,
} from "@destack/ui/input-group";
import {
    Item,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemMedia,
    ItemTitle,
} from "@destack/ui/item";
import { Toggle } from "@destack/ui/toggle";
import {
    EventCalendar,
    EventCalendarAgenda,
    EventCalendarBody,
    EventCalendarHeader,
    EventCalendarMonth,
    EventCalendarNext,
    EventCalendarPrevious,
    EventCalendarTime,
    EventCalendarToday,
    EventCalendarViewSwitcher,
} from "@destack/ui/event-calendar";
import { Button } from "@destack/ui/button";
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from "@destack/ui/card";
import { ObjectChip } from "@destack/ui/object";
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";
import { Checkbox } from "@destack/ui/checkbox";
import { Progress } from "@destack/ui/progress";
import { Separator } from "@destack/ui/separator";
import { Spinner } from "@destack/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@destack/ui/table";
import { type JSX, Show } from "@destack/view";

import { palette } from "../../palette.stylex";
import { AppHeader, appStyles, appText } from "../figure/app";
import { AgentAvatar } from "../figure/agent";
import {
    board,
    type CommitmentState,
    HANDOVER_DAY,
    messages,
    type ReplyState,
    type Story,
    type TaskState,
    useStory,
} from "./story";
import type { Job } from "./space";
import { milestones } from "../project/tracker";
import { releases } from "../project/changelog";
import { ServiceGlyph } from "../figure/tile";
import type { Source } from "./source";
import { Favicon } from "../figure/glyph";

/** An app narrower than a phone screen, which then drops what only fits wide. */
const NARROW = "@container (max-width: 479px)";

/** The time your week opens at. */
const OPEN = "09:00";

/** The task states in the order the list groups them, with their names and marks. */
const STATES: readonly (readonly [state: TaskState, name: string])[] = [
    ["started", "In progress"],
    ["review", "In review"],
    ["todo", "Todo"],
    ["done", "Done"],
];

/** The tasks you do for Ines, which her page lists. */
const INES_TASKS: ReadonlySet<string> = new Set(["T-12", "T-14", "T-13"]);

/** The icon of each app a task's link points into. */
const LINK_ICONS = {
    source: "git-pull-request",
    calendar: "calendar-blank",
    mail: "envelope-simple",
    table: "table",
} as const;

/** Read the object in another app each task points at, by the app's icon and its live value. */
function taskLinksOf(
    story: Story,
): Readonly<Record<string, readonly [icon: keyof typeof LINK_ICONS, text: string]>> {
    return {
        "T-12": ["source", "PR #48 · merged"],
        "T-14": ["calendar", "Review · Wed 11:00"],
        "T-16": ["mail", "29 of 31 matched"],
        "T-13": ["table", "3 clients"],
        "T-15": ["mail", "Saturday?"],
        "T-17": ["calendar", "Thu 08:30"],
        "T-18": ["table", `Payments · ${story.label(-8)}`],
        "T-19": ["table", "4 unused"],
    };
}

/** Read the last you heard from the person behind each commitment, by person. */
function lastHeardOf(story: Story): Readonly<Record<string, string>> {
    return {
        "Ines Ferreira": "Mail · 10:14",
        "Jonas Weber": "Mail · 09:31",
        "Mia Sørensen": "Mail · 08:52",
        "Kai Brandt": `Mail · ${story.label(-1)}`,
    };
}

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
        return <Canvas isRemixed={isRemixed} />;
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
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title={properties.source.table}
                actions={
                    <>
                        <span {...style.attrs(appStyles.note)}>Synced 4 min ago</span>
                        <Button>Chart it</Button>
                    </>
                }
            >
                <Toggle variant="outline" defaultPressed>
                    <Favicon icon={properties.source.icon} tint={properties.source.tint} />
                    From {properties.source.name}
                </Toggle>
                <Badge variant="secondary">
                    <ServiceGlyph icon="sync" />
                    Every 15 minutes
                </Badge>
                <Badge variant="secondary">Connected by your agent</Badge>
            </Toolbar>
            {/* show the table's headline figures */}
            <div {...style.attrs(styles.figures)}>
                {properties.source.figures.map(([label, value]) => (
                    <Card size="sm">
                        <CardHeader>
                            <CardDescription>{label}</CardDescription>
                            <CardTitle xstyle={styles.figureValue}>{value}</CardTitle>
                        </CardHeader>
                    </Card>
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
                    {properties.source.rows.map(([day, what, amount]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>{story().label(day)}</TableCell>
                            <TableCell xstyle={appText.body}>{what}</TableCell>
                            <TableCell xstyle={appText.body}>{amount}</TableCell>
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

/** Draw a state as a badge in its color, or as a plain one for a state without a color. */
function StatusBadge(properties: { color: SwatchName | undefined; children: JSX.Element }) {
    return properties.color === undefined ? (
        <Badge variant="secondary">{properties.children}</Badge>
    ) : (
        <Badge color={properties.color}>{properties.children}</Badge>
    );
}

/** Draw a tool of the mail toolbar: an icon button with its name for assistive technology. */
function MailTool(properties: { icon: IconBodies; label: string }) {
    return (
        <Button variant="ghost" size="icon-sm" aria-label={properties.label}>
            <Icon icon={properties.icon} />
        </Button>
    );
}

/** Draw a search field of an app, its placeholder saying what it searches. */
function Search(properties: { label: string; "data-xray"?: string }) {
    return (
        <InputGroup data-xray={properties["data-xray"]} xstyle={styles.search}>
            <InputGroupAddon>
                <Icon icon={magnifyingGlass} />
            </InputGroupAddon>
            <InputGroupInput
                type="search"
                placeholder={properties.label}
                aria-label={properties.label}
            />
        </InputGroup>
    );
}

/** Draw a person's face: you in teal, anyone else in their color, and your agent as its avatar. */
function Face(properties: { owner: string; tint?: SwatchName; size?: "sm" | "default" }) {
    return isAgentOwner(properties.owner) ? (
        <Avatar size={properties.size ?? "sm"}>
            <AgentAvatar size="fill" />
        </Avatar>
    ) : (
        <Avatar size={properties.size ?? "sm"} color={properties.tint ?? "teal"}>
            <AvatarFallback>{properties.owner}</AvatarFallback>
        </Avatar>
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
    // read the story the page tells
    const story = useStory();

    // count her tasks done, and read this week's events from her apps
    const herTasks = story().tasks.filter(([id]) => INES_TASKS.has(id));
    const done = herTasks.filter(([, , taskState]) => taskState === "done").length;
    const linked = () => story().events.filter((event) => event.title.includes(" · "));

    return (
        <Paned>
            <AppHeader title="Ines Ferreira">
                <Present />
                <span {...style.attrs(appStyles.note, styles.wide)}>Agent edited just now</span>
                <Button>Share</Button>
            </AppHeader>
            <article {...style.attrs(styles.document)}>
                <div {...style.attrs(appStyles.properties)}>
                    <Badge variant="secondary">
                        <StateMark state="started" />
                        Client
                    </Badge>
                    <Badge variant="secondary">Booking app</Badge>
                    <Badge variant="secondary" data-xray="Analytics" xstyle={styles.wide}>
                        14 mails this month
                    </Badge>
                    <Badge variant="secondary" data-xray="Jobs" data-source="Calendar event">
                        Review · Wed 11:00
                    </Badge>
                    {properties.isRemixed ? (
                        <Toggle variant="outline" defaultPressed xstyle={styles.changed}>
                            Last talked Mon 11:30
                        </Toggle>
                    ) : undefined}
                </div>
                <p {...style.attrs(appText.body)}>
                    Runs a physio practice with three rooms. Phase 2 adds card payments and moves
                    the app to her own domain before {story().monthName(HANDOVER_DAY, 1)}.
                </p>
                <div {...style.attrs(styles.annotated)}>
                    {/* her tasks, a live view of the Tasks app */}
                    <Card size="sm" data-xray="Database" data-source="Tasks list">
                        <CardHeader>
                            <CardTitle>Tasks for Ines</CardTitle>
                            <CardDescription>
                                {done} of {herTasks.length} done
                            </CardDescription>
                            <CardAction>
                                <Progress
                                    value={(done * 100) / herTasks.length}
                                    aria-label="Tasks done"
                                    xstyle={styles.progress}
                                />
                            </CardAction>
                        </CardHeader>
                        <Table aria-label="Tasks for Ines">
                            <TableBody>
                                {herTasks.map(([id, title, taskState, owner, tint, due]) => (
                                    <TableRow>
                                        <TableCell xstyle={styles.titleCell}>
                                            <span {...style.attrs(styles.inline)}>
                                                <StateMark state={taskState} />
                                                <span
                                                    {...style.attrs(
                                                        styles.clip,
                                                        taskState === "done" && styles.struck,
                                                        isCommented(title) && styles.highlight,
                                                    )}
                                                >
                                                    {title}
                                                </span>
                                            </span>
                                        </TableCell>
                                        <TableCell
                                            xstyle={[appText.meta, styles.quiet, styles.wide]}
                                        >
                                            {id}
                                        </TableCell>
                                        <TableCell>
                                            <Face owner={owner} tint={tint} size="sm" />
                                        </TableCell>
                                        <TableCell xstyle={[appText.meta, styles.quiet]}>
                                            {story().label(due)}
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    </Card>

                    {/* the agent's comment in the margin, a thread from Chat */}
                    <Card
                        size="sm"
                        data-xray="Messaging"
                        data-source="Chat thread"
                        xstyle={styles.comment}
                    >
                        <CardHeader>
                            <span {...style.attrs(styles.inline)}>
                                <Face owner="A" />
                                <CardTitle>Agent</CardTitle>
                                <CardDescription>09:13</CardDescription>
                            </span>
                        </CardHeader>
                        <CardContent xstyle={appText.meta}>
                            She asked about card payments, so I linked it here and in T-14.
                        </CardContent>
                        <CardFooter>
                            <Button variant="link" size="inline">
                                Reply
                            </Button>
                        </CardFooter>
                    </Card>
                </div>

                {/* this week, a live view of the Calendar app */}
                <section
                    data-xray="Database"
                    data-source="Calendar week"
                    {...style.attrs(styles.section)}
                >
                    <b {...style.attrs(appText.body)}>This week</b>
                    <Show when={story()} keyed>
                        {(current) => (
                            <EventCalendar
                                events={linked()}
                                defaultView="agenda"
                                defaultDate={current.monday}
                                weekends={false}
                                now={current.now}
                                timeZone={current.zone}
                            >
                                <EventCalendarBody>
                                    <EventCalendarAgenda />
                                </EventCalendarBody>
                            </EventCalendar>
                        )}
                    </Show>
                </section>

                {/* the estimate draft, attached */}
                <Item variant="outline" size="sm" xstyle={styles.attachment}>
                    <ItemMedia>
                        <Icon icon={fileText} />
                    </ItemMedia>
                    <ItemContent>
                        <ItemTitle>estimate-phase-2.pdf</ItemTitle>
                    </ItemContent>
                    <ItemDescription>180 KB</ItemDescription>
                </Item>
            </article>
        </Paned>
    );
}

/** Report whether a task carries the agent's comment. */
function isCommented(item: string): boolean {
    return item.startsWith("Send Ines");
}

/** Draw your tasks: your list grouped by state, your agent working through its share. */
function Tasks(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title="Tasks"
                actions={
                    <Button>
                        <Icon icon={plus} />
                        New task
                    </Button>
                }
            >
                <ToggleGroup defaultValue="list" variant="outline" aria-label="View">
                    <ToggleGroupItem value="list">
                        <Icon icon={listBullets} />
                        List
                    </ToggleGroupItem>
                    <ToggleGroupItem value="board">
                        <Icon icon={kanban} />
                        Board
                    </ToggleGroupItem>
                </ToggleGroup>
                <Badge variant="secondary" data-xray="Database">
                    <Icon icon={funnel} />
                    Due this week
                </Badge>
                <Badge variant="secondary" data-xray="Jobs" xstyle={styles.wide}>
                    <ServiceGlyph icon="sync" />
                    When done, tell whoever asked
                </Badge>
                {properties.isRemixed ? (
                    <Badge xstyle={styles.changed}>By priority</Badge>
                ) : undefined}
            </Toolbar>
            <Table data-xray="Database" aria-label="Tasks">
                <TableBody>
                    {STATES.map(([taskState, name]) => (
                        <>
                            <TableRow xstyle={styles.groupRow}>
                                <TableCell colspan={5}>
                                    <span {...style.attrs(styles.group)}>
                                        <StateMark state={taskState} />
                                        <b>{name}</b>
                                        <span {...style.attrs(styles.quiet)}>
                                            {
                                                story().tasks.filter(
                                                    (task) => task[2] === taskState,
                                                ).length
                                            }
                                        </span>
                                    </span>
                                </TableCell>
                            </TableRow>
                            {story()
                                .tasks.filter((task) => task[2] === taskState)
                                .map(([id, title, , owner, tint, due]) => (
                                    <TableRow>
                                        <TableCell xstyle={[styles.quiet, styles.taskId]}>
                                            {id}
                                        </TableCell>
                                        <TableCell xstyle={styles.titleCell}>
                                            <span {...style.attrs(styles.group)}>
                                                <StateMark state={taskState} />
                                                <span
                                                    {...style.attrs(
                                                        styles.clip,
                                                        taskState === "done" && styles.struck,
                                                    )}
                                                >
                                                    {properties.isRemixed ? (
                                                        <Badge
                                                            xstyle={[
                                                                styles.priority,
                                                                styles.changed,
                                                            ]}
                                                        >
                                                            {priorities[id] ?? "P3"}
                                                        </Badge>
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
                                            </span>
                                        </TableCell>
                                        <TableCell>
                                            <LinkChip id={id} />
                                        </TableCell>
                                        <TableCell xstyle={styles.quiet}>
                                            {story().label(due)}
                                        </TableCell>
                                        <TableCell>
                                            <Face owner={owner} tint={tint} />
                                        </TableCell>
                                    </TableRow>
                                ))}
                        </>
                    ))}
                </TableBody>
            </Table>
        </Paned>
    );
}

/** Draw the channel you share with Ines: its members, a day of messages, and the composer. */
function Chat(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <div {...style.attrs(styles.channelHead)}>
                <b {...style.attrs(appText.heading)}># booking-app</b>
                <span {...style.attrs(appText.meta, styles.quiet, styles.wide)}>
                    Phase 2 before {story().monthName(HANDOVER_DAY, 1)}
                </span>
                <span data-xray="Identity" {...style.attrs(styles.members)}>
                    <AvatarGroup aria-label="Members">
                        <Face owner="F" />
                        <Face owner="A" />
                        <Face owner="I" tint="crimson" />
                    </AvatarGroup>
                    <span {...style.attrs(appText.meta, styles.quiet)}>3</span>
                </span>
            </div>
            {/* the checklist your agent pins when asked */}
            {properties.isRemixed ? (
                <div {...style.attrs(appText.meta, styles.pinned, styles.changed)}>
                    <b>Pinned</b> Agreed · cards at booking, her domain by{" "}
                    {story().monthName(HANDOVER_DAY, 1)}
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
                            tint={author === "Ines" ? "crimson" : "teal"}
                            size="default"
                        />
                        <div {...style.attrs(styles.messageBody)}>
                            <span {...style.attrs(appText.body)}>
                                <b>{author}</b> <span {...style.attrs(styles.quiet)}>{time}</span>
                            </span>
                            <span {...style.attrs(appText.body)}>{line}</span>
                            {index === 1 ? (
                                <Item variant="outline" size="sm" xstyle={styles.linked}>
                                    <ItemMedia>
                                        <StateMark state="started" />
                                    </ItemMedia>
                                    <ItemContent>
                                        <ItemTitle>T-14 Send Ines the estimate</ItemTitle>
                                        <ItemDescription>Tasks · due Wed 11:00</ItemDescription>
                                    </ItemContent>
                                </Item>
                            ) : undefined}
                            {index === 1 ? (
                                <span {...style.attrs(styles.inline)}>
                                    <Toggle
                                        variant="outline"
                                        size="sm"
                                        defaultPressed
                                        aria-label="Thumbs up, 1"
                                    >
                                        👍 1
                                    </Toggle>
                                    <Toggle
                                        variant="outline"
                                        size="sm"
                                        aria-label="Folded hands, 1"
                                    >
                                        🙏 1
                                    </Toggle>
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
            <InputGroup>
                <InputGroupTextarea
                    rows={1}
                    placeholder="Message #booking-app"
                    aria-label="Message"
                />
                <InputGroupAddon align="block-end">
                    <InputGroupButton size="icon-sm" aria-label="Attach a file">
                        <Icon icon={paperclip} />
                    </InputGroupButton>
                    <InputGroupButton size="icon-sm" aria-label="Mention someone">
                        <Icon icon={at} />
                    </InputGroupButton>
                    <InputGroupButton size="icon-sm" aria-label="Add an emoji">
                        <Icon icon={smiley} />
                    </InputGroupButton>
                    <InputGroupButton
                        variant="default"
                        size="icon-sm"
                        aria-label="Send"
                        xstyle={styles.send}
                    >
                        <Icon icon={paperPlaneRight} />
                    </InputGroupButton>
                </InputGroupAddon>
            </InputGroup>
            <span {...style.attrs(appText.meta, styles.typing)}>
                <Spinner aria-label="Agent is typing" />
                Agent is typing…
            </span>
        </Paned>
    );
}

/** Draw the people you deal with, their mail and meetings read live from Mail and Calendar. */
function People(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title="People"
                actions={
                    <>
                        <Button variant="ghost">Grid</Button>
                        <Button>New row</Button>
                    </>
                }
            >
                <Search data-xray="Database" label={`Search ${story().people.length} people`} />
                {properties.isRemixed ? (
                    <Toggle variant="outline" defaultPressed xstyle={styles.changed}>
                        Owe a reply after 3 days
                    </Toggle>
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
                    {story().people.map(([name, relation, replyState, last, tint]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(styles.inline)}>
                                    <Face owner={name.slice(0, 1)} tint={tint} />
                                    {name}
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>{relation}</TableCell>
                            <TableCell xstyle={appText.body}>
                                <StatusBadge color={REPLY_COLORS[replyState]}>
                                    {replyStates[replyState]}
                                </StatusBadge>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {story().label(last)}
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {nextOf(name, replyState, properties.isRemixed)}
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Paned>
    );
}

/** Return a person's last column: a reply you owe once remixed, else your next meeting. */
function nextOf(name: string, replyState: ReplyState, isRemixed: boolean): string {
    // flag the replies you owe once remixed
    if (isRemixed) {
        return replyState === "owe" ? "Today" : "–";
    }

    // read the next meeting from Calendar
    return nextMeeting[name] ?? "–";
}

/** The color each reply state's badge takes, the plain badge for one waiting on them. */
const REPLY_COLORS: Readonly<Partial<Record<ReplyState, SwatchName>>> = {
    owe: "amber",
    clear: "green",
};

/** The name each reply state shows. */
const replyStates: Readonly<Record<ReplyState, string>> = {
    owe: "Reply owed",
    waiting: "Waiting on them",
    clear: "Up to date",
};

/** Draw your week as a board of notes, each task's note live from Tasks. */
function Canvas(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title="This week"
                actions={
                    <>
                        <Present />
                        <Button>Share</Button>
                    </>
                }
            >
                {properties.isRemixed ? (
                    <Toggle variant="outline" defaultPressed xstyle={styles.changed}>
                        What waits on what, drawn as arrows
                    </Toggle>
                ) : undefined}
            </Toolbar>
            <Board data-xray="Database" label="This week" xstyle={styles.board}>
                <BoardColumnList>
                    {board.map(([column, notes]) => (
                        <BoardColumn id={column} label={column} xstyle={styles.boardColumn}>
                            <BoardColumnHeader xstyle={appText.meta}>{column}</BoardColumnHeader>
                            <BoardColumnContent>
                                {notes.map((note) => {
                                    const task = story().tasks.find(([id]) => id === note);

                                    return task === undefined ? (
                                        <BoardItem id={note} xstyle={[appText.body, styles.sticky]}>
                                            {note}
                                        </BoardItem>
                                    ) : (
                                        <BoardItem
                                            id={note}
                                            label={task[1]}
                                            xstyle={[appText.body, styles.card]}
                                        >
                                            <span {...style.attrs(styles.inline)}>
                                                <StateMark state={task[2]} />
                                                <b {...style.attrs(appText.meta)}>{task[0]}</b>
                                            </span>
                                            {task[1]}
                                            <span {...style.attrs(appText.meta, styles.quiet)}>
                                                Tasks · due {story().label(task[5])}
                                            </span>
                                        </BoardItem>
                                    );
                                })}
                            </BoardColumnContent>
                        </BoardColumn>
                    ))}
                </BoardColumnList>
            </Board>
        </Paned>
    );
}

/** Draw the workflows of your space and their runs. */
function Flows(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title="Workflows"
                actions={
                    <>
                        <Button variant="ghost">Runs</Button>
                        <Button>New workflow</Button>
                    </>
                }
            >
                <Badge variant="secondary" data-xray="Jobs">
                    {story().workflows.length} workflows
                </Badge>
                {properties.isRemixed ? (
                    <Toggle variant="outline" defaultPressed xstyle={styles.changed}>
                        Alert me in Chat when a run fails
                    </Toggle>
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
                    {story().workflows.map(([name, trigger, total, last]) => (
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
            <Card size="sm">
                <CardHeader>
                    <CardTitle>Recent runs</CardTitle>
                    <CardDescription>Each step recorded once, retried on failure</CardDescription>
                </CardHeader>
                <Table aria-label="Recent runs">
                    <TableBody>
                        {story().runs.map(([workflow, subject, steps, duration, when]) => (
                            <TableRow>
                                <TableCell xstyle={appText.meta}>
                                    <span {...style.attrs(styles.inline)}>
                                        <span {...style.attrs(styles.dot, styles.live)} />
                                        <b {...style.attrs(styles.code)}>{workflow}</b>
                                    </span>
                                </TableCell>
                                <TableCell xstyle={[appText.meta, styles.titleCell, styles.clip]}>
                                    {subject}
                                </TableCell>
                                <TableCell xstyle={[appText.meta, styles.quiet]}>
                                    {steps} {steps === 1 ? "step" : "steps"} · {duration}
                                </TableCell>
                                <TableCell xstyle={[appText.meta, styles.quiet, styles.runWhen]}>
                                    {when}
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </Card>
        </Paned>
    );
}

/** Draw your week: the events your apps put there and the free times your booking page offers. */
function Week(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title={story().labelRange(0, 4, { month: "long", day: "numeric" })}
                actions={<Button data-xray="Messaging">Share booking link</Button>}
            >
                {properties.isRemixed ? (
                    <Toggle variant="outline" defaultPressed xstyle={styles.changed}>
                        Focus 9–11 every weekday
                    </Toggle>
                ) : undefined}
            </Toolbar>
            <Show when={story()} keyed>
                {(current) => (
                    <EventCalendar
                        data-xray="Database"
                        events={current.events}
                        defaultView="week"
                        defaultDate={current.monday}
                        weekends={false}
                        now={current.now}
                        timeZone={current.zone}
                        scrollTime={OPEN}
                        hourHeight="2.5rem"
                        xstyle={styles.week}
                    >
                        <EventCalendarHeader>
                            <EventCalendarPrevious />
                            <EventCalendarToday />
                            <EventCalendarNext />
                            <EventCalendarViewSwitcher />
                        </EventCalendarHeader>
                        <EventCalendarBody>
                            <EventCalendarMonth />
                            <EventCalendarTime />
                            <EventCalendarAgenda />
                        </EventCalendarBody>
                    </EventCalendar>
                )}
            </Show>
        </Paned>
    );
}

/** Draw a task's link to an object in another app, with that object's live value. */
function LinkChip(properties: { id: string }) {
    const story = useStory();
    const link = () => present(taskLinksOf(story())[properties.id], "task link");

    return <ObjectChip title={link()[1]} glyph={{ icon: LINK_ICONS[link()[0]] }} />;
}

/** Draw your Mail app after Apple Mail: the inbox beside Ines's open message, her card under it. */
function Mail(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <article inert {...style.attrs(styles.mail)}>
            {/* the inbox: its title, its categories, and its messages, Ines's open */}
            <div data-xray="Database" data-source="Mail inbox" {...style.attrs(styles.mailList)}>
                <span {...style.attrs(styles.mailListHead)}>
                    <span {...style.attrs(styles.fromText)}>
                        <b {...style.attrs(appText.body)}>Inbox</b>
                        <span {...style.attrs(appText.meta, styles.quiet)}>
                            Primary · {story().mails.length} messages
                        </span>
                    </span>
                    <span {...style.attrs(styles.mailTools)}>
                        <Button variant="ghost" size="icon-sm" aria-label="Filter">
                            <Icon icon={funnelSimple} />
                        </Button>
                        <Button variant="ghost" size="icon-sm" aria-label="More">
                            <Icon icon={dotsThree} />
                        </Button>
                    </span>
                </span>
                <ItemGroup aria-label="Inbox">
                    {story()
                        .mails.slice(0, 7)
                        .map(([sender, , subject, preview, time, isUnread], index) => (
                            <Item size="sm" xstyle={[styles.thread, index === 0 && state.selected]}>
                                <ItemContent>
                                    <ItemTitle xstyle={styles.threadTop}>
                                        <span {...style.attrs(styles.clip)}>
                                            {isUnread ? (
                                                <span {...style.attrs(styles.dot, styles.unread)} />
                                            ) : undefined}
                                            {sender}
                                        </span>
                                        <span {...style.attrs(appText.meta, styles.threadTime)}>
                                            {time}
                                        </span>
                                    </ItemTitle>
                                    <ItemDescription xstyle={styles.clip}>
                                        {subject}
                                    </ItemDescription>
                                    <ItemDescription xstyle={styles.preview}>
                                        {preview}
                                    </ItemDescription>
                                </ItemContent>
                            </Item>
                        ))}
                </ItemGroup>
            </div>

            {/* the open message under the toolbar */}
            <div {...style.attrs(styles.reading)}>
                <span {...style.attrs(styles.mailToolbar)}>
                    <MailTool icon={notePencil} label="New message" />
                    <Separator orientation="vertical" />
                    <MailTool icon={arrowBendUpLeft} label="Reply" />
                    <MailTool icon={arrowBendDoubleUpLeft} label="Reply all" />
                    <MailTool icon={arrowBendUpRight} label="Forward" />
                    <Separator orientation="vertical" />
                    <MailTool icon={archive} label="Archive" />
                    <MailTool icon={trash} label="Delete" />
                    <span {...style.attrs(styles.mailToolGroup, styles.wide)}>
                        <Separator orientation="vertical" />
                        <MailTool icon={folderSimple} label="Move" />
                        <MailTool icon={flag} label="Flag" />
                    </span>
                    <span {...style.attrs(styles.mailSearch, styles.wide)}>
                        <Search label="Search" />
                    </span>
                </span>
                <div {...style.attrs(styles.mailMessage)}>
                    <span data-xray="Messaging" {...style.attrs(styles.from)}>
                        <Face owner="I" tint="crimson" size="default" />
                        <span {...style.attrs(styles.fromText)}>
                            <b {...style.attrs(appText.body)}>Ines Ferreira</b>
                            <span {...style.attrs(appText.meta)}>Re: Phase 2</span>
                            <span {...style.attrs(appText.meta, styles.quiet)}>To: Florian</span>
                        </span>
                        <span {...style.attrs(styles.fromEnd)}>
                            <span {...style.attrs(appText.meta, styles.quiet)}>10:14</span>
                            <Button variant="link" size="inline">
                                Details
                            </Button>
                        </span>
                    </span>

                    {/* her card, the same one People shows, from Notes, Calendar and Tasks */}
                    <Card size="sm" data-xray="Database" data-source="Person card">
                        <CardContent xstyle={styles.personCard}>
                            {card.map(([label, value]) => (
                                <span {...style.attrs(styles.cardField)}>
                                    <span {...style.attrs(appText.meta, styles.quiet)}>
                                        {label}
                                    </span>
                                    <span {...style.attrs(appText.body)}>{value}</span>
                                </span>
                            ))}
                        </CardContent>
                    </Card>
                    <p {...style.attrs(appText.body)}>Hi Florian,</p>
                    <p {...style.attrs(appText.body)}>
                        Wednesday at 11 works. Can you bring the estimate? I'd like to decide on
                        card payments then.
                    </p>
                    <p {...style.attrs(appText.body)}>Thanks, Ines</p>
                    {properties.isRemixed ? (
                        <Item
                            variant="outline"
                            size="sm"
                            xstyle={[styles.attachment, styles.changed]}
                        >
                            <ItemMedia>
                                <Icon icon={paperclip} />
                            </ItemMedia>
                            <ItemContent>
                                <ItemTitle>estimate-phase-2.pdf</ItemTitle>
                            </ItemContent>
                            <ItemDescription>180 KB</ItemDescription>
                        </Item>
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
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title="Monday"
                actions={
                    <>
                        <span data-xray="Analytics" {...style.attrs(appStyles.note)}>
                            3 open
                        </span>
                        <Button>Plan my week</Button>
                    </>
                }
            >
                <Search
                    data-xray="Database"
                    label={`Search ${story().commitments.length} commitments`}
                />
                <Toggle variant="outline" defaultPressed>
                    <Icon icon={funnel} />
                    Open
                </Toggle>
                <Badge variant="secondary">From mail and chat</Badge>
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
                    {story().commitments.map(([person, what, channel, said, commitmentState]) => (
                        <TableRow>
                            <TableCell xstyle={styles.select}>
                                <Checkbox checked={commitmentState === "kept"} aria-label={what} />
                            </TableCell>
                            <TableCell xstyle={appText.body}>{person}</TableCell>
                            <TableCell xstyle={appText.body}>{what}</TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {properties.isRemixed
                                    ? (lastHeardOf(story())[person] ?? "–")
                                    : `${channel} · ${story().label(said)}`}
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {nextMeeting[person] ?? "–"}
                            </TableCell>
                            <TableCell xstyle={appText.body}>
                                <StatusBadge color={COMMITMENT_COLORS[commitmentState]}>
                                    {commitmentStates[commitmentState]}
                                </StatusBadge>
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

/** The color each commitment state's badge takes, the plain badge for one waiting on them. */
const COMMITMENT_COLORS: Readonly<Partial<Record<CommitmentState, SwatchName>>> = {
    open: "amber",
    kept: "green",
};

/** The name each commitment state shows. */
const commitmentStates: Readonly<Record<CommitmentState, string>> = {
    open: "Open",
    waiting: "Waiting",
    kept: "Done",
};

/** Draw Ines's booking app project as an app of your space, its dates from Calendar. */
function Tracker(properties: { isRemixed: boolean }) {
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <Toolbar
                title="Booking app"
                actions={
                    <>
                        <Present />
                        <span data-xray="Analytics" {...style.attrs(appStyles.note, styles.wide)}>
                            38 bookings today
                        </span>
                        <Button data-xray="Messaging">Update Ines</Button>
                    </>
                }
            >
                <Toggle variant="outline" defaultPressed>
                    Phase 2
                </Toggle>
                <Badge variant="secondary" data-xray="Jobs">
                    Status to Ines · Fri 16:00
                </Badge>
                <Badge variant="secondary" data-xray="Observability">
                    0 errors today
                </Badge>
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
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {story().labelPlan(due)}
                            </TableCell>
                            <TableCell xstyle={appText.body}>
                                {properties.isRemixed ? (
                                    <Badge variant="secondary">{task}</Badge>
                                ) : (
                                    <Badge color={isDone ? "green" : "amber"}>
                                        {isDone ? "Done" : "Open"}
                                    </Badge>
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
    // read the story the page tells
    const story = useStory();

    return (
        <Paned>
            <AppHeader title="Changelog">
                <Button>New release</Button>
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
                    {releases.map(([version, day, title, notes]) => (
                        <TableRow>
                            <TableCell xstyle={appText.body}>
                                <Badge variant="outline">v{version}</Badge>
                            </TableCell>
                            <TableCell xstyle={appText.body}>
                                <span {...style.attrs(styles.release)}>
                                    <b>{title}</b>
                                    <span {...style.attrs(appText.meta, styles.quiet, styles.clip)}>
                                        {notes.join(" · ")}
                                    </span>
                                </span>
                            </TableCell>
                            <TableCell xstyle={[appText.body, styles.quiet]}>
                                {story().label(day)}
                            </TableCell>
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
    live: {
        backgroundColor: palette.green,
    },
    attachment: {
        justifySelf: "start",
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
    section: {
        display: "grid",
        gap: space[2],
    },
    comment: {
        display: { default: null, [NARROW]: "none" },
        marginBlockStart: space[6],
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
        minWidth: 0,
    },
    threadTop: {
        alignItems: "baseline",
        display: "grid",
        gap: space[2],
        gridTemplateColumns: "minmax(0, 1fr) auto",
        width: "100%",
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
        display: "flex",
        gap: space[1],
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    mailToolGroup: {
        alignItems: "center",
        display: "inline-flex",
        gap: space[1],
    },
    mailSearch: {
        display: "flex",
        flex: "0 1 10rem",
        minWidth: 0,
        marginInlineStart: "auto",
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
    personCard: {
        columnGap: space[4],
        display: "grid",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [NARROW]: "repeat(2, minmax(0, 1fr))",
        },
        rowGap: space[2],
    },
    cardField: {
        display: "grid",
        gap: "0.125rem",
        minWidth: 0,
    },
    mark: {
        display: "inline-flex",
    },
    groupRow: {
        backgroundColor: color.muted,
    },
    group: {
        alignItems: "center",
        display: "flex",
        gap: space[2],
        minWidth: 0,
    },
    titleCell: {
        width: "100%",
        maxWidth: 0,
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
        marginInlineEnd: space[1],
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
    board: {
        overflow: "hidden",
    },
    boardColumn: {
        flexBasis: 0,
        flexGrow: 1,
        flexShrink: 1,
        minWidth: 0,
        width: 0,
    },
    card: {
        display: "grid",
        gap: space[1],
    },
    sticky: {
        backgroundColor: swatch.amberTint,
        borderColor: "transparent",
        color: swatch.amberTintText,
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
        width: "fit-content",
        marginBlockStart: space[1],
    },
    send: {
        marginInlineStart: "auto",
    },
    search: {
        flex: "0 1 14rem",
        minWidth: 0,
    },
    typing: {
        alignItems: "center",
        color: palette.violet,
        display: "flex",
        gap: space[2],
        marginTop: `calc(${space[2]} * -1)`,
        paddingInline: space[3],
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
        color: swatch.accentSolid,
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
    runWhen: {
        textAlign: "end",
    },
    figures: {
        display: "grid",
        gap: space[2],
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
    },
    figureValue: {
        fontVariantNumeric: "tabular-nums",
    },
    week: {
        minHeight: 0,
    },
    select: {
        width: "2.5rem",
    },
    joined: {
        alignItems: "center",
        display: "inline-flex",
        gap: space[1],
    },
});
