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
import { DottedFrame } from "./dotted";

export function Fiddle() {
    const [areaIndex, setAreaIndex] = createSignal(0);
    const [categoryIndex, setCategoryIndex] = createSignal(0);
    const [topicIndex, setTopicIndex] = createSignal(0);
    const [target, setTarget] = createSignal<OutputTarget>("assembly");
    const area = createMemo(() => exampleAreas[areaIndex()]);
    const category = createMemo(() => area().categories[categoryIndex()]);
    const topic = createMemo(() => category().topics[topicIndex()]);
    const targets = createMemo(() => targetsFor(topic()));
    const selectedTarget = createMemo(() => {
        if (targets().includes(target())) {
            return target();
        }

        return targets()[0];
    });
    const renderedOutput = createMemo(() => highlightedOutputFor(topic().key, selectedTarget()));

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
        <DottedFrame class="h-full min-h-0" depth="large">
            <section class="relative grid min-h-0 min-w-0 grid-rows-[auto_minmax(0,1fr)] border-[2.5px] border-neutral-950 bg-destack-panel">
                <span class="absolute -top-3 left-3 z-20 bg-destack-page px-1 text-sm font-extrabold lowercase">
                    explore
                </span>
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

                <div class="grid min-h-0 min-w-0 grid-cols-[minmax(0,1fr)] gap-4 overflow-hidden bg-destack-panel px-4 pt-4 pb-6 lg:grid-cols-[minmax(0,1fr)_2px_minmax(0,1fr)]">
                    <CodePanel>
                        <code innerHTML={snippets[topic().key]} />
                    </CodePanel>
                    <CodeDivider />
                    <CodePanel>
                        <code innerHTML={renderedOutput()} />
                    </CodePanel>
                </div>
            </section>
        </DottedFrame>
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
    return (
        <div class="relative z-10 m-3 mb-0">
            <header class="grid gap-2 border-[2px] border-neutral-950 bg-[radial-gradient(circle,#d6d0c4_0_1px,transparent_1.25px)] bg-[length:5px_5px] px-4 py-4 text-sm font-extrabold lowercase">
                <div class="grid min-w-0 gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
                    <PathRow
                        activeIndex={props.areaIndex}
                        labels={exampleAreas.map((item) => `${item.label.toLowerCase()}/`)}
                        onSelect={props.onAreaChange}
                        level={0}
                    />
                    <OutputPicker
                        onChange={props.onTargetChange}
                        target={props.target}
                        targets={props.targets}
                    />
                </div>
                <PathRow
                    activeIndex={props.categoryIndex}
                    labels={props.area.categories.map((item) => `${item.label.toLowerCase()}/`)}
                    level={1}
                    onSelect={props.onCategoryChange}
                />
                <PathRow
                    activeIndex={props.topicIndex}
                    labels={props.category.topics.map((item) => `${item.label}.ds`)}
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
            <div class="pointer-events-none absolute inset-y-0 right-0 z-10 w-5 bg-gradient-to-l from-destack-page to-transparent" />
            <div class="flex min-w-0 snap-x gap-2 overflow-x-auto overscroll-x-contain pr-6 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
                {props.labels.map((label, index) => (
                    <button
                        class="group flex shrink-0 snap-start items-center gap-1.5 border bg-destack-panel px-3 py-1.5 text-left"
                        classList={{
                            "border-neutral-950 text-neutral-950": props.activeIndex === index,
                            "border-neutral-950/30 text-neutral-500 shadow-none hover:border-neutral-950/70 hover:text-neutral-950":
                                props.activeIndex !== index,
                        }}
                        onClick={() => props.onSelect(index)}
                        type="button"
                    >
                        <span
                            class="size-2 shrink-0 rounded-full"
                            classList={{
                                "bg-destack-accent": props.activeIndex === index,
                                "bg-neutral-300 group-hover:bg-neutral-500": props.activeIndex !== index,
                            }}
                        />
                        <span class="block whitespace-nowrap">{label}</span>
                    </button>
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
                <OutputButton
                    isActive={props.target === target}
                    label={targetLabel(target)}
                    onClick={() => props.onChange(target)}
                />
            ))}
        </div>
    );
}

type OutputButtonProps = {
    isActive: boolean;
    label: string;
    onClick: () => void;
};

function OutputButton(props: OutputButtonProps) {
    return (
        <button
            class="group flex shrink-0 items-center gap-1.5 border bg-destack-panel px-3 py-1.5 text-left"
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
            <span class="block">{props.label}</span>
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

function targetLabel(target: OutputTarget): string {
    if (target === "assembly") {
        return ".asm";
    }

    if (target === "javascript") {
        return ".js";
    }

    return ".output";
}
