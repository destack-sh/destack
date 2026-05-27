import type { JSX } from "solid-js";
import { createMemo, createSignal } from "solid-js";

import {
    exampleAreas,
    highlightedOutputFor,
    targetsFor,
    type ExampleArea,
    type ExampleCategory,
    type OutputTarget,
} from "../examples";
import { snippets } from "../generated/snippets";
import { Panel } from "./panel";

// file-extension-style label per output target
const targetLabels: Record<OutputTarget, string> = {
    assembly: ".asm",
    javascript: ".js",
    output: ".output",
};

// top-level area labels are static, derive them once
const areaLabels = exampleAreas.map((area) => `${area.label.toLowerCase()}/`);

export function Fiddle() {
    // selection state: area > category > topic, plus output target
    const [areaIndex, setAreaIndex] = createSignal(0);
    const [categoryIndex, setCategoryIndex] = createSignal(0);
    const [topicIndex, setTopicIndex] = createSignal(0);
    const [target, setTarget] = createSignal<OutputTarget>("assembly");

    // current area, category, and topic resolved from the indices
    const area = createMemo(() => exampleAreas[areaIndex()]);
    const category = createMemo(() => area().categories[categoryIndex()]);
    const topic = createMemo(() => category().topics[topicIndex()]);

    // resolve the chosen target against what the topic supports, falling back to the first
    const targets = createMemo(() => targetsFor(topic()));
    const selectedTarget = createMemo(() => {
        const available = targets();
        return available.includes(target()) ? target() : available[0];
    });

    // pre-highlighted compiled output for the current topic + target
    const renderedOutput = createMemo(() => highlightedOutputFor(topic().key, selectedTarget()));

    // picking a new area or category resets the nested indices
    const selectArea = (index: number) => {
        setAreaIndex(index);
        setCategoryIndex(0);
        setTopicIndex(0);
    };
    const selectCategory = (index: number) => {
        setCategoryIndex(index);
        setTopicIndex(0);
    };

    return (
        <Panel class="h-full min-h-0" depth="deep" title="explore">
            <div class="grid h-full min-h-0 min-w-0 grid-rows-[auto_minmax(0,1fr)]">
                {/* path + target picker */}
                <FiddleHeader
                    area={area()}
                    areaIndex={areaIndex()}
                    category={category()}
                    categoryIndex={categoryIndex()}
                    onAreaChange={selectArea}
                    onCategoryChange={selectCategory}
                    onTargetChange={setTarget}
                    onTopicChange={setTopicIndex}
                    target={selectedTarget()}
                    targets={targets()}
                    topicIndex={topicIndex()}
                />

                {/* body */}
                <div class="grid min-h-0 min-w-0 grid-cols-[minmax(0,1fr)] gap-4 overflow-hidden bg-destack-panel px-4 pt-4 pb-6 lg:grid-cols-[minmax(0,1fr)_2px_minmax(0,1fr)]">
                    {/* source */}
                    <CodePanel>
                        <code innerHTML={snippets[topic().key]} />
                    </CodePanel>
                    {/* output */}
                    <CodeDivider />
                    <CodePanel>
                        <code innerHTML={renderedOutput()} />
                    </CodePanel>
                </div>
            </div>
        </Panel>
    );
}

type FiddleHeaderProps = {
    area: ExampleArea;
    areaIndex: number;
    category: ExampleCategory;
    categoryIndex: number;
    onAreaChange: (index: number) => void;
    onCategoryChange: (index: number) => void;
    onTargetChange: (target: OutputTarget) => void;
    onTopicChange: (index: number) => void;
    target: OutputTarget;
    targets: readonly OutputTarget[];
    topicIndex: number;
};

function FiddleHeader(props: FiddleHeaderProps) {
    // category and topic labels depend on the current selection
    const categoryLabels = () =>
        props.area.categories.map((category) => `${category.label.toLowerCase()}/`);
    const topicLabels = () => props.category.topics.map((topic) => `${topic.label}.ds`);

    return (
        <div class="relative z-10 m-3 mb-0">
            <header class="grid gap-2 border-2 border-neutral-950 bg-size-[5px_5px] bg-[radial-gradient(circle,#d6d0c4_0_1px,transparent_1.25px)] px-4 py-4 text-sm font-extrabold lowercase">
                {/* top row: area path + target picker */}
                <div class="grid min-w-0 gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
                    <PathRow
                        activeIndex={props.areaIndex}
                        labels={areaLabels}
                        level={0}
                        onSelect={props.onAreaChange}
                    />
                    <OutputPicker
                        onChange={props.onTargetChange}
                        target={props.target}
                        targets={props.targets}
                    />
                </div>

                {/* category row */}
                <PathRow
                    activeIndex={props.categoryIndex}
                    labels={categoryLabels()}
                    level={1}
                    onSelect={props.onCategoryChange}
                />

                {/* topic row */}
                <PathRow
                    activeIndex={props.topicIndex}
                    labels={topicLabels()}
                    level={2}
                    onSelect={props.onTopicChange}
                />
            </header>
        </div>
    );
}

type PathRowProps = {
    activeIndex: number;
    labels: readonly string[];
    level: 0 | 1 | 2;
    onSelect: (index: number) => void;
};

function PathRow(props: PathRowProps) {
    return (
        <div
            class="relative min-w-0"
            classList={{
                "pl-0": props.level === 0,
                "pl-8": props.level === 1,
                "pl-16": props.level === 2,
            }}
        >
            {/* nesting arrow on every row below the top */}
            {props.level > 0 && (
                <span
                    aria-hidden="true"
                    class="absolute top-1/2 -translate-y-1/2 text-base font-black leading-none text-neutral-400"
                    classList={{
                        "left-3": props.level === 1,
                        "left-11": props.level === 2,
                    }}
                >
                    {"↳"}
                </span>
            )}

            {/* fade out the right edge to hint at horizontal overflow */}
            <div class="pointer-events-none absolute inset-y-0 right-0 z-10 w-5 bg-linear-to-l from-destack-page to-transparent" />

            {/* scrollable row of selectable tags */}
            <div class="flex min-w-0 snap-x gap-2 overflow-x-auto overscroll-x-contain pr-6 scrollbar-none">
                {props.labels.map((label, index) => (
                    <Tag
                        isActive={props.activeIndex === index}
                        label={label}
                        onClick={() => props.onSelect(index)}
                    />
                ))}
            </div>
        </div>
    );
}

type OutputPickerProps = {
    onChange: (target: OutputTarget) => void;
    target: OutputTarget;
    targets: readonly OutputTarget[];
};

function OutputPicker(props: OutputPickerProps) {
    return (
        <div class="flex gap-2 md:justify-end">
            {props.targets.map((target) => (
                <Tag
                    isActive={props.target === target}
                    label={targetLabels[target]}
                    onClick={() => props.onChange(target)}
                />
            ))}
        </div>
    );
}

type TagProps = {
    isActive: boolean;
    label: string;
    onClick: () => void;
};

// selectable bordered tag with a status dot, shared by path rows and target picker
function Tag(props: TagProps) {
    return (
        <button
            class="group flex shrink-0 snap-start items-center gap-1.5 border bg-destack-panel px-3 py-1.5 text-left"
            classList={{
                "border-neutral-950 text-neutral-950": props.isActive,
                "border-neutral-950/30 text-neutral-500 shadow-none hover:border-neutral-950/70 hover:text-neutral-950":
                    !props.isActive,
            }}
            onClick={props.onClick}
            type="button"
        >
            <span
                class="size-2 shrink-0 rounded-full"
                classList={{
                    "bg-destack-accent": props.isActive,
                    "bg-neutral-300 group-hover:bg-neutral-500": !props.isActive,
                }}
            />
            <span class="block whitespace-nowrap">{props.label}</span>
        </button>
    );
}

type CodePanelProps = {
    children: JSX.Element;
};

function CodePanel(props: CodePanelProps) {
    return (
        <section class="grid min-h-0 min-w-0 grid-rows-[minmax(0,1fr)] overflow-hidden bg-destack-panel">
            <pre class="code-block">{props.children}</pre>
        </section>
    );
}

function CodeDivider() {
    return <div aria-hidden="true" class="hidden min-h-0 bg-neutral-950 lg:block" />;
}
