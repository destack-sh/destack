import { argument, t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space } from "@destack/theme/tokens.stylex";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
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
import { project, task } from "../object/index.ts";
import { PROJECT_TITLES } from "./state.ts";

/** The projects page's layout: a header with the quick create, then one row per project. */
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
    link: {
        color: "inherit",
        textDecoration: "none",
    },
});

/** List the space's projects live, each with its progress, with a quick create. */
export default function Projects() {
    // follow the projects outside the trash, and every task, which counts each project's progress
    const locale = useLocale();
    const store = useSpace({ project, task });
    const projects = useQuery(() =>
        store.query.project.findMany({
            where: { deletedAt: { isNull: true } },
            orderBy: { createdAt: "desc" },
        }),
    );
    const tasks = useQuery(() =>
        store.query.task.findMany({
            where: { state: { notIn: ["canceled"] }, deletedAt: { isNull: true } },
        }),
    );
    const progress = (id: string) => {
        const owned = tasks().filter((each) => each.projectId === id);

        return { done: owned.filter((each) => each.state === "done").length, all: owned.length };
    };

    // create a project from the name typed in, clearing the input
    const [name, setName] = createSignal("");
    const create = () => {
        if (name().trim() === "") {
            return;
        }
        store.mutate.project.create({ name: name().trim() });
        setName("");
    };

    return (
        <main {...style.attrs(styles.page)}>
            {/* Title, count and quick create */}
            <header {...style.attrs(styles.header)}>
                <h1 {...style.attrs(text["2xl"])}>{locale.render(t`Projects`)}</h1>
                <span {...style.attrs(text.sm, styles.count)}>{projects().length}</span>
            </header>
            <div {...style.attrs(styles.create)}>
                <Input
                    placeholder={locale.render(t`New project`)}
                    value={name()}
                    onInput={(event) => setName(event.currentTarget.value)}
                    onKeyDown={(event) => event.key === "Enter" && create()}
                />
                <Button onClick={() => create()}>{locale.render(t`Create`)}</Button>
            </div>

            {/* One row per project, opening its tasks */}
            <Show when={projects().length > 0} fallback={<NoProjects />}>
                <ItemGroup>
                    <For each={projects()}>
                        {(each) => (
                            <a href={`tasks?project=${each.id}`} {...style.attrs(styles.link)}>
                                <Item variant="outline" size="sm">
                                    <ItemContent>
                                        <ItemTitle>{each.name}</ItemTitle>
                                        <ItemDescription>
                                            {locale.render(
                                                t`${argument("done")} of ${argument("all")} tasks done`,
                                                progress(each.id),
                                            )}
                                        </ItemDescription>
                                    </ItemContent>
                                    <ItemActions>
                                        <Badge variant="secondary">
                                            {locale.render(PROJECT_TITLES[each.state])}
                                        </Badge>
                                    </ItemActions>
                                </Item>
                            </a>
                        )}
                    </For>
                </ItemGroup>
            </Show>
        </main>
    );
}

/** Explain what projects are before the first one exists. */
function NoProjects() {
    const locale = useLocale();

    return (
        <Empty>
            <EmptyHeader>
                <EmptyTitle>{locale.render(t`No projects yet`)}</EmptyTitle>
                <EmptyDescription>
                    {locale.render(t`Projects group the space's tasks, each with its progress.`)}
                </EmptyDescription>
            </EmptyHeader>
        </Empty>
    );
}
