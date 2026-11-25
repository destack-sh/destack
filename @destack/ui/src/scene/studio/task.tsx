import plusIcon from "@destack/icon/phosphor/plus";
import { Place } from "@destack/object";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import { createMemo, createSignal, For, type JSX, provide, Show } from "@destack/view";
import { ActionDialog, ActionSelect, ChoiceIcon } from "../../action/index.ts";
import { Activity, ActivityBadge, ActivityBody, ActivityItem } from "../../activity/index.ts";
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
import { DataList, DataListItem, DataListLabel, DataListValue } from "../../data-list/index.ts";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogFooter,
    DialogTitle,
} from "../../dialog/index.ts";
import { Editable } from "../../editable/index.ts";
import { Input } from "../../input/index.ts";
import { Item, ItemContent, ItemGroup, ItemTitle } from "../../item/index.ts";
import { Label } from "../../label/index.ts";
import { RelativeTime } from "../../relative-time/index.ts";
import { SidebarTrigger } from "../../sidebar/index.ts";
import { Stack } from "../../stack/index.ts";
import { Switch } from "../../switch/index.ts";
import { Textarea } from "../../textarea/index.ts";
import {
    assign,
    handleTasks,
    setDue,
    setPriority,
    setProject,
    setStatus,
    toggleLabel,
} from "./actions.example.ts";
import { CommentBox, CommentCard } from "./comment.tsx";
import { Due, Face } from "./shell.tsx";
import {
    choiceOf,
    isLate,
    personOf,
    PRIORITIES,
    PROJECTS,
    referenceOf,
    STATUSES,
    Studio,
    taskOf,
    type Task,
    type TaskEvent,
    type TaskPatch,
} from "./state.ts";

/** The styles of a task. */
const styles = style.create({
    screen: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        minHeight: 0,
    },
    bar: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        columnGap: space[2],
        minHeight: `calc(${size[2]} + ${space[2]})`,
        paddingInline: space[3],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    page: {
        display: "grid",
        gridTemplateColumns: {
            default: "minmax(0, 44rem) 15rem",
            "@media (max-width: 64rem)": "minmax(0, 1fr)",
        },
        justifyContent: "center",
        alignContent: "start",
        columnGap: space[8],
        rowGap: space[6],
        flexGrow: 1,
        minHeight: 0,
        overflow: "auto",
        paddingBlock: space[7],
        paddingInline: space[5],
    },
    title: {
        margin: 0,
        fontWeight: weight.semibold,
    },
    heading: {
        margin: 0,
        fontWeight: weight.semibold,
    },
    muted: {
        color: color.mutedForeground,
    },
    value: {
        justifyContent: "flex-start",
        maxWidth: "100%",
        marginInlineStart: `calc(-1 * ${space[2]})`,
    },
    properties: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[2],
    },
    name: {
        fontWeight: weight.semibold,
    },
    grow: {
        flexGrow: 1,
    },
});

/** Render a task's page: what it says, its subtasks and what happened, beside its properties. */
export function TaskPage(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** The task. */
    readonly task: Task;
    /** The view the person came from. */
    readonly view: string;
    /** Go back to the view. */
    readonly onBack: () => void;
    /** Open another task. */
    readonly onOpen: (task: string) => void;
}): JSX.Element {
    // follow the task and those under it
    const { studio } = properties;
    const task = () => properties.task;
    const subtasks = () => studio.tasks().filter((each) => each.parent === task().id);
    const threads = () => task().events.filter(({ answers }) => answers === undefined);

    return (
        <div
            ref={provide(() => Place.current(referenceOf(task().id)))}
            {...style.attrs(text.sm, styles.screen)}
        >
            {/* Where the person is */}
            <header {...style.attrs(styles.bar)}>
                <SidebarTrigger />
                <Breadcrumb>
                    <BreadcrumbList>
                        <BreadcrumbItem>
                            <BreadcrumbLink
                                href="#tasks"
                                onClick={(event) => {
                                    event.preventDefault();
                                    properties.onBack();
                                }}
                            >
                                {properties.view}
                            </BreadcrumbLink>
                        </BreadcrumbItem>
                        <BreadcrumbSeparator />
                        <BreadcrumbItem>
                            <BreadcrumbPage>{task().id}</BreadcrumbPage>
                        </BreadcrumbItem>
                    </BreadcrumbList>
                </Breadcrumb>
            </header>

            <div {...style.attrs(styles.page)}>
                <Stack space="5">
                    {/* What it says */}
                    <Stack space="3">
                        <h1 {...style.attrs(text["2xl"], styles.title)}>
                            <Editable
                                required
                                value={task().title}
                                onValueChange={(title) => studio.update([task().id], { title })}
                            />
                        </h1>
                        <Editable
                            multiline
                            value={task().text}
                            placeholder="Add a description…"
                            onValueChange={(written) =>
                                studio.update([task().id], { text: written })
                            }
                        />
                    </Stack>

                    {/* The tasks under it */}
                    <Subtasks studio={studio} tasks={subtasks()} onOpen={properties.onOpen} />

                    {/* What happened */}
                    <Stack space="3">
                        <h2 {...style.attrs(text.sm, styles.heading)}>Activity</h2>
                        <Activity aria-label="Activity">
                            <For each={threads()} keyed={(event) => event.id}>
                                {(event) => (
                                    <Show
                                        when={event().text !== undefined}
                                        fallback={<TaskChange event={event()} />}
                                    >
                                        <ActivityItem>
                                            <ActivityBody>
                                                <CommentCard
                                                    studio={studio}
                                                    task={task().id}
                                                    comment={event()}
                                                    answers={task().events.filter(
                                                        ({ answers }) => answers === event().id,
                                                    )}
                                                />
                                            </ActivityBody>
                                        </ActivityItem>
                                    </Show>
                                )}
                            </For>
                        </Activity>
                        <CommentBox
                            placeholder="Leave a comment…"
                            onSubmit={(words) => studio.comment(task().id, words)}
                        />
                    </Stack>
                </Stack>

                {/* Its properties */}
                <DataList>
                    <DataListItem>
                        <DataListLabel>Status</DataListLabel>
                        <DataListValue>
                            <ActionSelect action={setStatus} xstyle={styles.value} />
                        </DataListValue>
                    </DataListItem>
                    <DataListItem>
                        <DataListLabel>Priority</DataListLabel>
                        <DataListValue>
                            <ActionSelect action={setPriority} xstyle={styles.value} />
                        </DataListValue>
                    </DataListItem>
                    <DataListItem>
                        <DataListLabel>Assignee</DataListLabel>
                        <DataListValue>
                            <ActionSelect action={assign} xstyle={styles.value} />
                        </DataListValue>
                    </DataListItem>
                    <DataListItem>
                        <DataListLabel>Labels</DataListLabel>
                        <DataListValue>
                            <ActionSelect
                                action={toggleLabel}
                                placeholder="Add label"
                                xstyle={styles.value}
                            >
                                {(labels) => (
                                    <For each={labels}>
                                        {(label) => (
                                            <Badge
                                                {...(label.color === undefined
                                                    ? {}
                                                    : { color: label.color })}
                                            >
                                                {label.title}
                                            </Badge>
                                        )}
                                    </For>
                                )}
                            </ActionSelect>
                        </DataListValue>
                    </DataListItem>
                    <DataListItem>
                        <DataListLabel>Project</DataListLabel>
                        <DataListValue>
                            <ActionSelect
                                action={setProject}
                                placeholder="Add to project"
                                xstyle={styles.value}
                            />
                        </DataListValue>
                    </DataListItem>
                    <DataListItem>
                        <DataListLabel>Due</DataListLabel>
                        <DataListValue>
                            <ActionSelect action={setDue} xstyle={styles.value}>
                                {(days) => (
                                    <Show when={task().due} fallback={days[0]?.title}>
                                        {(due) => <Due day={due()} isLate={isLate(task())} />}
                                    </Show>
                                )}
                            </ActionSelect>
                        </DataListValue>
                    </DataListItem>
                </DataList>
            </div>
        </div>
    );
}

/** Render the tasks under a task, and a field that adds one. */
function Subtasks(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** The tasks. */
    readonly tasks: readonly Task[];
    /** Open a task. */
    readonly onOpen: (task: string) => void;
}): JSX.Element {
    return (
        <Show when={properties.tasks.length > 0}>
            <Stack space="2">
                <h2 {...style.attrs(text.sm, styles.heading)}>Subtasks</h2>
                <ItemGroup>
                    <For each={properties.tasks} keyed={(task) => task.id}>
                        {(task) => (
                            <Item
                                size="sm"
                                render={(attributes) => (
                                    <button
                                        {...attributes}
                                        type="button"
                                        onClick={() => properties.onOpen(task().id)}
                                    />
                                )}
                            >
                                <ChoiceIcon choice={choiceOf(STATUSES, task().status)} />
                                <ItemContent>
                                    <ItemTitle>{task().title}</ItemTitle>
                                </ItemContent>
                                <span {...style.attrs(styles.muted)}>{task().id}</span>
                            </Item>
                        )}
                    </For>
                </ItemGroup>
            </Stack>
        </Show>
    );
}

/** Render a change people made to a task as a line. */
function TaskChange(properties: {
    /** What happened. */
    readonly event: TaskEvent;
}): JSX.Element {
    const person = () => personOf(properties.event.person);

    return (
        <ActivityItem condensed>
            <ActivityBadge>
                <Show when={person()}>{(each) => <Face person={each()} />}</Show>
            </ActivityBadge>
            <ActivityBody>
                <span>
                    {person()?.name ?? "Somebody"} {changeOf(properties.event)} ·{" "}
                    <RelativeTime value={properties.event.at} />
                </span>
            </ActivityBody>
        </ActivityItem>
    );
}

/** Say what a change did, after who did it. */
function changeOf({ change }: TaskEvent): string {
    // read the value as people read it
    const { property, value } = change ?? { property: "created", value: null };
    const choices = { status: STATUSES, priority: PRIORITIES, project: PROJECTS };
    const written = typeof value === "string" ? value : "another value";
    const listed = Object.entries(choices).find(([name]) => name === property)?.[1];
    const title =
        property === "assignee"
            ? personOf(written)?.name
            : (listed?.find((choice) => choice.value === value)?.title ?? written);

    // tell a creation, a value taken off and one set
    if (property === "created") {
        return "created the task";
    } else if (value === null) {
        return `removed the ${property}`;
    }

    return `set the ${property} to ${title ?? written}`;
}

/** Render the dialog that creates a task, its properties set like any task's. */
export function CreateTask(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** Whether it is open. */
    readonly isOpen: boolean;
    /** What the task starts with, such as the group's it is created in. */
    readonly start: TaskPatch;
    /** Open or close it. */
    readonly onOpenChange: (isOpen: boolean) => void;
}): JSX.Element {
    // write a task of its own, which joins the studio once created
    const [isMore, setMore] = createSignal(false);
    const [written, setWritten] = createSignal(0);
    const draft = createMemo(() => {
        written();

        return new Studio([taskOf("HAR-0", { ...properties.start, title: "" }, "ada", 0)], () => 0);
    });
    const task = () => draft().tasks()[0];
    const create = () => {
        // add it, then write another or close
        const own = task();
        if (own === undefined || own.title.trim() === "") {
            return;
        }
        const { title, text: says, status, priority, assignee, labels, project, due } = own;
        properties.studio.create({
            title: title.trim(),
            text: says,
            status,
            priority,
            assignee,
            labels,
            project,
            due,
        });
        setWritten((count) => count + 1);
        if (!isMore()) {
            properties.onOpenChange(false);
        }
    };

    return (
        <Dialog open={properties.isOpen} onOpenChange={(isOpen) => properties.onOpenChange(isOpen)}>
            <DialogContent>
                <DialogTitle>New task</DialogTitle>
                <form
                    onSubmit={(event) => {
                        event.preventDefault();
                        create();
                    }}
                    onKeyDown={(event) => {
                        // create it from anywhere in the form
                        if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                            event.preventDefault();
                            create();
                        }
                    }}
                >
                    <Show when={draft()} keyed>
                        {(current) => (
                            <div
                                ref={[
                                    ...handleTasks(current),
                                    provide(() =>
                                        Place.selected(
                                            current.tasks().map(({ id }) => referenceOf(id)),
                                        ),
                                    ),
                                ]}
                            >
                                <Stack space="3">
                                    <Input
                                        aria-label="Title"
                                        placeholder="Task title"
                                        variant="plain"
                                        xstyle={[text.lg, styles.name]}
                                        value={task()?.title ?? ""}
                                        onInput={(event) =>
                                            current.update(
                                                current.tasks().map(({ id }) => id),
                                                { title: event.currentTarget.value },
                                            )
                                        }
                                    />
                                    <Textarea
                                        aria-label="Description"
                                        placeholder="Add a description…"
                                        variant="plain"
                                        value={task()?.text ?? ""}
                                        onInput={(event) =>
                                            current.update(
                                                current.tasks().map(({ id }) => id),
                                                { text: event.currentTarget.value },
                                            )
                                        }
                                    />
                                    <div {...style.attrs(styles.properties)}>
                                        <ActionSelect action={setStatus} variant="outline" />
                                        <ActionSelect action={setPriority} variant="outline" />
                                        <ActionSelect
                                            action={assign}
                                            variant="outline"
                                            placeholder="Assignee"
                                        />
                                        <ActionSelect
                                            action={toggleLabel}
                                            variant="outline"
                                            placeholder="Labels"
                                        />
                                        <ActionSelect
                                            action={setProject}
                                            variant="outline"
                                            placeholder="Project"
                                        />
                                        <ActionSelect
                                            action={setDue}
                                            variant="outline"
                                            placeholder="Due date"
                                        />
                                    </div>
                                </Stack>
                            </div>
                        )}
                    </Show>
                    <DialogFooter>
                        <Label>
                            <Switch checked={isMore()} onCheckedChange={setMore} /> Create more
                        </Label>
                        <span {...style.attrs(styles.grow)} />
                        <DialogClose
                            render={(attributes) => <Button {...attributes} variant="outline" />}
                        >
                            Cancel
                        </DialogClose>
                        <Button type="submit" disabled={(task()?.title ?? "").trim() === ""}>
                            <Icon icon={plusIcon} /> Create task
                        </Button>
                    </DialogFooter>
                </form>
                <ActionDialog />
            </DialogContent>
        </Dialog>
    );
}
