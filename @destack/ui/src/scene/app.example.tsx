import type { Hue } from "@destack/color";
import { defineExample } from "@destack/package/declare";
import { PlainDate, present, TimeZone } from "@destack/schema";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import {
    color,
    font,
    radius,
    size,
    space,
    stroke,
    weight,
    width,
    swatch,
} from "@destack/theme/tokens.stylex";
import { DataList, DataListItem, DataListLabel, DataListValue } from "../data-list/index.ts";
import { Icon } from "@destack/ui/icon";
import { createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import { Alert, AlertDescription, AlertTitle } from "../alert/index.ts";
import { Avatar, AvatarFallback, AvatarGroup, AvatarImage } from "../avatar/index.ts";
import { Badge } from "../badge/index.ts";
import {
    Board,
    BoardColumn,
    BoardColumnAdd,
    BoardColumnMenu,
    BoardComposer,
    BoardHiddenList,
    BoardColumnContent,
    BoardColumnCount,
    BoardColumnList,
    BoardMove,
    BoardColumnHandle,
    BoardColumnHeader,
    BoardItem,
} from "../board/index.ts";
import { Bubble, BubbleContent } from "../bubble/index.ts";
import {
    Composer,
    ComposerAddon,
    ComposerFooter,
    ComposerFrame,
    ComposerInput,
    ComposerSubmit,
} from "../composer/index.ts";
import { Marker, MarkerContent, MarkerIcon } from "../marker/index.ts";
import {
    Message,
    MessageToolbar,
    MessageAuthor,
    MessageAvatar,
    MessageContent,
    MessageFooter,
    MessageHeader,
} from "../message/index.ts";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerItem,
    MessageScrollerViewport,
} from "../message-scroller/index.ts";
import { ToolbarButton } from "../toolbar/index.ts";
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "../breadcrumb/index.ts";
import { Button } from "../button/index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../collapsible/index.ts";
import {
    type CalendarEvent,
    EventCalendar,
    EventCalendarBody,
    EventCalendarHeader,
    EventCalendarMonth,
    EventCalendarTime,
} from "../event-calendar/index.ts";
import { PICTURES } from "../example/picture.ts";
import { KbdShortcut } from "../kbd/index.ts";
import { Label } from "../label/index.ts";
import { Prose } from "../prose/index.ts";
import "../prose/prose.css";
import {
    Sidebar,
    SidebarContent,
    SidebarFooter,
    SidebarGroup,
    SidebarGroupLabel,
    SidebarHeader,
    SidebarInset,
    SidebarMenu,
    SidebarMenuBadge,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarProvider,
    SidebarRail,
    SidebarTrigger,
} from "../sidebar/index.ts";
import { SortableMove } from "../sortable/index.ts";
import { Cluster } from "../cluster/index.ts";
import { Stack } from "../stack/index.ts";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../table/index.ts";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../tabs/index.ts";
import {
    Timeline,
    TimelineBody,
    TimelineGrid,
    TimelineHeader,
    type TimelineItem,
    TimelineLinkLayer,
    TimelineMarker,
    TimelineRowList,
    TimelineNow,
} from "../timeline/index.ts";

/** The studio's time zone, where its days start and its meetings happen. */
const ZONE = "Europe/Lisbon";

/** The day the launch goes out. */
const LAUNCH_DAY = "2026-11-27";

/** A place an app's sidebar leads to. */
interface Place {
    /** Its name. */
    readonly name: string;
    /** The icon that stands for it. */
    readonly icon: JSX.Element;
    /** How many things wait there, when any do. */
    readonly count?: number;
}

/** A group of places under one label in an app's sidebar. */
interface PlaceGroup {
    /** The label over the places. */
    readonly label: string;
    /** The places. */
    readonly places: readonly Place[];
}

/** A person tasks are assigned to. */
interface Person {
    /** Their name. */
    readonly name: string;
    /** Their initials. */
    readonly initials: string;
    /** Their picture, when they have one. */
    readonly picture?: string;
}

/** A task of the launch. */
interface Task {
    /** Its id, as the team quotes it. */
    readonly id: string;
    /** What to do. */
    readonly title: string;
    /** The kind of work, and the swatch its tag takes. */
    readonly tag: readonly [string, Hue];
    /** The person it is assigned to. */
    readonly owner: Person;
    /** The day work on it starts. */
    readonly start: PlainDate;
    /** The day it is due. */
    readonly due: PlainDate;
}

/** The people of the studio. */
export const PEOPLE = {
    ada: { name: "Ada Lovelace", initials: "AL", picture: PICTURES.ada },
    grace: { name: "Grace Hopper", initials: "GH", picture: PICTURES.grace },
    jonas: { name: "Jonas Abiola", initials: "JA" },
} as const satisfies Record<string, Person>;

/** The tags of the launch's work, each with its swatch. */
const TAGS = {
    site: ["Site", "green"],
    story: ["Story", "blue"],
    plan: ["Plan", "purple"],
} as const satisfies Record<string, readonly [string, Hue]>;

/** The number the first new task counts on from, past the launch's own. */
const FIRST_NEW = 20;

/** The launch's tasks by the column they start in. */
const COLUMNS: readonly {
    readonly id: string;
    readonly label: string;
    readonly icon: () => JSX.Element;
    readonly tasks: readonly Task[];
}[] = [
    {
        id: "backlog",
        label: "Backlog",
        icon: () => <Icon name="circle-dashed" xstyle={styles.muted} />,
        tasks: [
            {
                id: "LAU-21",
                title: "Write the changelog",
                tag: TAGS.story,
                owner: PEOPLE.grace,
                start: "2026-11-26",
                due: "2026-11-30",
            },
            {
                id: "LAU-22",
                title: "Collect three quotes",
                tag: TAGS.plan,
                owner: PEOPLE.jonas,
                start: "2026-11-30",
                due: "2026-12-04",
            },
            {
                id: "LAU-23",
                title: "Draft the press note",
                tag: TAGS.story,
                owner: PEOPLE.ada,
                start: "2026-11-27",
                due: "2026-12-02",
            },
        ],
    },
    {
        id: "doing",
        label: "Doing",
        icon: () => <Icon name="circle-half" weight="fill" xstyle={styles.doing} />,
        tasks: [
            {
                id: "LAU-14",
                title: "Record the demo",
                tag: TAGS.story,
                owner: PEOPLE.grace,
                start: "2026-11-23",
                due: "2026-11-26",
            },
            {
                id: "LAU-15",
                title: "Pricing page",
                tag: TAGS.site,
                owner: PEOPLE.ada,
                start: "2026-11-19",
                due: "2026-11-27",
            },
        ],
    },
    {
        id: "review",
        label: "Review",
        icon: () => <Icon name="eye" xstyle={styles.review} />,
        tasks: [
            {
                id: "LAU-11",
                title: "Sign-up flow copy",
                tag: TAGS.site,
                owner: PEOPLE.jonas,
                start: "2026-11-23",
                due: "2026-11-25",
            },
            {
                id: "LAU-12",
                title: "Billing terms",
                tag: TAGS.plan,
                owner: PEOPLE.ada,
                start: "2026-11-24",
                due: "2026-11-26",
            },
        ],
    },
    {
        id: "done",
        label: "Done",
        icon: () => <Icon name="check-circle" weight="fill" xstyle={styles.done} />,
        tasks: [
            {
                id: "LAU-08",
                title: "Set up the domain",
                tag: TAGS.site,
                owner: PEOPLE.ada,
                start: "2026-11-16",
                due: "2026-11-20",
            },
        ],
    },
];

/** The tasks someone is working on or reviewing. */
const IN_PROGRESS = COLUMNS.filter(
    (column) => column.id === "doing" || column.id === "review",
).flatMap((column) => column.tasks);

/** The styles of the apps: their bars, their pages and what they show. */
const styles = style.create({
    bar: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        flexWrap: "wrap",
        columnGap: space[3],
        rowGap: space[1],
        minHeight: size[4],
        paddingBlock: space[1],
        paddingInline: space[4],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    grow: {
        flexGrow: 1,
    },
    page: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        minHeight: 0,
        overflow: "auto",
        padding: space[5],
    },
    brand: {
        fontWeight: weight.medium,
    },
    mark: {
        width: space[5],
        height: space[5],
        borderRadius: radius[2],
        backgroundColor: swatch.accentSolid,
    },
    person: {
        minWidth: 0,
    },
    muted: {
        color: color.mutedForeground,
    },
    code: {
        color: color.mutedForeground,
        fontFamily: font.code,
    },
    views: {
        display: "flex",
        flexDirection: "column",
        gap: space[4],
        minHeight: 0,
    },
    card: {
        paddingBlock: space[1],
    },
    cardTitle: {
        fontWeight: weight.medium,
    },
    doing: {
        color: swatch.warningMark,
    },
    review: {
        color: swatch.infoMark,
    },
    done: {
        color: swatch.successMark,
    },
    document: {
        display: "grid",
        gridTemplateColumns: {
            default: `minmax(0, 44rem) ${width.popover}`,
            "@media (max-width: 60rem)": "minmax(0, 1fr)",
        },
        justifyContent: "center",
        alignItems: "start",
        columnGap: space[8],
        rowGap: space[6],
        paddingBlock: space[8],
    },
    pageHeader: {
        display: "flex",
        flexDirection: "column",
        gridColumn: "1 / -1",
        gap: space[4],
    },
    title: {
        margin: 0,
        fontWeight: weight.semibold,
    },
    prose: {
        maxInlineSize: "none",
    },
    paragraph: {
        margin: 0,
    },
    properties: {
        rowGap: space[3],
    },
    comments: {
        // center the first author on the text's first line
        paddingBlockStart: `calc(${space[1]} / 2)`,
    },
    thread: {
        width: "100%",
        maxWidth: "46rem",
        marginInline: "auto",
    },
    steps: {
        paddingBlock: space[2],
        paddingInlineStart: space[5],
        color: color.mutedForeground,
    },
    found: {
        overflow: "hidden",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
    },
    answer: {
        gap: space[4],
    },
    // fill the page edge to edge, as the rows scroll to it
    conversation: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        minHeight: 0,
        margin: `calc(-1 * ${space[5]})`,
    },
    composer: {
        flexShrink: 0,
        width: "100%",
        maxWidth: "46rem",
        marginInline: "auto",
        paddingInline: space[4],
        paddingBlockEnd: space[4],
    },
});

/** Render a person's picture, or their initials without one. */
export function Face(properties: {
    /** The person the face shows. */
    readonly person: Person;
}): JSX.Element {
    return (
        <Avatar size="sm">
            <Show when={properties.person.picture}>
                {(picture) => <AvatarImage src={picture()} alt={properties.person.name} />}
            </Show>
            <AvatarFallback>{properties.person.initials}</AvatarFallback>
        </Avatar>
    );
}

/** Render an app: its sidebar of places, then a bar and a page beside it. */
export function App(properties: {
    /** The app's name. */
    readonly name: string;
    /** The groups of places its sidebar leads to. */
    readonly groups: readonly PlaceGroup[];
    /** The place the person is at. */
    readonly current: string;
    /** What follows the breadcrumb in the bar. */
    readonly actions?: JSX.Element;
    /** The page. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <SidebarProvider>
            <Sidebar collapsible="icon">
                <SidebarHeader>
                    <Cluster
                        space="2"
                        align="center"
                        xstyle={[text.sm, styles.brand]}
                        render={(attributes) => <span {...attributes} />}
                    >
                        <span aria-hidden="true" {...style.attrs(styles.mark)} />
                        Harbour Studio
                    </Cluster>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <SidebarMenuButton>
                                <Icon name="magnifying-glass" />
                                Search
                            </SidebarMenuButton>
                            <SidebarMenuBadge>
                                <KbdShortcut keys="mod+k" />
                            </SidebarMenuBadge>
                        </SidebarMenuItem>
                    </SidebarMenu>
                </SidebarHeader>
                <SidebarContent>
                    <For each={properties.groups}>
                        {(group) => (
                            <SidebarGroup>
                                <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
                                <SidebarMenu>
                                    <For each={group.places}>
                                        {(place) => (
                                            <SidebarMenuItem>
                                                <SidebarMenuButton
                                                    href={`#${place.name}`}
                                                    active={place.name === properties.current}
                                                >
                                                    {place.icon}
                                                    {place.name}
                                                </SidebarMenuButton>
                                                <Show when={place.count}>
                                                    {(count) => (
                                                        <SidebarMenuBadge>
                                                            {count()}
                                                        </SidebarMenuBadge>
                                                    )}
                                                </Show>
                                            </SidebarMenuItem>
                                        )}
                                    </For>
                                </SidebarMenu>
                            </SidebarGroup>
                        )}
                    </For>
                </SidebarContent>
                <SidebarFooter>
                    <Cluster
                        space="2"
                        align="center"
                        xstyle={[text.sm, styles.person]}
                        render={(attributes) => <span {...attributes} />}
                    >
                        <Face person={PEOPLE.ada} />
                        {PEOPLE.ada.name}
                    </Cluster>
                </SidebarFooter>
                <SidebarRail />
            </Sidebar>
            <SidebarInset>
                <header {...style.attrs(text.sm, styles.bar)}>
                    <SidebarTrigger />
                    <Breadcrumb>
                        <BreadcrumbList>
                            <BreadcrumbItem>
                                <BreadcrumbLink href="#">{properties.name}</BreadcrumbLink>
                            </BreadcrumbItem>
                            <BreadcrumbSeparator />
                            <BreadcrumbItem>
                                <BreadcrumbPage>{properties.current}</BreadcrumbPage>
                            </BreadcrumbItem>
                        </BreadcrumbList>
                    </Breadcrumb>
                    <span {...style.attrs(styles.grow)} />
                    {properties.actions}
                </header>
                <div {...style.attrs(text.sm, styles.page)}>{properties.children}</div>
            </SidebarInset>
        </SidebarProvider>
    );
}

/** Render the launch's tasks as a board whose cards move between columns. */
function TaskBoard(): JSX.Element {
    // follow the tasks of each column and the order of the columns
    const [tasks, setTasks] = createSignal<Record<string, readonly Task[]>>(
        Object.fromEntries(COLUMNS.map((column) => [column.id, column.tasks])),
    );
    const [order, setOrder] = createSignal<readonly string[]>(COLUMNS.map((column) => column.id));
    const columns = () =>
        order().map((id) =>
            present(
                COLUMNS.find((column) => column.id === id),
                "column",
            ),
        );

    return (
        <Board
            label="Launch"
            onItemMove={(move) =>
                setTasks((current) =>
                    BoardMove.apply(
                        current,
                        move,
                        (cell) => cell.column,
                        (task) => task.id,
                    ),
                )
            }
            onColumnMove={(move) =>
                setOrder((current) => SortableMove.place(current, move.id, move, (id) => id))
            }
            onCreate={(draft) =>
                setTasks((current) => {
                    // number the task after the ones on the board
                    const count = Object.values(current).flat().length;
                    const task: Task = {
                        id: `LAU-${String(FIRST_NEW + count)}`,
                        title: draft.title,
                        tag: TAGS.site,
                        owner: PEOPLE.ada,
                        start: "2026-11-30",
                        due: "2026-12-04",
                    };
                    const column = current[draft.column] ?? [];

                    return {
                        ...current,
                        [draft.column]:
                            draft.position === "start" ? [task, ...column] : [...column, task],
                    };
                })
            }
        >
            <BoardColumnList>
                <For each={columns()}>
                    {(column) => (
                        <BoardColumn id={column.id} label={column.label}>
                            <BoardColumnHeader>
                                <BoardColumnHandle />
                                {column.icon()}
                                {column.label}
                                <BoardColumnCount />
                                <BoardColumnMenu />
                                <BoardColumnAdd />
                            </BoardColumnHeader>
                            <BoardComposer position="start" />
                            <BoardColumnContent>
                                <For each={tasks()[column.id]}>
                                    {(task) => (
                                        <BoardItem id={task.id} label={task.title}>
                                            <Stack space="2" xstyle={styles.card}>
                                                <Cluster
                                                    space="2"
                                                    align="center"
                                                    justify="between"
                                                    render={(attributes) => (
                                                        <span {...attributes} />
                                                    )}
                                                >
                                                    <span {...style.attrs(text.xs, styles.code)}>
                                                        {task.id}
                                                    </span>
                                                    <Face person={task.owner} />
                                                </Cluster>
                                                <span {...style.attrs(styles.cardTitle)}>
                                                    {task.title}
                                                </span>
                                                <Cluster
                                                    space="2"
                                                    align="center"
                                                    xstyle={text.xs}
                                                    render={(attributes) => (
                                                        <span {...attributes} />
                                                    )}
                                                >
                                                    <Badge color={task.tag[1]}>{task.tag[0]}</Badge>
                                                    <span {...style.attrs(styles.muted)}>
                                                        <Due day={task.due} />
                                                    </span>
                                                </Cluster>
                                            </Stack>
                                        </BoardItem>
                                    )}
                                </For>
                            </BoardColumnContent>
                            <BoardComposer />
                        </BoardColumn>
                    )}
                </For>
                <BoardHiddenList />
            </BoardColumnList>
        </Board>
    );
}

/** Render a due day as its day and month in the reader's locale. */
function Due(properties: { readonly day: PlainDate }): JSX.Element {
    // write the day the way the reader's locale writes short dates
    const locale = useLocale();

    return <>{locale.plainDate(properties.day, { month: "short", day: "numeric" })}</>;
}

/** Read the instant of a wall time on a day in the studio's time zone. */
function wallTime(day: PlainDate, time: string): number {
    return TimeZone.instant(ZONE, day, time);
}

/** Lay the launch's tasks out as timeline items, each under its tag, the tag spanning its tasks. */
function timelineItems(): TimelineItem[] {
    // gather the tasks of every column
    const tasks = COLUMNS.flatMap((column) => column.tasks);

    // add each tag as a row over its tasks, then the tasks themselves
    return Object.values(TAGS).flatMap(([label, preset]) => {
        // place each task of the tag from its start to the end of its due day
        const tagged = tasks.filter((task) => task.tag[0] === label);
        const children = tagged.map((task) => ({
            key: task.id,
            label: task.title,
            start: wallTime(task.start, "00:00"),
            end: wallTime(PlainDate.add(task.due, { days: 1 }), "00:00"),
            parent: label,
            color: preset,
        }));

        // span the tag's row over its tasks
        const start = Math.min(...children.map((child) => child.start));
        const end = Math.max(...children.map((child) => child.end));

        return [{ key: label, label, start, end }, ...children];
    });
}

/** Render the launch's tasks along the weeks, by tag, with today and the launch day marked. */
function TaskTimeline(): JSX.Element {
    // save the tasks where people leave them
    const [items, setItems] = createSignal<readonly TimelineItem[]>(timelineItems());

    return (
        <Timeline
            items={items()}
            timeZone={ZONE}
            links={[
                { from: "LAU-11", to: "LAU-15", type: "finish-to-start", lag: 0 },
                { from: "LAU-14", to: "LAU-23", type: "finish-to-start", lag: 0 },
            ]}
            onItemsChange={(changed) => {
                // replace each changed item
                const byKey = new Map(changed.map((item) => [item.key, item]));
                setItems((current) => current.map((item) => byKey.get(item.key) ?? item));
            }}
        >
            <TimelineHeader corner="Task" />
            <TimelineBody>
                <TimelineGrid />
                <TimelineNow />
                <TimelineMarker at={wallTime(LAUNCH_DAY, "00:00")}>Launch</TimelineMarker>
                <TimelineLinkLayer />
                <TimelineRowList />
            </TimelineBody>
        </Timeline>
    );
}

/** The studio's launch week: the offsite, the launch and the meetings around it. */
const LAUNCH_WEEK: readonly CalendarEvent[] = [
    {
        key: "offsite",
        title: "Offsite in Lisbon",
        start: "2026-11-23",
        end: "2026-11-25",
        color: "purple",
    },
    { key: "launch-day", title: "Launch day", start: LAUNCH_DAY, end: LAUNCH_DAY, color: "orange" },
    {
        key: "plan",
        title: "Week plan",
        start: wallTime("2026-11-23", "09:00"),
        end: wallTime("2026-11-23", "10:00"),
        color: "blue",
    },
    {
        key: "site",
        title: "Site review",
        start: wallTime("2026-11-23", "13:00"),
        end: wallTime("2026-11-23", "14:30"),
        color: "green",
    },
    {
        key: "demo",
        title: "Record the demo",
        start: wallTime("2026-11-24", "10:00"),
        end: wallTime("2026-11-24", "12:00"),
        color: "blue",
    },
    {
        key: "hiring",
        title: "Hiring sync",
        start: wallTime("2026-11-24", "15:00"),
        end: wallTime("2026-11-24", "16:00"),
        color: "purple",
    },
    {
        key: "standup",
        title: "Standup",
        start: wallTime("2026-11-25", "09:00"),
        end: wallTime("2026-11-25", "09:30"),
    },
    {
        key: "pricing",
        title: "Pricing",
        start: wallTime("2026-11-25", "11:00"),
        end: wallTime("2026-11-25", "12:30"),
        color: "green",
    },
    {
        key: "press",
        title: "Press call",
        start: wallTime("2026-11-25", "11:30"),
        end: wallTime("2026-11-25", "12:30"),
        color: "orange",
    },
    {
        key: "rehearsal",
        title: "Launch rehearsal",
        start: wallTime("2026-11-26", "14:00"),
        end: wallTime("2026-11-26", "16:00"),
        color: "blue",
    },
    {
        key: "launch",
        title: "Launch",
        start: wallTime(LAUNCH_DAY, "09:00"),
        end: wallTime(LAUNCH_DAY, "12:00"),
        color: "green",
    },
];

/** Render the launch week's meetings, opened on the working morning, moved and created in place. */
function TaskCalendar(): JSX.Element {
    // save the events where people leave them, and add the ones they sweep out
    const [events, setEvents] = createSignal<readonly CalendarEvent[]>(LAUNCH_WEEK);

    return (
        <EventCalendar
            events={events()}
            defaultDate="2026-11-25"
            defaultView="week"
            timeZone={ZONE}
            scrollTime="08:00"
            dayMaxEvents={3}
            onEventChange={(changed) =>
                setEvents((current) =>
                    current.map((event) => (event.key === changed.key ? changed : event)),
                )
            }
            onCreate={(span) =>
                setEvents((current) => [
                    ...current,
                    { key: `new-${String(span.start)}`, title: "New event", ...span },
                ])
            }
        >
            <EventCalendarHeader />
            <EventCalendarBody>
                <EventCalendarMonth />
                <EventCalendarTime />
            </EventCalendarBody>
        </EventCalendar>
    );
}

/** Render a comment by the side of a page. */
function Comment(properties: {
    /** The person who wrote the comment. */
    readonly person: Person;
    /** The time it was written. */
    readonly time: string;
    /** The text. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <Message>
            <MessageAvatar position="top">
                <Face person={properties.person} />
            </MessageAvatar>
            <MessageContent>
                <MessageHeader>
                    <MessageAuthor>{properties.person.name}</MessageAuthor>
                    {properties.time}
                </MessageHeader>
                <div {...style.attrs(text.sm)}>{properties.children}</div>
            </MessageContent>
        </Message>
    );
}

/** Render the launch's tasks as a table: one row each, with its tag, owner and due day. */
function TaskTable(): JSX.Element {
    return (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead scope="col">Task</TableHead>
                    <TableHead scope="col">Status</TableHead>
                    <TableHead scope="col">Tag</TableHead>
                    <TableHead scope="col">Owner</TableHead>
                    <TableHead scope="col">Due</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                <For each={COLUMNS}>
                    {(column) => (
                        <For each={column.tasks}>
                            {(task) => (
                                <TableRow>
                                    <TableCell>
                                        <Cluster
                                            space="2"
                                            align="center"
                                            render={(attributes) => <span {...attributes} />}
                                        >
                                            <span {...style.attrs(text.xs, styles.code)}>
                                                {task.id}
                                            </span>
                                            <span {...style.attrs(styles.cardTitle)}>
                                                {task.title}
                                            </span>
                                        </Cluster>
                                    </TableCell>
                                    <TableCell>
                                        <Badge variant="outline">{column.label}</Badge>
                                    </TableCell>
                                    <TableCell>
                                        <Badge color={task.tag[1]}>{task.tag[0]}</Badge>
                                    </TableCell>
                                    <TableCell>
                                        <Cluster
                                            space="2"
                                            align="center"
                                            xstyle={styles.person}
                                            render={(attributes) => <span {...attributes} />}
                                        >
                                            <Face person={task.owner} />
                                            {task.owner.name}
                                        </Cluster>
                                    </TableCell>
                                    <TableCell>
                                        <Due day={task.due} />
                                    </TableCell>
                                </TableRow>
                            )}
                        </For>
                    )}
                </For>
            </TableBody>
        </Table>
    );
}

/** A work tool: a launch's tasks as a board, a table, a timeline and a calendar. */
export const appWork = defineExample({
    of: App,
    name: "work",
    description: "a work tool: a launch's tasks as a board, a table, a timeline and a calendar",
    layout: "fullscreen",
    render: () => (
        <App
            name="Tasks"
            groups={[
                {
                    label: "You",
                    places: [
                        { name: "Inbox", icon: <Icon name="tray" />, count: 4 },
                        { name: "My tasks", icon: <Icon name="check-circle" /> },
                    ],
                },
                {
                    label: "Projects",
                    places: [
                        { name: "Launch", icon: <Icon name="rocket-launch" />, count: 8 },
                        { name: "Site", icon: <Icon name="browser" /> },
                        { name: "Hiring", icon: <Icon name="users" />, count: 2 },
                    ],
                },
            ]}
            current="Launch"
            actions={
                <>
                    <AvatarGroup aria-label="Working on the launch">
                        <Face person={PEOPLE.ada} />
                        <Face person={PEOPLE.grace} />
                        <Face person={PEOPLE.jonas} />
                    </AvatarGroup>
                    <Button variant="outline">
                        <Icon name="funnel" />
                        Filter
                    </Button>
                    <Button>
                        <Icon name="plus" />
                        New task
                    </Button>
                </>
            }
        >
            <Tabs defaultValue="board" xstyle={styles.views}>
                <TabsList aria-label="View">
                    <TabsTrigger value="board">Board</TabsTrigger>
                    <TabsTrigger value="table">Table</TabsTrigger>
                    <TabsTrigger value="timeline">Timeline</TabsTrigger>
                    <TabsTrigger value="calendar">Calendar</TabsTrigger>
                </TabsList>
                <TabsContent value="board">
                    <TaskBoard />
                </TabsContent>
                <TabsContent value="table">
                    <TaskTable />
                </TabsContent>
                <TabsContent value="timeline">
                    <TaskTimeline />
                </TabsContent>
                <TabsContent value="calendar">
                    <TaskCalendar />
                </TabsContent>
            </Tabs>
        </App>
    ),
});

/** A document: a brief with its properties, a decision, a checklist, its tasks and comments. */
export const appDocument = defineExample({
    of: App,
    name: "document",
    description:
        "a document: a brief with its properties, a decision, a checklist, its tasks and comments",
    layout: "fullscreen",
    render: () => (
        <App
            name="Notes"
            groups={[
                {
                    label: "Pages",
                    places: [
                        { name: "Handbook", icon: <Icon name="book-open" /> },
                        { name: "Launch brief", icon: <Icon name="file-text" /> },
                        { name: "Press note", icon: <Icon name="file-text" /> },
                        { name: "Meeting notes", icon: <Icon name="notebook" />, count: 3 },
                    ],
                },
            ]}
            current="Launch brief"
            actions={
                <>
                    <span {...style.attrs(text.xs, styles.muted)}>Edited 4 minutes ago</span>
                    <Button variant="outline">Share</Button>
                </>
            }
        >
            <article {...style.attrs(styles.document)}>
                {/* Icon, title and properties */}
                <header {...style.attrs(styles.pageHeader)}>
                    <span aria-hidden="true" {...style.attrs(text["5xl"])}>
                        🚀
                    </span>
                    <h1 {...style.attrs(text["4xl"], styles.title)}>Launch brief</h1>
                    <DataList xstyle={styles.properties}>
                        <DataListItem>
                            <DataListLabel>Status</DataListLabel>
                            <DataListValue>
                                <Badge color="blue">In review</Badge>
                            </DataListValue>
                        </DataListItem>
                        <DataListItem align="center">
                            <DataListLabel>Owner</DataListLabel>
                            <DataListValue>
                                <Cluster
                                    space="2"
                                    align="center"
                                    xstyle={styles.person}
                                    render={(attributes) => <span {...attributes} />}
                                >
                                    <Face person={PEOPLE.ada} />
                                    {PEOPLE.ada.name}
                                </Cluster>
                            </DataListValue>
                        </DataListItem>
                        <DataListItem>
                            <DataListLabel>Launch day</DataListLabel>
                            <DataListValue>Friday 27 November</DataListValue>
                        </DataListItem>
                    </DataList>
                </header>

                {/* Text */}
                <Prose xstyle={styles.prose}>
                    <p>
                        We open sign-ups on Friday. The page, the demo and the first email go out
                        together, and the team watches the first day in one room.
                    </p>
                    <Alert>
                        <Icon name="lightbulb" />
                        <AlertTitle>Decision</AlertTitle>
                        <AlertDescription>
                            No discount on launch day. The price on the page is the price.
                        </AlertDescription>
                    </Alert>
                    <h2>What has to be true</h2>
                    <Stack space="3">
                        <Label xstyle={text.base}>
                            <Checkbox name="domain" defaultChecked /> The domain resolves everywhere
                        </Label>
                        <Label xstyle={text.base}>
                            <Checkbox name="pricing" /> The pricing page says what each plan costs
                        </Label>
                        <Label xstyle={text.base}>
                            <Checkbox name="demo" /> The demo is under two minutes
                        </Label>
                    </Stack>
                    <h2>Open tasks</h2>
                    <TaskTable />
                </Prose>
                {/* Comments */}
                <Stack
                    space="5"
                    aria-label="Comments"
                    xstyle={styles.comments}
                    render={(attributes) => <aside {...attributes} />}
                >
                    <Comment person={PEOPLE.jonas} time="10:42">
                        Monthly and yearly, or only monthly on day one?
                    </Comment>
                    <Comment person={PEOPLE.grace} time="Yesterday">
                        The demo is at 2:10 now. Cutting the intro.
                    </Comment>
                    <Message>
                        <MessageAvatar position="top">
                            <Face person={PEOPLE.ada} />
                        </MessageAvatar>
                        <MessageContent>
                            <Composer onSubmit={() => {}}>
                                <ComposerFrame>
                                    <ComposerInput placeholder="Reply" aria-label="Reply" />
                                    <ComposerAddon align="inline-end">
                                        <ComposerSubmit />
                                    </ComposerAddon>
                                </ComposerFrame>
                            </Composer>
                        </MessageContent>
                    </Message>
                </Stack>
            </article>
        </App>
    ),
});

/** A conversation: a question, what the answer read and found, and the box to ask again. */
export const appConversation = defineExample({
    of: App,
    name: "conversation",
    description:
        "a conversation: a question, what the answer read, the tasks it found, and the box to ask again",
    layout: "fullscreen",
    render: () => (
        <App
            name="Assistant"
            groups={[
                {
                    label: "Today",
                    places: [
                        { name: "What is left for launch", icon: <Icon name="chat-circle" /> },
                        { name: "Draft the press note", icon: <Icon name="chat-circle" /> },
                    ],
                },
                {
                    label: "This week",
                    places: [
                        { name: "Plan names", icon: <Icon name="chat-circle" /> },
                        { name: "Offsite travel", icon: <Icon name="chat-circle" /> },
                    ],
                },
            ]}
            current="What is left for launch"
            actions={
                <Button variant="outline">
                    <Icon name="plus" />
                    New conversation
                </Button>
            }
        >
            <div {...style.attrs(styles.conversation)}>
                <MessageScroller>
                    <MessageScrollerViewport>
                        <MessageScrollerContent xstyle={styles.thread}>
                            <MessageScrollerItem messageId="question" scrollAnchor>
                                <Message align="end">
                                    <MessageContent>
                                        <Bubble variant="secondary">
                                            <BubbleContent>
                                                What is still open for the launch on Friday, and who
                                                has it?
                                            </BubbleContent>
                                        </Bubble>
                                    </MessageContent>
                                </Message>
                            </MessageScrollerItem>
                            <MessageScrollerItem messageId="steps">
                                <Collapsible>
                                    <Marker
                                        render={(attributes) => (
                                            <CollapsibleTrigger {...attributes} />
                                        )}
                                    >
                                        <MarkerIcon>
                                            <Icon name="magnifying-glass" />
                                        </MarkerIcon>
                                        <MarkerContent>Read 8 tasks in Launch</MarkerContent>
                                    </Marker>
                                    <CollapsibleContent xstyle={[text.sm, styles.steps]}>
                                        Searched Tasks for the Launch project, then read the tasks
                                        that are not done.
                                    </CollapsibleContent>
                                </Collapsible>
                            </MessageScrollerItem>
                            <MessageScrollerItem messageId="answer">
                                <Message>
                                    <MessageContent xstyle={styles.answer}>
                                        <p {...style.attrs(text.base, styles.paragraph)}>
                                            Four tasks are in progress or in review. One is at risk:
                                        </p>
                                        <div {...style.attrs(styles.found)}>
                                            <Table>
                                                <TableBody>
                                                    <For each={IN_PROGRESS}>
                                                        {(task) => (
                                                            <TableRow>
                                                                <TableCell>
                                                                    <Cluster
                                                                        space="2"
                                                                        align="center"
                                                                        render={(attributes) => (
                                                                            <span {...attributes} />
                                                                        )}
                                                                    >
                                                                        <span
                                                                            {...style.attrs(
                                                                                text.xs,
                                                                                styles.code,
                                                                            )}
                                                                        >
                                                                            {task.id}
                                                                        </span>
                                                                        <span
                                                                            {...style.attrs(
                                                                                styles.cardTitle,
                                                                            )}
                                                                        >
                                                                            {task.title}
                                                                        </span>
                                                                    </Cluster>
                                                                </TableCell>
                                                                <TableCell>
                                                                    <Cluster
                                                                        space="2"
                                                                        align="center"
                                                                        xstyle={styles.person}
                                                                        render={(attributes) => (
                                                                            <span {...attributes} />
                                                                        )}
                                                                    >
                                                                        <Face person={task.owner} />
                                                                        {task.owner.name}
                                                                    </Cluster>
                                                                </TableCell>
                                                                <TableCell>
                                                                    <Show
                                                                        when={task.id === "LAU-14"}
                                                                        fallback={
                                                                            <Badge
                                                                                color={task.tag[1]}
                                                                            >
                                                                                {task.tag[0]}
                                                                            </Badge>
                                                                        }
                                                                    >
                                                                        <Badge color="red">
                                                                            At risk
                                                                        </Badge>
                                                                    </Show>
                                                                </TableCell>
                                                            </TableRow>
                                                        )}
                                                    </For>
                                                </TableBody>
                                            </Table>
                                        </div>
                                        <p {...style.attrs(text.base, styles.paragraph)}>
                                            The demo runs 2:10 against a two-minute limit. Grace is
                                            cutting the intro today.
                                        </p>
                                        <Cluster space="2">
                                            <Button variant="outline">Open the board</Button>
                                            <Button variant="outline">Remind the owners</Button>
                                        </Cluster>
                                        <MessageFooter>
                                            <MessageToolbar
                                                placement="inline"
                                                autohide="not-last"
                                                aria-label="Actions"
                                            >
                                                <ToolbarButton
                                                    variant="ghost"
                                                    size="icon-sm"
                                                    aria-label="Copy"
                                                >
                                                    <Icon name="copy" />
                                                </ToolbarButton>
                                                <ToolbarButton
                                                    variant="ghost"
                                                    size="icon-sm"
                                                    aria-label="Good answer"
                                                >
                                                    <Icon name="thumbs-up" />
                                                </ToolbarButton>
                                                <ToolbarButton
                                                    variant="ghost"
                                                    size="icon-sm"
                                                    aria-label="Answer again"
                                                >
                                                    <Icon name="arrows-clockwise" />
                                                </ToolbarButton>
                                            </MessageToolbar>
                                        </MessageFooter>
                                    </MessageContent>
                                </Message>
                            </MessageScrollerItem>
                        </MessageScrollerContent>
                    </MessageScrollerViewport>
                    <MessageScrollerButton />
                </MessageScroller>
                <div {...style.attrs(styles.composer)}>
                    <Composer onSubmit={() => {}}>
                        <ComposerFrame>
                            <ComposerInput placeholder="Ask about anything in this space" />
                            <ComposerFooter>
                                <Button
                                    variant="ghost"
                                    size="icon"
                                    shape="round"
                                    aria-label="Attach a file"
                                >
                                    <Icon name="plus" />
                                </Button>
                                <Badge color="orange">Launch</Badge>
                                <ComposerSubmit />
                            </ComposerFooter>
                        </ComposerFrame>
                    </Composer>
                </div>
            </div>
        </App>
    ),
});
