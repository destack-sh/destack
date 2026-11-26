import arrowLeftIcon from "@destack/icon/phosphor/arrow-left";
import arrowRightIcon from "@destack/icon/phosphor/arrow-right";
import circleIcon from "@destack/icon/phosphor/circle";
import fileIcon from "@destack/icon/phosphor/file";
import fileTsxIcon from "@destack/icon/phosphor/file-tsx";
import fileTextIcon from "@destack/icon/phosphor/file-text";
import houseIcon from "@destack/icon/phosphor/house";
import kanbanIcon from "@destack/icon/phosphor/kanban";
import plusIcon from "@destack/icon/phosphor/plus";
import type { IconBodies } from "@destack/icon";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createSignal, For, type JSX, Show, untrack, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuTrigger,
} from "../context-menu/index.ts";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "../empty/index.ts";
import { Icon } from "../icon/index.ts";
import { Overflow, OverflowMenu } from "../overflow/index.ts";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "../resizable/index.ts";
import { Sortable, SortableContainer, SortableItem, SortableMove } from "../sortable/index.ts";
import {
    Tabs,
    TabsClose,
    TabsContent,
    TabsItem,
    TabsList,
    tabsListVariants,
    TabsTitle,
    TabsTrigger,
} from "../tabs/index.ts";

/** The styles of an editor's and a workspace's tabs. */
const styles = style.create({
    window: {
        width: "100%",
        height: "24rem",
        overflow: "hidden",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
    },
    group: {
        height: "100%",
        gap: 0,
    },
    // take the width of its icon
    pinned: {
        flexShrink: 0,
        minWidth: "auto",
    },
    preview: {
        fontStyle: "italic",
    },
    panel: {
        padding: space[4],
    },
    code: {
        margin: 0,
        color: color.mutedForeground,
    },
    page: {
        padding: space[6],
    },
    title: {
        margin: 0,
    },
    // fill the panel beside the pages
    side: {
        display: "flex",
        flexDirection: "column",
        height: "100%",
        padding: space[2],
        gap: space[1],
        backgroundColor: color.muted,
    },
    column: {
        alignSelf: "stretch",
        flexShrink: 1,
        width: "100%",
        minHeight: 0,
    },
    // stand right beside the panels
    close: {
        gap: 0,
    },
});

/** A file an editor has open. */
interface File {
    /** The path, which its tab stands for. */
    readonly path: string;
    /** Whether it has changes that are not saved. */
    readonly isUnsaved?: boolean;
    /** Whether it stays open at the start of its group. */
    readonly isPinned?: boolean;
    /** Whether the next opened file takes its place. */
    readonly isPreview?: boolean;
}

/** The files of an editor's groups. */
type Groups = Readonly<Record<string, readonly File[]>>;

/** The files open at first. */
const FILES: Groups = {
    left: [
        { path: "README.md", isPinned: true },
        { path: "timeline.tsx" },
        { path: "overflow.tsx", isUnsaved: true },
        { path: "control.ts" },
        { path: "tabs.test.tsx", isPreview: true },
    ],
    right: [{ path: "tabs.tsx", isUnsaved: true }, { path: "package.json" }],
};

/** Files in two groups, which tabs move between. */
export const tabsEditor = defineExample({
    of: Tabs,
    name: "editor",
    description: "an editor's files in two groups, unsaved, pinned and in preview",
    render: () => <Editor />,
});

/** Render an editor's groups side by side. */
function Editor(): JSX.Element {
    // move files within a group and to the other
    const [groups, setGroups] = createSignal<Groups>(FILES);
    const [selected, setSelected] = createSignal<Readonly<Record<string, string>>>({
        left: "overflow.tsx",
        right: "tabs.tsx",
    });

    // change a group's files, and which is selected
    const select = (group: string, path: string): void => {
        setSelected((all) => ({ ...all, [group]: path }));
    };
    const change = (group: string, files: (all: readonly File[]) => readonly File[]): void => {
        setGroups((all) => ({ ...all, [group]: files(all[group] ?? []) }));
    };

    return (
        <Sortable
            indicator="gap"
            onMove={(move) => {
                // select the moved file where it lands
                setGroups((all) => SortableMove.apply(all, move, (file) => file.path));
                setSelected((all) => ({ ...all, [move.container]: move.id }));
            }}
        >
            <ResizablePanelGroup xstyle={styles.window}>
                <ResizablePanel defaultSize={60} minSize={20}>
                    <Group
                        id="left"
                        files={groups()["left"] ?? []}
                        selected={selected()["left"] ?? ""}
                        onSelect={(path) => select("left", path)}
                        onChange={(files) => change("left", files)}
                    />
                </ResizablePanel>
                <ResizableHandle />
                <ResizablePanel minSize={20}>
                    <Group
                        id="right"
                        files={groups()["right"] ?? []}
                        selected={selected()["right"] ?? ""}
                        onSelect={(path) => select("right", path)}
                        onChange={(files) => change("right", files)}
                    />
                </ResizablePanel>
            </ResizablePanelGroup>
        </Sortable>
    );
}

/** Render one group of files. */
function Group(properties: {
    /** The id of the group. */
    readonly id: string;
    /** The files it has open. */
    readonly files: readonly File[];
    /** The path of the selected file. */
    readonly selected: string;
    /** Handle another file being selected. */
    readonly onSelect: (path: string) => void;
    /** Handle its files changing. */
    readonly onChange: (change: (files: readonly File[]) => readonly File[]) => void;
}): JSX.Element {
    // open a new file after the others
    let count = 0;
    const open = (): void => {
        // number the files opened
        count += 1;
        const path = `untitled-${properties.id}-${String(count)}`;
        properties.onChange((all) => [...all, { path, isUnsaved: true }]);
        properties.onSelect(path);
    };

    return (
        <Tabs
            value={properties.selected}
            onValueChange={(path) => properties.onSelect(path)}
            onClose={(closed) =>
                properties.onChange((all) => all.filter((file) => file.path !== closed))
            }
            xstyle={styles.group}
        >
            <Overflow xstyle={tabsListVariants({ variant: "card" })}>
                <SortableContainer
                    id={properties.id}
                    orientation="horizontal"
                    render={(part) => (
                        <TabsList {...part} aria-label="Files" variant="card" sizing="shrink" />
                    )}
                >
                    <For each={properties.files} keyed={(file) => file.path}>
                        {(file) => <FileTab file={file()} onChange={properties.onChange} />}
                    </For>
                </SortableContainer>
                <OverflowMenu />
                <Button variant="ghost" size="icon" aria-label="New file" onClick={open}>
                    <Icon icon={plusIcon} />
                </Button>
            </Overflow>
            <For each={properties.files} keyed={(file) => file.path}>
                {(file) => (
                    <TabsContent value={file().path} xstyle={styles.panel}>
                        <pre {...style.attrs(text.sm, styles.code)}>{`// ${file().path}`}</pre>
                    </TabsContent>
                )}
            </For>
            <Show when={properties.files.length === 0}>
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>No file open</EmptyTitle>
                        <EmptyDescription>Drag a tab here, or open a new file.</EmptyDescription>
                    </EmptyHeader>
                </Empty>
            </Show>
        </Tabs>
    );
}

/** Render the tab of a file, with a menu of its own. */
function FileTab(properties: {
    /** The file. */
    readonly file: File;
    /** Handle the files of its group changing. */
    readonly onChange: (change: (files: readonly File[]) => readonly File[]) => void;
}): JSX.Element {
    // change the file where it stands
    const path = untrack(() => properties.file.path);
    const isPinned = (): boolean => properties.file.isPinned === true;
    const update = (changes: Partial<File>): void =>
        properties.onChange((all) =>
            all.map((file) => (file.path === path ? { ...file, ...changes } : file)),
        );

    // stand first while it is pinned
    const pin = (): void =>
        properties.onChange((all) => {
            const next = all.map((file) =>
                file.path === path ? { ...file, isPinned: file.isPinned !== true } : file,
            );

            return next.toSorted(
                (one, other) => Number(other.isPinned === true) - Number(one.isPinned === true),
            );
        });

    return (
        <ContextMenu>
            <SortableItem
                id={path}
                activator="item"
                sensors={["pointer"]}
                render={(part) => (
                    <ContextMenuTrigger
                        {...part}
                        render={(inner) => (
                            <TabsItem
                                {...inner}
                                value={path}
                                xstyle={isPinned() && styles.pinned}
                            />
                        )}
                    />
                )}
            >
                <TabsTrigger
                    aria-label={isPinned() ? path : undefined}
                    onDblClick={() => update({ isPreview: false })}
                    xstyle={properties.file.isPreview === true && styles.preview}
                >
                    <Icon icon={iconOf(path)} />
                    <Show when={!isPinned()}>
                        <TabsTitle>{path}</TabsTitle>
                    </Show>
                </TabsTrigger>
                <Show when={!isPinned()}>
                    <Show when={properties.file.isUnsaved === true} fallback={<TabsClose />}>
                        <TabsClose>
                            <Icon icon={circleIcon} weight="fill" size="0.6em" />
                        </TabsClose>
                    </Show>
                </Show>
            </SortableItem>
            <ContextMenuContent>
                <ContextMenuItem onSelect={pin}>{isPinned() ? "Unpin" : "Pin"}</ContextMenuItem>
                <ContextMenuItem onSelect={() => update({ isUnsaved: false })}>
                    Save
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem
                    onSelect={() =>
                        properties.onChange((all) => all.filter((file) => file.path !== path))
                    }
                >
                    Close
                </ContextMenuItem>
                <ContextMenuItem
                    onSelect={() =>
                        properties.onChange((all) =>
                            all.filter((file) => file.path === path || file.isPinned === true),
                        )
                    }
                >
                    Close others
                </ContextMenuItem>
            </ContextMenuContent>
        </ContextMenu>
    );
}

/** Read the icon of a file by its extension. */
function iconOf(path: string): IconBodies {
    if (path.endsWith(".tsx")) {
        return fileTsxIcon;
    } else if (path.endsWith(".md")) {
        return fileTextIcon;
    }

    return fileIcon;
}

/** A page a workspace has open. */
interface Page {
    /** The id, which its tab stands for. */
    readonly id: string;
    /** The title. */
    readonly title: string;
    /** The icon. */
    readonly icon: IconBodies;
}

/** The pages open at first. */
const PAGES: readonly Page[] = [
    { id: "home", title: "Home", icon: houseIcon },
    { id: "roadmap", title: "Roadmap for the spring release", icon: kanbanIcon },
    { id: "notes", title: "Notes from the design review", icon: fileTextIcon },
    { id: "launch", title: "Launch plan", icon: fileTextIcon },
];

/** Pages along the top of a workspace's window. */
export const tabsPages = defineExample({
    of: Tabs,
    name: "pages",
    description: "a workspace's pages along the top of its window, narrowing with the room",
    render: () => <Pages />,
});

/** Render a workspace's pages and the bar of their tabs. */
function Pages(): JSX.Element {
    // keep the pages in order, and go back and forth among them
    const locale = useLocale();
    const [pages, setPages] = createSignal(PAGES);
    const [value, setValue] = createSignal("roadmap");
    const go = (by: number): void => {
        // take the page beside the selected one
        const at = pages().findIndex((page) => page.id === value());
        const next = pages()[at + by];
        if (next !== undefined) {
            setValue(next.id);
        }
    };

    // open a new page after the others
    let count = 0;
    const open = (): void => {
        // number the pages opened
        count += 1;
        const id = `page-${String(count)}`;
        setPages((all) => [...all, { id, title: "New page", icon: fileIcon }]);
        setValue(id);
    };

    // stop at either end
    const isFirst = (): boolean => pages()[0]?.id === value();
    const isLast = (): boolean => pages().at(-1)?.id === value();
    const [back, forth] =
        locale.direction === "rtl"
            ? [arrowRightIcon, arrowLeftIcon]
            : [arrowLeftIcon, arrowRightIcon];

    return (
        <Tabs
            value={value()}
            onValueChange={setValue}
            onClose={(closed) => setPages((all) => all.filter((page) => page.id !== closed))}
            xstyle={[styles.window, styles.close]}
        >
            <Sortable
                indicator="gap"
                onMove={(move) =>
                    setPages(
                        (all) =>
                            SortableMove.apply({ pages: all }, move, (page) => page.id)["pages"] ??
                            all,
                    )
                }
            >
                <Overflow xstyle={tabsListVariants({ variant: "card" })}>
                    <Button
                        variant="ghost"
                        size="icon"
                        aria-label="Back"
                        disabled={isFirst()}
                        onClick={() => go(-1)}
                    >
                        <Icon icon={back} />
                    </Button>
                    <Button
                        variant="ghost"
                        size="icon"
                        aria-label="Forward"
                        disabled={isLast()}
                        onClick={() => go(1)}
                    >
                        <Icon icon={forth} />
                    </Button>
                    <SortableContainer
                        id="pages"
                        orientation="horizontal"
                        render={(part) => (
                            <TabsList {...part} aria-label="Pages" variant="card" sizing="fixed" />
                        )}
                    >
                        <For each={pages()} keyed={(page) => page.id}>
                            {(page) => (
                                <SortableItem
                                    id={page().id}
                                    activator="item"
                                    sensors={["pointer"]}
                                    render={(part) => <TabsItem {...part} value={page().id} />}
                                >
                                    <TabsTrigger>
                                        <Icon icon={page().icon} />
                                        <TabsTitle>{page().title}</TabsTitle>
                                    </TabsTrigger>
                                    <Show when={page().id !== "home"}>
                                        <TabsClose />
                                    </Show>
                                </SortableItem>
                            )}
                        </For>
                    </SortableContainer>
                    <OverflowMenu />
                    <Button variant="ghost" size="icon" aria-label="New page" onClick={open}>
                        <Icon icon={plusIcon} />
                    </Button>
                </Overflow>
            </Sortable>
            <For each={pages()} keyed={(page) => page.id}>
                {(page) => (
                    <TabsContent value={page().id} xstyle={styles.page}>
                        <h1 {...style.attrs(text["2xl"], styles.title)}>{page().title}</h1>
                    </TabsContent>
                )}
            </For>
        </Tabs>
    );
}

/** Pages down the side of a browser's window. */
export const tabsSide = defineExample({
    of: Tabs,
    name: "side",
    description: "a browser's pages down the side of its window, which collapses to their icons",
    render: () => <Side />,
});

/** Render a browser's pages and the column of their tabs. */
function Side(): JSX.Element {
    // keep the pages in order, and open new ones first
    const [pages, setPages] = createSignal(PAGES);
    const [value, setValue] = createSignal("roadmap");
    let count = 0;
    const open = (): void => {
        // number the pages opened
        count += 1;
        const id = `page-${String(count)}`;
        setPages((all) => [...all, { id, title: "New page", icon: fileIcon }]);
        setValue(id);
    };

    return (
        <Tabs
            orientation="vertical"
            value={value()}
            onValueChange={setValue}
            onClose={(closed) => setPages((all) => all.filter((page) => page.id !== closed))}
            xstyle={[styles.window, styles.close]}
        >
            <ResizablePanelGroup>
                <ResizablePanel
                    defaultSize={28}
                    minSize="160px"
                    maxSize={50}
                    collapsible
                    collapsedSize="52px"
                >
                    <Sortable
                        indicator="gap"
                        xstyle={styles.side}
                        onMove={(move) =>
                            setPages(
                                (all) =>
                                    SortableMove.apply({ pages: all }, move, (page) => page.id)[
                                        "pages"
                                    ] ?? all,
                            )
                        }
                    >
                        <SortableContainer
                            id="pages"
                            render={(part) => (
                                <TabsList
                                    {...part}
                                    aria-label="Pages"
                                    variant="subtle"
                                    sizing="shrink"
                                    xstyle={styles.column}
                                />
                            )}
                        >
                            <For each={pages()} keyed={(page) => page.id}>
                                {(page) => (
                                    <SortableItem
                                        id={page().id}
                                        activator="item"
                                        sensors={["pointer"]}
                                        render={(part) => <TabsItem {...part} value={page().id} />}
                                    >
                                        <TabsTrigger>
                                            <Icon icon={page().icon} />
                                            <TabsTitle>{page().title}</TabsTitle>
                                        </TabsTrigger>
                                        <Show when={page().id !== "home"}>
                                            <TabsClose />
                                        </Show>
                                    </SortableItem>
                                )}
                            </For>
                        </SortableContainer>
                        <Button variant="ghost" size="icon" aria-label="New page" onClick={open}>
                            <Icon icon={plusIcon} />
                        </Button>
                    </Sortable>
                </ResizablePanel>
                <ResizableHandle />
                <ResizablePanel>
                    <For each={pages()} keyed={(page) => page.id}>
                        {(page) => (
                            <TabsContent value={page().id} xstyle={styles.page}>
                                <h1 {...style.attrs(text["2xl"], styles.title)}>{page().title}</h1>
                            </TabsContent>
                        )}
                    </For>
                </ResizablePanel>
            </ResizablePanelGroup>
        </Tabs>
    );
}
