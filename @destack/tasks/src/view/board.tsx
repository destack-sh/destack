import { argument, t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import { Button } from "@destack/ui/button";
import { Item, ItemActions, ItemContent, ItemGroup, ItemTitle } from "@destack/ui/item";
import { For, Show, useLocale, useQuery, useSpace } from "@destack/view";
import { task } from "../object/index.ts";
import { BOARD_STATES, type TaskState, NEXT_STATE, STATE_TITLES } from "./state.ts";

/** The board's layout: a title, then one column per state on a muted surface. */
const styles = style.create({
    page: {
        display: "grid",
        gap: space[5],
        paddingBlock: space[6],
        paddingInline: space[6],
    },
    columns: {
        alignItems: "start",
        display: "grid",
        gap: space[3],
        gridTemplateColumns: "repeat(auto-fit, minmax(14rem, 1fr))",
    },
    column: {
        backgroundColor: color.muted,
        borderRadius: radius[3],
        display: "grid",
        gap: space[2],
        padding: space[2],
    },
    columnTitle: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        gap: space[2],
        paddingInline: space[1],
    },
    card: {
        backgroundColor: color.background,
    },
});

/** Show the space's tasks as a board, one column per state in its own order. */
export default function Board() {
    const locale = useLocale();

    return (
        <main {...style.attrs(styles.page)}>
            {/* Title */}
            <h1 {...style.attrs(text["2xl"])}>{locale.render(t`Board`)}</h1>

            {/* Columns */}
            <div {...style.attrs(styles.columns)}>
                <For each={BOARD_STATES}>{(state) => <Column state={state} />}</For>
            </div>
        </main>
    );
}

/** Show one state's tasks by position, each moving on to the next state. */
function Column(properties: {
    /** The state the column shows. */
    state: TaskState;
}) {
    // follow the state's tasks in their board order
    const locale = useLocale();
    const store = useSpace({ task });
    const tasks = useQuery(() =>
        store.query.task.findMany({
            where: { state: properties.state, deletedAt: { isNull: true } },
            orderBy: { position: "asc" },
        }),
    );

    return (
        <section {...style.attrs(styles.column)}>
            {/* Column title and count */}
            <h2 {...style.attrs(text.sm, styles.columnTitle)}>
                {locale.render(STATE_TITLES[properties.state])}
                <span>{tasks().length}</span>
            </h2>

            {/* Cards */}
            <ItemGroup>
                <For each={tasks()}>
                    {(each) => (
                        <Item variant="outline" size="sm" xstyle={styles.card}>
                            <ItemContent>
                                <ItemTitle>{each.title}</ItemTitle>
                            </ItemContent>
                            <Show when={NEXT_STATE[each.state]}>
                                {(next) => (
                                    <ItemActions>
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={() =>
                                                store.mutate.task.update({
                                                    id: each.id,
                                                    state: next(),
                                                })
                                            }
                                        >
                                            {locale.render(t`Move to ${argument("state")}`, {
                                                state: locale.render(STATE_TITLES[next()]),
                                            })}
                                        </Button>
                                    </ItemActions>
                                )}
                            </Show>
                        </Item>
                    )}
                </For>
            </ItemGroup>
        </section>
    );
}
