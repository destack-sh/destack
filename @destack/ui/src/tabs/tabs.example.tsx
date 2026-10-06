import { defineExample, type PropertiesOf } from "@destack/package/declare";
import type { JSX } from "@destack/view";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs.tsx";

/** A note's editor and preview as tabs. */
export const tabsEditorPreview = defineExample({
    of: Tabs,
    name: "editor-preview",
    description: "a note's editor and preview as tabs",
    render: () => (
        <Tabs defaultValue="edit">
            <TabsList aria-label="Note">
                <TabsTrigger value="edit">Edit</TabsTrigger>
                <TabsTrigger value="preview">Preview</TabsTrigger>
            </TabsList>
            <TabsContent value="edit">The editor</TabsContent>
            <TabsContent value="preview">The rendered note</TabsContent>
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
