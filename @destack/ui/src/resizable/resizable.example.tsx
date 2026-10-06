import { defineExample } from "@destack/package/declare";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "./resizable.tsx";

/** A notebook list beside the open note, the person choosing the split. */
export const resizablePanelGroupNotebookSplit = defineExample({
    of: ResizablePanelGroup,
    name: "notebook-split",
    description: "a notebook list beside the open note, the person choosing the split",
    render: () => (
        <ResizablePanelGroup direction="horizontal">
            <ResizablePanel defaultSize={30} minSize={20} maxSize={50}>
                Notebooks
            </ResizablePanel>
            <ResizableHandle withHandle aria-label="Resize the notebook list" />
            <ResizablePanel>Note</ResizablePanel>
        </ResizablePanelGroup>
    ),
});

/** A notebook list collapsed beside the open note, its split saved for the next visit. */
export const resizablePanelGroupNotebookCollapsed = defineExample({
    of: ResizablePanelGroup,
    name: "notebook-collapsed",
    description:
        "a notebook list collapsed beside the open note, its split saved for the next visit",
    render: () => (
        <ResizablePanelGroup direction="horizontal" autoSaveId="notebook-split">
            <ResizablePanel defaultSize={0} minSize={20} maxSize={50} collapsible>
                Notebooks
            </ResizablePanel>
            <ResizableHandle withHandle aria-label="Resize the notebook list" />
            <ResizablePanel>Note</ResizablePanel>
        </ResizablePanelGroup>
    ),
});
