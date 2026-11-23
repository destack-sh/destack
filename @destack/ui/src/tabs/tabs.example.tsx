import { defineExample, type PropertiesOf } from "@destack/package/declare";
import type { JSX } from "@destack/view";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs.tsx";

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
