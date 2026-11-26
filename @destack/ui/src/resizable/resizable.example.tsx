import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "./resizable.tsx";

/** The styles of the split: a framed pair of panes with room around their text. */
const styles = style.create({
    split: {
        height: "20rem",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        overflow: "hidden",
    },
    fixed: {
        width: "800px",
        maxWidth: "100%",
    },
    pane: {
        height: "100%",
        padding: space[4],
    },
    list: {
        backgroundColor: color.muted,
    },
    note: {
        margin: 0,
        color: color.mutedForeground,
    },
});

/** A notebook list beside the open note, the person choosing the split. */
export const resizablePanelGroupNotebookSplit = defineExample({
    of: ResizablePanelGroup,
    name: "notebookSplit",
    description: t`A notebook list beside the open note, the person choosing the split.`,
    layout: "padded",
    properties: {
        direction: "horizontal",
    },
    render: (properties) => (
        <ResizablePanelGroup {...properties} xstyle={styles.split}>
            <ResizablePanel defaultSize={30} minSize={20} maxSize={50}>
                <div {...style.attrs(text.sm, styles.pane, styles.list)}>Notebooks</div>
            </ResizablePanel>
            <ResizableHandle withHandle aria-label="Resize the notebook list" />
            <ResizablePanel>
                <div {...style.attrs(text.sm, styles.pane)}>
                    <p {...style.attrs(styles.note)}>Trip to Lisbon</p>
                </div>
            </ResizablePanel>
        </ResizablePanelGroup>
    ),
});

/** A notebook list collapsed beside the open note, its split saved for the next visit. */
export const resizablePanelGroupNotebookCollapsed = defineExample({
    of: ResizablePanelGroup,
    name: "notebookCollapsed",
    description: t`A notebook list collapsed beside the open note, its split saved for the next visit.`,
    layout: "padded",
    properties: {
        direction: "horizontal",
        autoSaveId: "notebook-split",
    },
    render: (properties) => (
        <ResizablePanelGroup {...properties} xstyle={styles.split}>
            <ResizablePanel defaultSize={0} minSize={20} maxSize={50} collapsible>
                <div {...style.attrs(text.sm, styles.pane, styles.list)}>Notebooks</div>
            </ResizablePanel>
            <ResizableHandle withHandle aria-label="Resize the notebook list" />
            <ResizablePanel>
                <div {...style.attrs(text.sm, styles.pane)}>
                    <p {...style.attrs(styles.note)}>Trip to Lisbon</p>
                </div>
            </ResizablePanel>
        </ResizablePanelGroup>
    ),
});

/** An inbox beside the open message in an 800px split, its list between 240 and 480 pixels. */
export const resizablePanelGroupInboxSplit = defineExample({
    of: ResizablePanelGroup,
    name: "inboxSplit",
    description: t`An inbox beside the open message, its list between 240 and 480 pixels.`,
    layout: "padded",
    properties: {
        direction: "horizontal",
    },
    render: (properties) => (
        <ResizablePanelGroup {...properties} xstyle={[styles.split, styles.fixed]}>
            <ResizablePanel defaultSize={40} minSize="240px" maxSize="480px">
                <div {...style.attrs(text.sm, styles.pane, styles.list)}>Inbox</div>
            </ResizablePanel>
            <ResizableHandle withHandle aria-label="Resize the inbox" />
            <ResizablePanel>
                <div {...style.attrs(text.sm, styles.pane)}>
                    <p {...style.attrs(styles.note)}>Booking for Saturday</p>
                </div>
            </ResizablePanel>
        </ResizablePanelGroup>
    ),
});
