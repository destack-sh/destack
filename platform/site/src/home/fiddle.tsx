import { createEffect, createMemo, createSignal } from "solid-js";

import {
    exampleAreas,
    highlightedOutputFor,
    outputFor,
    targetsFor,
    type OutputTarget,
} from "../examples";
import { snippets } from "../generated/snippets";
import { ResizeHandle, dragHorizontally } from "./fiddle/component/resize";
import { Explorer } from "./fiddle/explorer";
import { StatusBar } from "./fiddle/panel/status";
import type { Cursor, Status, ViewerFile } from "./fiddle/panel/types";
import { ViewerPanel } from "./fiddle/panel/viewer";
import { readmeForArea, readmeForCategory } from "./fiddle/readme";
import { Panel } from "./panel";

const minimumTreeWidth = 220;
const maximumTreeWidth = 360;
const minimumSourceShare = 36;
const maximumSourceShare = 66;

const targetLabels: Record<OutputTarget, string> = {
    assembly: ".asm",
    javascript: ".js",
    output: "terminal",
};

export function Fiddle() {
    const [areaIndex, setAreaIndex] = createSignal(0);
    const [categoryIndex, setCategoryIndex] = createSignal<number | undefined>(0);
    const [topicIndex, setTopicIndex] = createSignal<number | undefined>(0);
    const [treeWidth, setTreeWidth] = createSignal(248);
    const [sourceShare, setSourceShare] = createSignal(52);
    const [target, setTarget] = createSignal<OutputTarget>("assembly");
    const [cursor, setCursor] = createSignal<Cursor>({ column: 1, line: 1 });

    const area = createMemo(() => exampleAreas[areaIndex()]);
    const category = createMemo(() => {
        const index = categoryIndex();
        return index == undefined ? undefined : area().categories[index];
    });
    const topic = createMemo(() => {
        const categoryValue = category();
        const index = topicIndex();
        return categoryValue == undefined || index == undefined
            ? undefined
            : categoryValue.topics[index];
    });

    const targets = createMemo(() => {
        const topicValue = topic();
        return topicValue == undefined ? [] : targetsFor(topicValue);
    });
    const selectedTarget = createMemo(() => {
        const available = targets();
        return available.includes(target()) ? target() : available[0] ?? "assembly";
    });

    const files = createMemo<readonly ViewerFile[]>(() => {
        const topicValue = topic();
        if (topicValue != undefined) {
            const outputTarget = selectedTarget();

            const source = {
                kind: "source",
                name: `${topicValue.label}.ds`,
                path: `${area().label.toLowerCase()}/${topicValue.key}.ds`,
                html: snippets[topicValue.key].html,
                source: snippets[topicValue.key].source,
            } satisfies ViewerFile;
            const terminal = {
                kind: "terminal",
                name: "terminal",
                path: `${area().label.toLowerCase()}/${topicValue.key}.output`,
                html: highlightedOutputFor(topicValue.key, "output"),
                source: outputFor(topicValue.key, "output"),
            } satisfies ViewerFile;

            if (outputTarget === "output") {
                return [source, terminal];
            }

            return [
                source,
                {
                    kind: "output",
                    name: `${topicValue.label}${targetLabels[outputTarget]}`,
                    path: `${area().label.toLowerCase()}/${topicValue.key}${targetLabels[outputTarget]}`,
                    html: highlightedOutputFor(topicValue.key, outputTarget),
                    source: outputFor(topicValue.key, outputTarget),
                },
                terminal,
            ];
        }

        const categoryValue = category();
        if (categoryValue != undefined) {
            return [
                {
                    kind: "markdown",
                    name: "README.md",
                    path: `${area().label.toLowerCase()}/${categoryValue.label.toLowerCase()}/README.md`,
                    ...readmeForCategory(area(), categoryValue),
                },
            ];
        }

        return [
            {
                kind: "markdown",
                name: "README.md",
                path: `${area().label.toLowerCase()}/README.md`,
                ...readmeForArea(area()),
            },
        ];
    });

    const status = createMemo<Status>(() => {
        const [file] = files();
        const source = file?.source ?? "";
        const lines = source === "" ? 0 : source.split("\n").length;

        return {
            cursor: cursor(),
            file: file?.name ?? "README.md",
            files: files().length,
            lines,
            mode: file?.kind ?? "markdown",
        };
    });
    createEffect(() => {
        const [file] = files();
        if (file?.kind !== "source") {
            setCursor({ column: 1, line: 1 });
        }
    });

    const selectArea = (index: number) => {
        setAreaIndex(index);
        setCategoryIndex(undefined);
        setTopicIndex(undefined);
    };
    const selectCategory = (index: number) => {
        setCategoryIndex(index);
        setTopicIndex(undefined);
    };
    const selectMarkdownLink = (href: string, basePath: string) => {
        const path = markdownPath(href, basePath);
        if (path == undefined) {
            return false;
        }

        const target = selectionFor(path);
        if (target == undefined) {
            return false;
        }

        setAreaIndex(target.areaIndex);
        setCategoryIndex(target.categoryIndex);
        setTopicIndex(target.topicIndex);

        return true;
    };
    const resizeTree = (event: PointerEvent) => {
        const start = treeWidth();
        const origin = event.clientX;

        dragHorizontally(event, (clientX) => {
            const width = start + clientX - origin;
            setTreeWidth(clamp(width, minimumTreeWidth, maximumTreeWidth));
        });
    };
    const resizeSource = (event: PointerEvent) => {
        const container = (event.currentTarget as HTMLElement).parentElement;
        if (container == undefined) {
            return;
        }

        const bounds = container.getBoundingClientRect();
        dragHorizontally(event, (clientX) => {
            const share = ((clientX - bounds.left) / bounds.width) * 100;
            setSourceShare(clamp(share, minimumSourceShare, maximumSourceShare));
        });
    };

    return (
        <Panel class="h-full min-h-0" depth="deep" title="explore">
            <div
                class="grid h-full min-h-0 min-w-0 grid-rows-[minmax(0,1fr)_1.75rem] overflow-hidden bg-destack-panel"
                style={`--tree-width: ${treeWidth()}px`}
            >
                <div class="grid min-h-0 min-w-0 grid-cols-[minmax(0,1fr)] overflow-hidden lg:grid-cols-[var(--tree-width)_2px_minmax(0,1fr)]">
                    <Explorer
                        areaIndex={areaIndex()}
                        categoryIndex={categoryIndex()}
                        onAreaChange={selectArea}
                        onCategoryChange={selectCategory}
                        onTopicChange={setTopicIndex}
                        topicIndex={topicIndex()}
                    />

                    <ResizeHandle label="resize file tree" onPointerDown={resizeTree} />

                    <div class="min-h-0 min-w-0 overflow-hidden border-t-2 border-neutral-950 bg-destack-panel lg:border-t-0">
                        <ViewerPanel
                            files={files()}
                            onCursorChange={setCursor}
                            onMarkdownLink={selectMarkdownLink}
                            onResize={resizeSource}
                            onTargetChange={setTarget}
                            sourceShare={sourceShare()}
                            target={selectedTarget()}
                            targetLabels={targetLabels}
                            targets={targets()}
                        />
                    </div>
                </div>

                <StatusBar status={status()} />
            </div>
        </Panel>
    );
}

function clamp(value: number, minimum: number, maximum: number) {
    return Math.max(minimum, Math.min(maximum, value));
}

function markdownPath(href: string, basePath: string) {
    const hrefPath = href.split(/[?#]/, 1)[0]?.trim();
    if (hrefPath == undefined || hrefPath === "" || isExternal(hrefPath)) {
        return undefined;
    }

    const path = hrefPath.startsWith("/") ? hrefPath.slice(1) : hrefPath;
    const first = path.split("/", 1)[0];
    const base = exampleAreas.some((area) => area.label.toLowerCase() === first)
        ? []
        : basePath.split("/").slice(0, -1);

    return normalizePath([...base, ...path.split("/")]);
}

function normalizePath(parts: readonly string[]) {
    const path: string[] = [];
    for (const part of parts) {
        if (part === "" || part === ".") {
            continue;
        }

        if (part === "..") {
            path.pop();
            continue;
        }

        path.push(part);
    }

    return path.join("/");
}

function selectionFor(path: string) {
    const [areaName, categoryName, fileName] = path.split("/");
    const areaIndex = exampleAreas.findIndex(
        (area) => area.label.toLowerCase() === areaName?.toLowerCase(),
    );
    if (areaIndex < 0) {
        return undefined;
    }

    if (categoryName == undefined || categoryName === "README.md") {
        return { areaIndex, categoryIndex: undefined, topicIndex: undefined };
    }

    const areaValue = exampleAreas[areaIndex];
    const categoryIndex = areaValue.categories.findIndex(
        (category) => category.label.toLowerCase() === categoryName.toLowerCase(),
    );
    if (categoryIndex < 0) {
        return undefined;
    }

    if (fileName == undefined || fileName === "README.md") {
        return { areaIndex, categoryIndex, topicIndex: undefined };
    }

    const categoryValue = areaValue.categories[categoryIndex];
    const topicIndex = categoryValue.topics.findIndex((topic) => {
        const sourceName = `${topic.label}.ds`;
        const keyName = `${topic.key.split("/").at(-1)}.ds`;

        return fileName === sourceName || fileName === keyName;
    });
    if (topicIndex < 0) {
        return undefined;
    }

    return { areaIndex, categoryIndex, topicIndex };
}

function isExternal(path: string) {
    return /^[a-z][a-z0-9+.-]*:/i.test(path) || path.startsWith("//");
}
