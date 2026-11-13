import { t } from "@destack/locale";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import { Cluster } from "@destack/ui/cluster";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import { Input } from "@destack/ui/input";
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemTitle,
} from "@destack/ui/item";
import { createSignal, For, Show, useLocale, useQuery, useSpace } from "@destack/view";
import { task } from "../object/index.ts";
import { NEXT_STATE, STATE_TITLES } from "./state.ts";

/** List the space's open tasks live, newest first, with a search and a quick create. */
export default function Tasks() {
    // follow the tasks that are neither done nor canceled, matching the search
    const locale = useLocale();
    const space = useSpace({ task });
    const [search, setSearch] = createSignal("");
    const open = { state: { notIn: ["done", "canceled"] as const } };
    const tasks = useQuery(() =>
        space.query.task.findMany({
            where: search() === "" ? open : { AND: [open, { SEARCH: search() }] },
            orderBy: { createdAt: "desc" },
        }),
    );

    // create a task from the title typed in, clearing the input
    const [title, setTitle] = createSignal("");
    const create = () => {
        if (title().trim() === "") {
            return;
        }
        space.mutate.task.create({ title: title().trim() });
        setTitle("");
    };

    return (
        <div>
            {/* New task and search */}
            <Cluster>
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
            </Cluster>

            {/* Open tasks */}
            <Show when={tasks().length > 0} fallback={<NoTasks />}>
                <ItemGroup>
                    <For each={tasks()}>
                        {(each) => (
                            <Item>
                                <ItemContent>
                                    <ItemTitle>{each.title}</ItemTitle>
                                    <ItemDescription>{each.description}</ItemDescription>
                                </ItemContent>
                                <ItemActions>
                                    <Badge>{locale.render(STATE_TITLES[each.state])}</Badge>
                                    <Show when={NEXT_STATE[each.state]}>
                                        {(next) => (
                                            <Button
                                                onClick={() =>
                                                    space.mutate.task.update({
                                                        id: each.id,
                                                        state: next(),
                                                    })
                                                }
                                            >
                                                {locale.render(STATE_TITLES[next()])}
                                            </Button>
                                        )}
                                    </Show>
                                </ItemActions>
                            </Item>
                        )}
                    </For>
                </ItemGroup>
            </Show>
        </div>
    );
}

/** Explain where tasks appear before the first one does. */
function NoTasks() {
    const locale = useLocale();

    return (
        <Empty>
            <EmptyHeader>
                <EmptyTitle>{locale.render(t`No open tasks`)}</EmptyTitle>
                <EmptyDescription>
                    {locale.render(t`Tasks you create in this space appear here.`)}
                </EmptyDescription>
            </EmptyHeader>
        </Empty>
    );
}
