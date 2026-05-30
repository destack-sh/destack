import type { JSX } from "solid-js";
import { Show } from "solid-js";

import { FileTab, FileTabs } from "../component/tab";
import { CodePanel } from "./code";
import { MarkdownPanel } from "./markdown";
import { Pane } from "./pane";
import { SourcePanel } from "./source";
import { TerminalPanel } from "./terminal";
import type { Cursor, MarkdownLinkHandler, ViewerFile } from "./types";

type FilePanelProps = {
    classList?: Record<string, boolean>;
    file: ViewerFile;
    onCursorChange?: (cursor: Cursor) => void;
    onMarkdownLink: MarkdownLinkHandler;
    tabs?: JSX.Element;
};

export function FilePanel(props: FilePanelProps) {
    return (
        <Show keyed when={props.file}>
            {(file) => (
                <Pane
                    bodyClass="overflow-hidden"
                    class="border-neutral-950"
                    classList={props.classList}
                    tabs={
                        props.tabs ?? (
                            <FileTabs>
                                <FileTab
                                    isActive={true}
                                    label={file.name}
                                    onClick={() => undefined}
                                />
                            </FileTabs>
                        )
                    }
                >
                    {file.kind === "markdown" ? (
                        <MarkdownPanel
                            html={file.html}
                            onLink={(href) => props.onMarkdownLink(href, file.path)}
                        />
                    ) : file.kind === "source" ? (
                        <SourcePanel
                            html={file.html}
                            onCursorChange={props.onCursorChange}
                            source={file.source}
                        />
                    ) : file.kind === "terminal" ? (
                        <TerminalPanel html={file.html} />
                    ) : (
                        <CodePanel html={file.html} />
                    )}
                </Pane>
            )}
        </Show>
    );
}
