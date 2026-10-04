import type { JSX } from "@solidjs/web";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "./resizable.tsx";

/** Show a notebook list beside the open note, the person choosing the split. */
export function ResizableExample(): JSX.Element {
    return (
        <ResizablePanelGroup direction="horizontal">
            <ResizablePanel defaultSize={30} minSize={20} maxSize={50}>
                Notebooks
            </ResizablePanel>
            <ResizableHandle withHandle aria-label="Resize the notebook list" />
            <ResizablePanel>Note</ResizablePanel>
        </ResizablePanelGroup>
    );
}
