import fileIcon from "@destack/icon/phosphor/file";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import { createSignal, For, type JSX } from "@destack/view";
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
    TabsTrigger,
} from "./tabs.tsx";

/** The months of a year. */
const MONTHS = Array.from({ length: 12 }, (_, month) =>
    new Date(Date.UTC(2026, month, 1)).toLocaleString("en-US", { month: "long", timeZone: "UTC" }),
);

/** A note's editor and preview as tabs. */
export const tabsEditorPreview = defineExample({
    of: Tabs,
    name: "editor-preview",
    description: "a note's editor and preview as tabs",
    properties: {
        defaultValue: "edit",
    },
    render: (properties) => (
        <Tabs {...properties}>
            <TabsList aria-label="Note">
                <TabsTrigger value="edit">Edit</TabsTrigger>
                <TabsTrigger value="preview">Preview</TabsTrigger>
            </TabsList>
            <TabsContent value="edit">The editor</TabsContent>
            <TabsContent value="preview">The rendered note</TabsContent>
        </Tabs>
    ),
});

/** A space's views as underlined tabs. */
export const tabsUnderlined = defineExample({
    of: Tabs,
    name: "underlined",
    description: "a space's views as underlined tabs",
    properties: {
        defaultValue: "overview",
    },
    render: (properties) => (
        <Tabs {...properties}>
            <TabsList aria-label="Space" variant="line">
                <TabsTrigger value="overview">Overview</TabsTrigger>
                <TabsTrigger value="members">Members</TabsTrigger>
                <TabsTrigger value="settings">Settings</TabsTrigger>
            </TabsList>
            <TabsContent value="overview">What changed this week</TabsContent>
            <TabsContent value="members">Who is here</TabsContent>
            <TabsContent value="settings">How it is set up</TabsContent>
        </Tabs>
    ),
});

/** A space's settings as tabs beside their panels. */
export const tabsVertical = defineExample({
    of: Tabs,
    name: "vertical",
    description: "a space's settings as tabs beside their panels",
    properties: {
        defaultValue: "general",
        orientation: "vertical",
    },
    render: (properties) => (
        <Tabs {...properties}>
            <TabsList aria-label="Settings" variant="line">
                <TabsTrigger value="general">General</TabsTrigger>
                <TabsTrigger value="access">Access</TabsTrigger>
                <TabsTrigger value="billing">Billing</TabsTrigger>
            </TabsList>
            <TabsContent value="general">Name and icon</TabsContent>
            <TabsContent value="access">Who may do what</TabsContent>
            <TabsContent value="billing">Plan and invoices</TabsContent>
        </Tabs>
    ),
});

/** A note's tabs that select as the focus moves, its history tab unavailable. */
export const tabsHistoryUnavailable = defineExample({
    of: Tabs,
    name: "history-unavailable",
    description: "a note's tabs that select as the focus moves, its history tab unavailable",
    properties: { activationMode: "automatic" },
    render: renderHistoryTabs,
});

/** A note's tabs that select on Enter, Space or a click, its history tab unavailable. */
export const tabsManualActivation = defineExample({
    of: Tabs,
    name: "manual-activation",
    description:
        "a note's tabs that select on Enter, Space or a click, its history tab unavailable",
    properties: { activationMode: "manual" },
    render: renderHistoryTabs,
});

/** Render a note's edit, preview and unavailable history tabs. */
function renderHistoryTabs(properties: PropertiesOf<typeof Tabs>): JSX.Element {
    return (
        <Tabs defaultValue="edit" {...properties}>
            <TabsList aria-label="Note">
                <TabsTrigger value="edit">Edit</TabsTrigger>
                <TabsTrigger value="preview">Preview</TabsTrigger>
                <TabsTrigger value="history" disabled>
                    History
                </TabsTrigger>
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
    description: "the files an editor has open, which close, reorder and end in a menu",
    properties: {},
    render: () => <Files />,
});

/** Render the open files. */
function Files(): JSX.Element {
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
                                        <Icon icon={fileIcon} /> {file}
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

/** Tabs whose mark slides to the selected one. */
export const tabsIndicator = defineExample({
    of: Tabs,
    name: "indicator",
    description: "tabs whose mark slides to the selected one",
    properties: { defaultValue: "week" },
    render: (properties) => (
        <Tabs {...properties}>
            <TabsList aria-label="Range">
                <TabsTrigger value="day">Day</TabsTrigger>
                <TabsTrigger value="week">Week</TabsTrigger>
                <TabsTrigger value="quarter">Quarter</TabsTrigger>
                <TabsIndicator />
            </TabsList>
            <TabsContent value="day">Today</TabsContent>
            <TabsContent value="week">This week</TabsContent>
            <TabsContent value="quarter">This quarter</TabsContent>
        </Tabs>
    ),
});

/** More tabs than fit, which scroll. */
export const tabsScrolling = defineExample({
    of: Tabs,
    name: "scrolling",
    description: "more tabs than fit, which scroll with the sides that have more faded",
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
