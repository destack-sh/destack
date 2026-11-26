import { t } from "@destack/locale";
import fileIcon from "@destack/icon/phosphor/file";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import * as style from "@destack/style";
import { width } from "@destack/theme/tokens.stylex";
import { createSignal, For, type JSX, omit } from "@destack/view";
import { Icon } from "../icon/index.ts";
import { Overflow, OverflowMenu } from "../overflow/index.ts";
import { Sortable, SortableContainer, SortableItem, SortableMove } from "../sortable/index.ts";
import {
    Tabs,
    TabsClose,
    TabsContent,
    TabsIndicator,
    TabsItem,
    TabsList,
    type TabsListProperties,
    type TabsProperties,
    TabsTitle,
    TabsTrigger,
} from "./tabs.tsx";

/** The room the narrow examples give their tabs. */
const styles = style.create({
    /** A row's width, narrower than a note's tabs. */
    row: { inlineSize: width.row },
    /** A hover card's width, narrower than the notes' names. */
    card: { inlineSize: width.hoverCard },
});

/** Notes with names longer than a tab has room for. */
const NOTES = [
    { value: "fair", title: "Everything we still need to print before the book fair in Lisbon" },
    { value: "quotes", title: "The two printer quotes for the flyer, side by side" },
    { value: "labels", title: "Shelf labels for the shop, in Portuguese and Arabic" },
] as const;

/** The months of a year. */
const MONTHS = Array.from({ length: 12 }, (_, month) =>
    new Date(Date.UTC(2026, month, 1)).toLocaleString("en-US", { month: "long", timeZone: "UTC" }),
);

/** A note's tabs as their defaults show them. */
export const tabsDefault = defineExample({
    of: Tabs,
    name: "default",
    description: t`A note's edit, preview and history tabs.`,
    properties: { defaultValue: "edit" },
    render: (properties) => <NoteTabs {...properties} />,
});

/** A note's tabs in a narrow row, laid out by their list's variant and sizing. */
export const tabsListDefault = defineExample({
    of: TabsList,
    name: "default",
    description: t`A note's tabs in a narrow row, laid out by their list's variant and sizing.`,
    render: (properties) => <NoteTabs defaultValue="edit" xstyle={styles.row} list={properties} />,
});

/** A note's tabs that select on Enter, Space or a click. */
export const tabsManual = defineExample({
    of: Tabs,
    name: "manual",
    description: t`A note's tabs that select on Enter, Space or a click, not as the focus moves.`,
    properties: { defaultValue: "edit", activationMode: "manual" },
    render: (properties) => <NoteTabs {...properties} />,
});

/** A note's tabs, its history tab unavailable. */
export const tabsTriggerDisabled = defineExample({
    of: TabsTrigger,
    name: "disabled",
    description: t`A note's tabs, its history tab unavailable.`,
    properties: { disabled: true },
    render: (properties) => <NoteTabs defaultValue="edit" history={properties} />,
});

/** A note's tabs with a mark sliding to the selected one. */
export const tabsIndicator = defineExample({
    of: Tabs,
    name: "indicator",
    description: t`A note's tabs, a mark sliding to the selected one.`,
    properties: { defaultValue: "edit" },
    render: (properties) => <NoteTabs {...properties} hasIndicator />,
});

/** Render a note's edit, preview and history tabs, varying the tabs, the list or a tab. */
function NoteTabs(
    properties: TabsProperties & {
        /** The properties of the list. */
        readonly list?: Pick<TabsListProperties, "variant" | "sizing">;
        /** The properties of the history tab. */
        readonly history?: PropertiesOf<typeof TabsTrigger>;
        /** Whether a mark slides to the selected tab. */
        readonly hasIndicator?: boolean;
    },
): JSX.Element {
    return (
        <Tabs {...omit(properties, "list", "history", "hasIndicator")}>
            <TabsList aria-label="Note" {...properties.list}>
                <TabsTrigger value="edit">
                    <TabsTitle>Edit</TabsTitle>
                </TabsTrigger>
                <TabsTrigger value="preview">
                    <TabsTitle>Preview</TabsTitle>
                </TabsTrigger>
                <TabsTrigger value="history" {...properties.history}>
                    <TabsTitle>History</TabsTitle>
                </TabsTrigger>
                {properties.hasIndicator === true ? <TabsIndicator /> : undefined}
            </TabsList>
            <TabsContent value="edit">Editor</TabsContent>
            <TabsContent value="preview">Rendered</TabsContent>
            <TabsContent value="history">Versions</TabsContent>
        </Tabs>
    );
}

/** The files an editor has open, which close, reorder and end in a menu. */
export const tabsFiles = defineExample({
    of: Tabs,
    name: "files",
    description: t`The files an editor has open, which close, reorder and end in a menu.`,
    render: (properties) => <Files {...properties} />,
});

/** Render the open files. */
function Files(properties: TabsProperties): JSX.Element {
    // keep the files in their order
    const [files, setFiles] = createSignal<readonly string[]>([
        "plan.md",
        "timeline.tsx",
        "overflow.tsx",
        "tabs.tsx",
        "README.md",
        "package.json",
    ]);

    return (
        <Tabs
            defaultValue="timeline.tsx"
            onClose={(closed) => setFiles((all) => all.filter((file) => file !== closed))}
            {...properties}
        >
            <Sortable
                indicator="gap"
                onMove={(move) =>
                    setFiles(
                        (all) =>
                            SortableMove.apply({ files: all }, move, (file) => file)["files"] ??
                            all,
                    )
                }
            >
                <Overflow>
                    <SortableContainer
                        id="files"
                        orientation="horizontal"
                        render={(part) => <TabsList aria-label="Files" {...part} />}
                    >
                        <For each={files()}>
                            {(file) => (
                                <SortableItem
                                    id={file}
                                    activator="item"
                                    sensors={["pointer"]}
                                    render={(part) => <TabsItem value={file} {...part} />}
                                >
                                    <TabsTrigger>
                                        <Icon icon={fileIcon} />
                                        <TabsTitle>{file}</TabsTitle>
                                    </TabsTrigger>
                                    {file === "plan.md" ? undefined : <TabsClose />}
                                </SortableItem>
                            )}
                        </For>
                    </SortableContainer>
                    <OverflowMenu />
                </Overflow>
            </Sortable>
            <For each={files()}>{(file) => <TabsContent value={file}>{file}</TabsContent>}</For>
        </Tabs>
    );
}

/** More tabs than fit, which scroll. */
export const tabsOverflowing = defineExample({
    of: Tabs,
    name: "overflowing",
    description: t`More tabs than fit, which scroll with the sides that have more faded.`,
    properties: { defaultValue: "March" },
    render: (properties) => (
        <Tabs {...properties}>
            <TabsList aria-label="Month">
                <For each={MONTHS}>
                    {(month) => <TabsTrigger value={month}>{month}</TabsTrigger>}
                </For>
            </TabsList>
            <For each={MONTHS}>{(month) => <TabsContent value={month}>{month}</TabsContent>}</For>
        </Tabs>
    ),
});

/** Tabs with names longer than they have room for. */
export const tabsLong = defineExample({
    of: Tabs,
    name: "long",
    description: t`Notes as tabs whose names are longer than their room, which fade at their end.`,
    properties: { defaultValue: "fair" },
    render: (properties) => (
        <Tabs {...properties} xstyle={styles.card}>
            <TabsList aria-label="Notes" sizing="shrink">
                <For each={NOTES}>
                    {(note) => (
                        <TabsTrigger value={note.value}>
                            <TabsTitle>{note.title}</TabsTitle>
                        </TabsTrigger>
                    )}
                </For>
            </TabsList>
            <For each={NOTES}>
                {(note) => <TabsContent value={note.value}>{note.title}</TabsContent>}
            </For>
        </Tabs>
    ),
});
