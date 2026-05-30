import type { JSX } from "solid-js";
import { Show } from "solid-js";

import type { OutputTarget } from "../../../examples";
import { ResizeHandle } from "../component/resize";
import { FileTab, FileTabs } from "../component/tab";
import { FilePanel } from "./file";
import type { Cursor, MarkdownLinkHandler, ViewerFile } from "./types";

type ViewerPanelProps = {
    files: readonly ViewerFile[];
    onCursorChange: (cursor: Cursor) => void;
    onMarkdownLink: MarkdownLinkHandler;
    onResize: (event: PointerEvent) => void;
    onTargetChange: (target: OutputTarget) => void;
    sourceShare: number;
    target: OutputTarget;
    targetLabels: Record<OutputTarget, string>;
    targets: readonly OutputTarget[];
};

export function ViewerPanel(props: ViewerPanelProps) {
    return (
        <section class="h-full min-h-0 min-w-0 overflow-hidden">
            <FileGrid
                files={props.files}
                onCursorChange={props.onCursorChange}
                onMarkdownLink={props.onMarkdownLink}
                onResize={props.onResize}
                onTargetChange={props.onTargetChange}
                sourceShare={props.sourceShare}
                target={props.target}
                targetLabels={props.targetLabels}
                targets={props.targets}
            />
        </section>
    );
}

type FileGridProps = {
    files: readonly ViewerFile[];
    onCursorChange: (cursor: Cursor) => void;
    onMarkdownLink: MarkdownLinkHandler;
    onResize: (event: PointerEvent) => void;
    onTargetChange: (target: OutputTarget) => void;
    sourceShare: number;
    target: OutputTarget;
    targetLabels: Record<OutputTarget, string>;
    targets: readonly OutputTarget[];
};

function FileGrid(props: FileGridProps) {
    return (
        <Show keyed when={props.files}>
            {(files) =>
                files[0]?.kind === "source" ? (
                    <SplitFiles
                        files={files}
                        onCursorChange={props.onCursorChange}
                        onMarkdownLink={props.onMarkdownLink}
                        onResize={props.onResize}
                        onTargetChange={props.onTargetChange}
                        sourceShare={props.sourceShare}
                        target={props.target}
                        targetLabels={props.targetLabels}
                        targets={props.targets}
                    />
                ) : (
                    <TiledFiles
                        files={files}
                        onCursorChange={props.onCursorChange}
                        onMarkdownLink={props.onMarkdownLink}
                    />
                )
            }
        </Show>
    );
}

type SplitFilesProps = {
    files: readonly ViewerFile[];
    onCursorChange: (cursor: Cursor) => void;
    onMarkdownLink: MarkdownLinkHandler;
    onResize: (event: PointerEvent) => void;
    onTargetChange: (target: OutputTarget) => void;
    sourceShare: number;
    target: OutputTarget;
    targetLabels: Record<OutputTarget, string>;
    targets: readonly OutputTarget[];
};

function SplitFiles(props: SplitFilesProps) {
    const [source, output, terminal] = props.files;
    const basename = source.name.replace(/\.ds$/, "");
    const outputTabs = (
        <FileTabs>
            {props.targets.map((target) => (
                <FileTab
                    isActive={props.target === target}
                    label={tabLabelFor(basename, target, props.targetLabels)}
                    onClick={() => props.onTargetChange(target)}
                />
            ))}
        </FileTabs>
    );

    return (
        <div
            class="grid h-full min-h-0 min-w-0 overflow-hidden"
            style={`grid-template-columns: minmax(0, ${props.sourceShare}%) 2px minmax(0, 1fr)`}
        >
            <FilePanel
                file={source}
                onCursorChange={props.onCursorChange}
                onMarkdownLink={props.onMarkdownLink}
            />
            <ResizeHandle label="resize output panel" onPointerDown={props.onResize} />
            <div class="min-h-0 min-w-0 overflow-hidden bg-destack-panel">
                {terminal == undefined ? (
                    <FilePanel file={output} onMarkdownLink={props.onMarkdownLink} />
                ) : (
                    <RightStack
                        onMarkdownLink={props.onMarkdownLink}
                        output={output}
                        outputTabs={outputTabs}
                        terminal={terminal}
                    />
                )}
            </div>
        </div>
    );
}

type RightStackProps = {
    onMarkdownLink: MarkdownLinkHandler;
    output: ViewerFile;
    outputTabs: JSX.Element;
    terminal: ViewerFile;
};

function RightStack(props: RightStackProps) {
    return (
        <div class="grid h-full min-h-0 min-w-0 grid-rows-[minmax(0,2fr)_2px_minmax(0,1fr)] overflow-hidden bg-destack-panel">
            <FilePanel
                file={props.output}
                onMarkdownLink={props.onMarkdownLink}
                tabs={props.outputTabs}
            />
            <Separator />
            <FilePanel file={props.terminal} onMarkdownLink={props.onMarkdownLink} />
        </div>
    );
}

type TiledFilesProps = {
    files: readonly ViewerFile[];
    onCursorChange: (cursor: Cursor) => void;
    onMarkdownLink: MarkdownLinkHandler;
};

function TiledFiles(props: TiledFilesProps) {
    return (
        <div
            class="grid h-full min-h-0 min-w-0 gap-0 overflow-hidden"
            classList={{
                "grid-cols-[minmax(0,1fr)]": props.files.length === 1,
                "lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] lg:grid-rows-2":
                    props.files.length === 3,
                "lg:grid-cols-2 lg:grid-rows-2": props.files.length >= 4,
            }}
        >
            {props.files.slice(0, 4).map((file, index) => (
                <FilePanel
                    classList={{
                        "lg:row-span-2": props.files.length === 3 && index === 0,
                    }}
                    file={file}
                    onCursorChange={props.onCursorChange}
                    onMarkdownLink={props.onMarkdownLink}
                />
            ))}
        </div>
    );
}

function Separator() {
    return <div class="min-h-0 min-w-0 bg-neutral-950" />;
}

function tabLabelFor(
    basename: string,
    target: OutputTarget,
    targetLabels: Record<OutputTarget, string>,
) {
    if (target === "output") {
        return targetLabels[target];
    }

    return `${basename}${targetLabels[target]}`;
}
