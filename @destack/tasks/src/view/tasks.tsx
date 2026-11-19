import { argument, t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { Button } from "@destack/ui/button";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import { Input } from "@destack/ui/input";
import { createSignal, For, Show, useLocale, useQuery, useSpace } from "@destack/view";
import { project, task } from "../object/index.ts";
import { parseAsString, useQueryState } from "@destack/view/router";
import { schema } from "@destack/schema";
import { BOARD_STATES, NEXT_STATE, STATE_TITLES, type TaskState } from "./state.ts";

/** The list's layout: a header with the quick create, then one group of rows per state. */
const styles = style.create({
    page: {
        display: "grid",
        gap: space[5],
        marginInline: "auto",
        maxInlineSize: "56rem",
        paddingBlock: space[6],
        paddingInline: space[6],
    },
    header: {
        alignItems: "baseline",
        display: "flex",
        gap: space[3],
    },
    count: {
        color: color.mutedForeground,
    },
    create: {
        display: "flex",
        gap: space[2],
    },
    group: {
        display: "grid",
    },
    groupTitle: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: radius[2],
        color: color.mutedForeground,
        display: "flex",
        gap: space[2],
        paddingBlock: space[1],
        paddingInline: space[3],
    },
    row: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: space[3],
        minBlockSize: "2.5rem",
        paddingInline: space[3],
    },
    title: {
        flex: 1,
        minInlineSize: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
});

/** List the space's open tasks live, grouped by state, with a quick create and a search. */
export default function Tasks() {
    // follow the open tasks of the project asked for that match the search
    const locale = useLocale();
    const store = useSpace({ task, project });
    const [search, setSearch] = createSignal("");
    const [projectParameter] = useQueryState("project", parseAsString);
    const projectId = () => {
        const parameter = projectParameter();

        return parameter === null ? null : schema.identifier("project").parse(parameter);
    };
    const projects = useQuery(() => store.query.project.findMany({}));
    const named = () => projects().find((each) => each.id === projectId());
    const open = () => {
        const id = projectId();

        return {
            state: { notIn: ["canceled"] as const },
            deletedAt: { isNull: true },
            ...(id === null ? {} : { projectId: id }),
        };
    };
    const tasks = useQuery(() =>
        store.query.task.findMany({
            where: search() === "" ? open() : { AND: [open(), { SEARCH: search() }] },
            orderBy: { createdAt: "desc" },
        }),
    );

    // create a task from the title typed in, clearing the input
    const [title, setTitle] = createSignal("");
    const create = () => {
        if (title().trim() === "") {
            return;
        }
        const id = projectId();
        store.mutate.task.create({
            title: title().trim(),
            state: "todo",
            ...(id === null ? {} : { projectId: id }),
        });
        setTitle("");
    };

    return (
        <main {...style.attrs(styles.page)}>
            {/* Title, count and search */}
            <header {...style.attrs(styles.header)}>
                <h1 {...style.attrs(text["2xl"])}>{named()?.name ?? locale.render(t`Tasks`)}</h1>
                <span {...style.attrs(text.sm, styles.count)}>{tasks().length}</span>
            </header>
            <div {...style.attrs(styles.create)}>
                <Input
                    placeholder={locale.render(t`New task`)}
                    value={title()}
                    onInput={(event) => setTitle(event.currentTarget.value)}
                    onKeyDown={(event) => event.key === "Enter" && create()}
                />
                <Button onClick={() => create()}>{locale.render(t`Create`)}</Button>
                <Input
                    type="search"
                    placeholder={locale.render(t`Search`)}
                    value={search()}
                    onInput={(event) => setSearch(event.currentTarget.value)}
                />
            </div>

            {/* One group per state, in workflow order */}
            <Show when={tasks().length > 0} fallback={<NoTasks />}>
                <For each={BOARD_STATES}>
                    {(state) => (
                        <TaskGroup
                            state={state}
                            tasks={tasks().filter((each) => each.state === state)}
                            onAdvance={(id, next) => store.mutate.task.update({ id, state: next })}
                        />
                    )}
                </For>
            </Show>
        </main>
    );
}

/** Show one state's tasks under its title, each advancing to the next state. */
function TaskGroup(properties: {
    /** The state the group shows. */
    state: TaskState;
    /** The group's tasks. */
    tasks: readonly { readonly id: string; readonly title: string; readonly state: TaskState }[];
    /** Move a task to its next state. */
    onAdvance: (id: string, next: TaskState) => void;
}) {
    const locale = useLocale();

    return (
        <Show when={properties.tasks.length > 0}>
            <section {...style.attrs(styles.group)}>
                {/* State title with its count */}
                <h2 {...style.attrs(text.sm, styles.groupTitle)}>
                    {locale.render(STATE_TITLES[properties.state])}
                    <span>{properties.tasks.length}</span>
                </h2>

                {/* Rows */}
                <For each={properties.tasks}>
                    {(each) => (
                        <div {...style.attrs(text.sm, styles.row)}>
                            <span {...style.attrs(styles.title)}>{each.title}</span>
                            <Show when={NEXT_STATE[each.state]}>
                                {(next) => (
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        onClick={() => properties.onAdvance(each.id, next())}
                                    >
                                        {locale.render(t`Move to ${argument("state")}`, {
                                            state: locale.render(STATE_TITLES[next()]),
                                        })}
                                    </Button>
                                )}
                            </Show>
                        </div>
                    )}
                </For>
            </section>
        </Show>
    );
}

/** Explain where tasks appear before the first one does. */
function NoTasks() {
    const locale = useLocale();

    return (
        <Empty>
            <EmptyHeader>
                <EmptyTitle>{locale.render(t`No tasks yet`)}</EmptyTitle>
                <EmptyDescription>
                    {locale.render(t`Tasks you create in this space appear here, live.`)}
                </EmptyDescription>
            </EmptyHeader>
        </Empty>
    );
}
