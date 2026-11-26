import caretDown from "@destack/icon/phosphor/caret-down";
import plusIcon from "@destack/icon/phosphor/plus";
import type { JsonCondition } from "@destack/db";
import { creating, Place } from "@destack/object";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, shadow, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import {
    createMemo,
    createSignal,
    For,
    handle,
    type JSX,
    omit,
    provide,
    Show,
    useActions,
} from "@destack/view";
import { ActionButton, ChoiceIcon } from "../../action/index.ts";
import { ActionBar, ActionBarClear, ActionBarCount, ActionBarMenu } from "../../action/index.ts";
import { Avatar, AvatarFallback } from "../../avatar/index.ts";
import {
    Board,
    BoardColumn,
    BoardColumnAdd,
    BoardColumnContent,
    BoardColumnCount,
    BoardColumnHeader,
    BoardColumnList,
    BoardComposer,
    BoardHiddenList,
    BoardItem,
} from "../../board/index.ts";
import { Badge } from "../../badge/index.ts";
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "../../breadcrumb/index.ts";
import { Button } from "../../button/index.ts";
import { Card, CardContent } from "../../card/index.ts";
import { Checkbox } from "../../checkbox/index.ts";
import { Cluster } from "../../cluster/index.ts";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "../../empty/index.ts";
import {
    Filter,
    FilterAdd,
    FilterClear,
    type FilterFields,
    FilterList,
    FilterMatch,
} from "../../filter/index.ts";
import {
    GridList,
    GridListCell,
    type GridListCellProperties,
    GridListControl,
    GridListEmpty,
    GridListProvider,
    GridListSectionHeader,
    type GridListSectionHeaderProperties,
    GridListViewport,
} from "../../grid-list/index.ts";
import { Icon } from "@destack/ui/icon";
import { Stack } from "../../stack/index.ts";
import { SidebarTrigger } from "../../sidebar/index.ts";
import { assign, createTask, peek, setPriority, setStatus } from "./actions.example.ts";
import { Display } from "./display.tsx";
import { taskType } from "./object.example.ts";
import { Due, Face } from "./shell.tsx";
import {
    choiceOf,
    isLate,
    LABELS,
    PEOPLE,
    personOf,
    PRIORITIES,
    PROJECTS,
    referenceOf,
    STATUSES,
    type Studio,
    type Task,
    type TaskField,
    type TaskSection,
} from "./state.ts";

/** The height of a row and of a heading, in pixels. */
const ROW_HEIGHT = 40;

/** The height of a card, in pixels, until it is measured. */
const CARD_HEIGHT = 104;

/** The styles of the tasks. */
const styles = style.create({
    screen: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        minHeight: 0,
        position: "relative",
    },
    bar: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        columnGap: space[2],
        minHeight: `calc(${size[2]} + ${space[2]})`,
        paddingInline: space[3],
    },
    ruled: {
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    grow: { flexGrow: 1 },
    filter: {
        flexShrink: 0,
    },
    views: {
        display: "flex",
        flexGrow: 1,
        gap: space[1],
        minWidth: 0,
        overflowX: "auto",
        scrollbarWidth: "none",
    },
    conditions: {
        flexWrap: "wrap",
        paddingBlock: space[1],
    },
    body: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        minHeight: 0,
        position: "relative",
    },
    board: {
        overflow: "hidden",
        padding: space[3],
    },
    fill: {
        flexGrow: 1,
        minHeight: 0,
        height: "100%",
    },
    columns: {
        minHeight: 0,
    },
    muted: {
        color: color.mutedForeground,
    },
    cardTitle: {
        fontWeight: weight.medium,
    },
    peek: {
        position: "absolute",
        insetBlockStart: space[3],
        insetInlineEnd: space[4],
        zIndex: 3,
        width: "22rem",
        maxWidth: `calc(100% - 2 * ${space[4]})`,
        boxShadow: shadow.overlay,
        pointerEvents: "none",
    },
    peekTitle: {
        fontWeight: weight.semibold,
    },
    viewport: {
        flexGrow: 1,
        minHeight: 0,
        maxHeight: "none",
        paddingInline: space[2],
        paddingBlockEnd: space[8],
    },
    heading: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        height: `${String(ROW_HEIGHT)}px`,
        paddingInline: space[2],
        borderRadius: radius[2],
        // cover the rows that scroll under it
        backgroundColor: `color-mix(in oklab, ${color.muted}, ${color.background} 30%)`,
        color: color.foreground,
        fontWeight: weight.medium,
    },
    inner: {
        paddingInlineStart: space[6],
        backgroundColor: color.background,
    },
    count: {
        color: color.mutedForeground,
        fontWeight: weight.regular,
    },
    caret: {
        color: color.mutedForeground,
    },
    folded: {
        transform: "rotate(-90deg)",
    },
    row: {
        display: "flex",
        alignItems: "center",
        justifyContent: "flex-start",
        gap: space[2],
        width: "100%",
        height: `${String(ROW_HEIGHT)}px`,
        paddingInline: space[2],
        cursor: "default",
    },
    check: {
        display: "inline-flex",
        flexShrink: 0,
        opacity: {
            default: 0,
            ":is([data-item]:hover > *, [data-item]:focus > *, [data-selected] > *)": 1,
        },
    },
    icon: {
        flexShrink: 0,
    },
    id: {
        display: { default: null, "@media (max-width: 40rem)": "none" },
        flexShrink: 0,
        whiteSpace: "nowrap",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    // align the titles after the longest id, and the letters wider than digits before it
    idWidth: (characters: number) => ({ minWidth: `${String(characters + 1)}ch` }),
    title: {
        flexShrink: 1,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
        fontWeight: weight.medium,
    },
    nested: {
        paddingInlineStart: `calc(${space[2]} + ${space[6]})`,
    },
    tags: {
        display: { default: "flex", "@media (max-width: 40rem)": "none" },
        flexShrink: 0,
        gap: space[1],
    },
    face: {
        display: "inline-flex",
        flexShrink: 0,
        color: color.mutedForeground,
    },
});

/** The fields a person filters tasks by. */
const FIELDS: FilterFields = {
    title: { label: "Title", type: { kind: "text" } },
    status: { label: "Status", type: { kind: "choice" }, choices: STATUSES },
    priority: { label: "Priority", type: { kind: "choice" }, choices: PRIORITIES },
    assignee: {
        label: "Assignee",
        type: { kind: "person" },
        choices: PEOPLE.map(({ id, name }) => ({ value: id, title: name })),
    },
    project: { label: "Project", type: { kind: "choice" }, choices: PROJECTS },
};

/** Render a view: its name and filter, and its tasks as rows or as cards. */
export function Tasks(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** The view's id. */
    readonly view: string;
    /** Go to another view. */
    readonly onGo: (view: string) => void;
    /** Open a task. */
    readonly onOpen: (task: string) => void;
}): JSX.Element {
    const { studio } = properties;
    const view = () => studio.view(properties.view);

    return (
        <div {...style.attrs(text.sm, styles.screen)}>
            {/* Where the person is */}
            <header {...style.attrs(styles.bar, styles.ruled)}>
                <SidebarTrigger />
                <Breadcrumb>
                    <BreadcrumbList>
                        <BreadcrumbItem>
                            <BreadcrumbLink
                                href="#tasks"
                                onClick={(event) => {
                                    event.preventDefault();
                                    properties.onGo("all");
                                }}
                            >
                                Tasks
                            </BreadcrumbLink>
                        </BreadcrumbItem>
                        <BreadcrumbSeparator />
                        <BreadcrumbItem>
                            <BreadcrumbPage>{view().name}</BreadcrumbPage>
                        </BreadcrumbItem>
                    </BreadcrumbList>
                </Breadcrumb>
                <span {...style.attrs(styles.grow)} />
                <ActionButton action={createTask} icon={plusIcon} variant="ghost" size="sm" />
            </header>

            {/* The views, and what they show */}
            <Filter
                fields={FIELDS}
                value={view().where}
                onValueChange={(where: JsonCondition) => studio.arrange(properties.view, { where })}
                render={(attributes) => <div {...attributes} {...style.attrs(styles.filter)} />}
            >
                <div {...style.attrs(styles.bar, styles.ruled)}>
                    <div {...style.attrs(styles.views)}>
                        <For each={studio.views()} keyed={(each) => each.id}>
                            {(each) => (
                                <Button
                                    size="sm"
                                    variant={each().id === properties.view ? "secondary" : "ghost"}
                                    aria-pressed={each().id === properties.view ? "true" : "false"}
                                    onClick={() => properties.onGo(each().id)}
                                >
                                    {each().name}
                                </Button>
                            )}
                        </For>
                    </div>
                    <FilterAdd />
                    <Display studio={studio} view={properties.view} />
                </div>
                <Show when={Object.keys(view().where).length > 0}>
                    <div {...style.attrs(styles.bar, styles.ruled, styles.conditions)}>
                        <FilterList />
                        <FilterMatch />
                        <FilterClear />
                    </div>
                </Show>
            </Filter>

            {/* The tasks */}
            <Show
                when={view().layout === "board"}
                fallback={
                    <TaskList studio={studio} view={properties.view} onOpen={properties.onOpen} />
                }
            >
                <TaskBoard studio={studio} view={properties.view} onOpen={properties.onOpen} />
            </Show>
        </div>
    );
}

/** What a view's tasks are rendered with. */
interface TaskListProperties {
    /** The studio. */
    readonly studio: Studio;
    /** The view's id. */
    readonly view: string;
    /** Open a task. */
    readonly onOpen: (task: string) => void;
}

/** Read the key of the group around a group in it. */
function outerOf(key: string): string {
    return key.slice(0, key.lastIndexOf("/"));
}

/** Render a view's tasks as rows under their groups. */
function TaskList(properties: TaskListProperties): JSX.Element {
    // list the view's tasks by group, those of collapsed groups left out
    const { studio } = properties;
    const view = () => studio.view(properties.view);
    const groups = createMemo(() => studio.sections(view()));
    const groupOf = (key: string): TaskSection | undefined =>
        groups().find((group) => group.key === key);
    const isCollapsed = (key: string) => view().collapsed.includes(key);
    const [isPeeking, setPeeking] = createSignal(false, { ownedWrite: true });
    const idLength = createMemo(() =>
        Math.max(0, ...groups().flatMap(({ tasks }) => tasks.map(({ id }) => id.length))),
    );
    const list = new GridListControl<Task>({
        sections: () =>
            groups()
                // hide what is in a collapsed group
                .filter(({ key, isInner }) => !isInner || !isCollapsed(outerOf(key)))
                .map(({ key, choice, tasks }) => ({
                    key,
                    label: choice.title,
                    items: isCollapsed(key) ? [] : tasks,
                })),
        key: (task) => task.id,
        text: (task) => task.title,
        selection: { multiple: true, behavior: "replace" },
        rowHeight: ROW_HEIGHT,
        hasEmptySections: true,
        hasStickyHeadings: true,
        shouldFocusOnHover: true,
        get order() {
            return Object.keys(view().orderBy).length === 0 ? "manual" : "sorted";
        },
        reference: referenceOf,
        onAction: (task) => properties.onOpen(task.id),
        onMove: ({ id, section, next }) => studio.move(id, groupOf(section)?.patch ?? {}, next),
    });

    // collapse a group, or expand it
    const toggle = (key: string) => {
        const { collapsed } = view();
        studio.arrange(properties.view, {
            collapsed: isCollapsed(key)
                ? collapsed.filter((each) => each !== key)
                : [...collapsed, key],
        });
    };
    const peeked = () => (isPeeking() ? list.active() : undefined);

    return (
        <div
            ref={handle(peek, {
                isEnabled: (element) => list.keyAt(element) !== undefined,
                run: () => void setPeeking((is) => !is),
            })}
            {...style.attrs(styles.body)}
        >
            <GridListProvider control={list}>
                <GridListViewport xstyle={styles.viewport}>
                    <GridListEmpty>
                        <Empty>
                            <EmptyHeader>
                                <EmptyTitle>No tasks</EmptyTitle>
                                <EmptyDescription>No task meets the filter.</EmptyDescription>
                            </EmptyHeader>
                        </Empty>
                    </GridListEmpty>
                    <GridList
                        control={list}
                        aria-label={view().name}
                        label={(task) => task.title}
                        sectionHeader={(heading) => (
                            <TaskHeading
                                {...heading}
                                group={groupOf(heading.section)}
                                isCollapsed={isCollapsed(heading.section)}
                                onToggle={() => toggle(heading.section)}
                            />
                        )}
                    >
                        {(cell) => (
                            <TaskRow
                                {...cell}
                                fields={view().fields}
                                idLength={idLength()}
                                onPick={(isRange: boolean) =>
                                    list.selection?.select(
                                        cell.item.id,
                                        { shiftKey: isRange, metaKey: true, ctrlKey: false },
                                        list.collection,
                                    )
                                }
                            />
                        )}
                    </GridList>
                </GridListViewport>
            </GridListProvider>

            {/* The task the person looks at */}
            <Show when={peeked()}>{(task) => <Peek task={task()} />}</Show>

            {/* The actions on the selected tasks */}
            <ActionBar
                items={list.selection?.values() ?? []}
                reference={referenceOf}
                onClear={() => list.selection?.clear()}
            >
                <ActionBarCount />
                <ActionButton action={setStatus} variant="ghost" size="sm" />
                <ActionButton action={setPriority} variant="ghost" size="sm" />
                <ActionButton action={assign} variant="ghost" size="sm" />
                <ActionBarMenu />
                <ActionBarClear />
            </ActionBar>
        </div>
    );
}

/** Render a view's tasks as cards in columns, one per group. */
function TaskBoard(properties: TaskListProperties): JSX.Element {
    // list the tasks by group, the empty ones hidden at the end
    const { studio } = properties;
    const view = () => studio.view(properties.view);
    const groups = createMemo(() =>
        studio.sections({ ...view(), subgroup: null, hasEmptyGroups: true }),
    );
    const hidden = () =>
        groups()
            .filter(({ tasks }) => tasks.length === 0)
            .map(({ key }) => key);
    const patchOf = (key: string) => groups().find((group) => group.key === key)?.patch ?? {};
    const [selected, setSelected] = createSignal<readonly string[]>([], { ownedWrite: true });

    return (
        <div {...style.attrs(styles.body, styles.board)}>
            <Board
                label={view().name}
                xstyle={styles.fill}
                order={Object.keys(view().orderBy).length === 0 ? "manual" : "sorted"}
                selection={{
                    multiple: true,
                    behavior: "replace",
                    value: selected(),
                    onValueChange: (values) => void setSelected(values),
                }}
                hiddenColumns={hidden()}
                reference={referenceOf}
                onAction={(task) => properties.onOpen(task)}
                onItemMove={({ id, column, next }) => studio.move(id, patchOf(column), next)}
                onCreate={({ column, title }) => studio.create({ title, ...patchOf(column) })}
            >
                <BoardColumnList xstyle={styles.columns}>
                    <For each={groups()} keyed={({ key }) => key}>
                        {(group) => (
                            <BoardColumn id={group().key} label={group().choice.title}>
                                <BoardColumnHeader>
                                    <GroupIcon group={group()} />
                                    {group().choice.title}
                                    <BoardColumnCount />
                                    <BoardColumnAdd />
                                </BoardColumnHeader>
                                <BoardComposer position="start" />
                                <BoardColumnContent
                                    each={group().tasks}
                                    itemKey={(task) => task.id}
                                    estimateSize={() => CARD_HEIGHT}
                                    textOf={(id) => studio.find(id)?.title ?? ""}
                                >
                                    {(task) => <TaskCard task={task} fields={view().fields} />}
                                </BoardColumnContent>
                                <BoardComposer />
                            </BoardColumn>
                        )}
                    </For>
                    <BoardHiddenList />
                </BoardColumnList>
            </Board>
        </div>
    );
}

/** Render a task as a card. */
function TaskCard(properties: {
    /** The task. */
    readonly task: Task;
    /** The properties it shows. */
    readonly fields: readonly TaskField[];
}): JSX.Element {
    const task = () => properties.task;
    const has = (field: TaskField) => properties.fields.includes(field);

    return (
        <BoardItem id={task().id} label={task().title}>
            <Stack space="2">
                <Cluster space="2" align="center" justify="between">
                    <span {...style.attrs(text.xs, styles.muted)}>{task().id}</span>
                    <Show when={personOf(task().assignee)}>
                        {(person) => <Face person={person()} />}
                    </Show>
                </Cluster>
                <span {...style.attrs(styles.cardTitle)}>{task().title}</span>
                <Cluster space="2" align="center">
                    <Show when={has("priority") && task().priority !== "none"}>
                        <ChoiceIcon choice={choiceOf(PRIORITIES, task().priority)} />
                    </Show>
                    <Show when={has("labels")}>
                        <For each={task().labels}>
                            {(label) => <Tag choice={choiceOf(LABELS, label)} />}
                        </For>
                    </Show>
                    <Show when={has("due") ? task().due : null}>
                        {(due) => <Due day={due()} isLate={isLate(task())} />}
                    </Show>
                </Cluster>
            </Stack>
        </BoardItem>
    );
}

/** Render what a task says beside the list, which the person moves through. */
function Peek(properties: {
    /** The task. */
    readonly task: Task;
}): JSX.Element {
    const task = () => properties.task;

    return (
        <Card xstyle={styles.peek}>
            <CardContent>
                <Stack space="3">
                    <span {...style.attrs(styles.muted)}>{task().id}</span>
                    <strong {...style.attrs(text.base, styles.peekTitle)}>{task().title}</strong>
                    <Cluster space="3" align="center">
                        <Cluster space="1" align="center">
                            <ChoiceIcon choice={choiceOf(STATUSES, task().status)} />
                            {choiceOf(STATUSES, task().status).title}
                        </Cluster>
                        <Cluster space="1" align="center">
                            <ChoiceIcon choice={choiceOf(PRIORITIES, task().priority)} />
                            {choiceOf(PRIORITIES, task().priority).title}
                        </Cluster>
                    </Cluster>
                    <span {...style.attrs(styles.muted)}>
                        {task().text === "" ? "No description" : task().text}
                    </span>
                </Stack>
            </CardContent>
        </Card>
    );
}

/** Render the heading of a group: its value, how many tasks it has, and whether they show. */
function TaskHeading(
    properties: GridListSectionHeaderProperties & {
        /** The group. */
        readonly group: TaskSection | undefined;
        /** Whether its tasks are hidden. */
        readonly isCollapsed: boolean;
        /** Hide its tasks, or show them. */
        readonly onToggle: () => void;
    },
): JSX.Element {
    const rest = omit(properties, "group", "isCollapsed", "onToggle", "xstyle");

    return (
        <GridListSectionHeader
            {...rest}
            xstyle={[
                text.sm,
                styles.heading,
                properties.group?.isInner === true && styles.inner,
                properties.xstyle,
            ]}
        >
            <Button
                size="icon-sm"
                variant="ghost"
                aria-label={properties.label}
                aria-expanded={properties.isCollapsed ? "false" : "true"}
                onClick={() => properties.onToggle()}
            >
                <Icon
                    icon={caretDown}
                    xstyle={[styles.caret, properties.isCollapsed && styles.folded]}
                />
            </Button>
            <Show when={properties.group}>
                {(group) => (
                    <>
                        <GroupIcon group={group()} />
                        <span role="columnheader">{group().choice.title}</span>
                        <span {...style.attrs(styles.count)}>{group().count}</span>
                        <span {...style.attrs(styles.grow)} />
                        <span
                            ref={provide(() => new Place().with(creating(taskType), group().patch))}
                        >
                            <ActionButton
                                action={createTask}
                                icon={plusIcon}
                                size="icon-sm"
                                variant="ghost"
                            />
                        </span>
                    </>
                )}
            </Show>
        </GridListSectionHeader>
    );
}

/** Render the icon of a group's value: a person's picture, or the value's icon. */
function GroupIcon(properties: {
    /** The group. */
    readonly group: TaskSection;
}): JSX.Element {
    return (
        <Show
            when={personOf(properties.group.choice.value)}
            fallback={<ChoiceIcon choice={properties.group.choice} />}
        >
            {(person) => <Face person={person()} />}
        </Show>
    );
}

/** Render a task as a row: its check, properties and title. */
function TaskRow(
    properties: GridListCellProperties<Task> & {
        /** The properties it shows. */
        readonly fields: readonly TaskField[];
        /** The length of the longest id. */
        readonly idLength: number;
        /** Select the task, or the range up to it. */
        readonly onPick: (isRange: boolean) => void;
    },
): JSX.Element {
    // read the task, and the properties it shows
    const actions = useActions();
    const task = () => properties.item;
    const rest = omit(properties, "item", "fields", "idLength", "onPick");
    const has = (field: TaskField) => properties.fields.includes(field);

    return (
        <GridListCell {...rest} xstyle={[styles.row, task().parent !== null && styles.nested]}>
            <span {...style.attrs(styles.check)}>
                <Checkbox
                    tabindex={-1}
                    aria-label={task().title}
                    checked={properties.isSelected}
                    onCheckedChange={() => properties.onPick(false)}
                />
            </span>
            <Show when={has("priority")}>
                <Button
                    size="icon-sm"
                    variant="ghost"
                    tabindex={-1}
                    aria-label={choiceOf(PRIORITIES, task().priority).title}
                    xstyle={styles.icon}
                    onClick={(event) => actions.start(setPriority, event.currentTarget)}
                >
                    <ChoiceIcon choice={choiceOf(PRIORITIES, task().priority)} />
                </Button>
            </Show>
            <Show when={has("id")}>
                <span {...style.attrs(styles.id, styles.idWidth(properties.idLength))}>
                    {task().id}
                </span>
            </Show>
            <Button
                size="icon-sm"
                variant="ghost"
                tabindex={-1}
                aria-label={choiceOf(STATUSES, task().status).title}
                xstyle={styles.icon}
                onClick={(event) => actions.start(setStatus, event.currentTarget)}
            >
                <ChoiceIcon choice={choiceOf(STATUSES, task().status)} />
            </Button>
            <span {...style.attrs(styles.title)}>{task().title}</span>
            <span {...style.attrs(styles.grow)} />
            <Show when={has("labels")}>
                <span {...style.attrs(styles.tags)}>
                    <For each={task().labels}>
                        {(label) => <Tag choice={choiceOf(LABELS, label)} />}
                    </For>
                </span>
            </Show>
            <Show when={has("due") ? task().due : null}>
                {(due) => <Due day={due()} isLate={isLate(task())} />}
            </Show>
            <Show when={has("assignee")}>
                <span {...style.attrs(styles.face)}>
                    <Show
                        when={personOf(task().assignee)}
                        fallback={
                            <Avatar size="sm">
                                <AvatarFallback>
                                    <Icon name="user" />
                                </AvatarFallback>
                            </Avatar>
                        }
                    >
                        {(person) => <Face person={person()} />}
                    </Show>
                </span>
            </Show>
        </GridListCell>
    );
}

/** Render a label as a tag in its color. */
function Tag(properties: {
    /** The label. */
    readonly choice: ReturnType<typeof choiceOf>;
}): JSX.Element {
    return (
        <Show when={properties.choice.color} fallback={<Badge>{properties.choice.title}</Badge>}>
            {(hue) => <Badge color={hue()}>{properties.choice.title}</Badge>}
        </Show>
    );
}
