import { t } from "@destack/locale";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import { Grid } from "@destack/ui/grid";
import { Item, ItemActions, ItemContent, ItemGroup, ItemTitle } from "@destack/ui/item";
import { For, Show, useLocale, useQuery, useSpace } from "@destack/view";
import { task } from "../object/index.ts";
import { BOARD_STATES, type TaskState, NEXT_STATE, STATE_TITLES } from "./state.ts";

/** Show the space's tasks as a board, one column per state in its own order. */
export default function Board() {
    return (
        <Grid>
            <For each={BOARD_STATES}>{(state) => <Column state={state} />}</For>
        </Grid>
    );
}

/** Show one state's tasks by position, each moving on to the next state. */
function Column(properties: {
    /** The state the column shows. */
    state: TaskState;
}) {
    // follow the state's tasks in their board order
    const locale = useLocale();
    const space = useSpace({ task });
    const tasks = useQuery(() =>
        space.query.task.findMany({
            where: { state: properties.state },
            orderBy: { position: "asc" },
        }),
    );

    return (
        <section>
            {/* Column title and count */}
            <h2>
                {locale.render(STATE_TITLES[properties.state])} <Badge>{tasks().length}</Badge>
            </h2>

            {/* Cards */}
            <ItemGroup>
                <For each={tasks()}>
                    {(each) => (
                        <Item>
                            <ItemContent>
                                <ItemTitle>{each.title}</ItemTitle>
                            </ItemContent>
                            <Show when={NEXT_STATE[each.state]}>
                                {(next) => (
                                    <ItemActions>
                                        <Button
                                            onClick={() =>
                                                space.mutate.task.update({
                                                    id: each.id,
                                                    state: next(),
                                                })
                                            }
                                        >
                                            {locale.render(t`Move on`)}
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
