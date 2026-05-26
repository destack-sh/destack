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
                    fiddle
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
            <DottedFrame depth="medium">
                <header class="grid gap-2 bg-destack-ink px-4 py-3 text-sm font-extrabold lowercase text-destack-panel">
                    <div class="grid min-w-0 gap-4 md:grid-cols-[minmax(0,1fr)_auto]">
                        <PathRow
                            activeIndex={props.areaIndex}
                            labels={exampleAreas.map((item) => `${item.label.toLowerCase()}/`)}
                            onSelect={props.onAreaChange}
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
                        onSelect={props.onCategoryChange}
                    />
                    <PathRow
                        activeIndex={props.topicIndex}
                        labels={props.category.topics.map((item) => `${item.label}.ds`)}
                        onSelect={props.onTopicChange}
                    />
                </header>
            </DottedFrame>
        </div>
    );
}

type PathRowProps = {
    activeIndex: number;
    labels: readonly string[];
    onSelect: (index: number) => void;
};

function PathRow(props: PathRowProps) {
    return (
        <div class="relative min-w-0">
            <div class="pointer-events-none absolute inset-y-0 right-0 z-10 w-4 bg-gradient-to-l from-destack-ink to-transparent" />
            <div class="flex min-w-0 snap-x gap-4 overflow-x-auto overscroll-x-contain pr-5 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
                {props.labels.map((label, index) => (
                    <button
                        class="group flex shrink-0 snap-start items-center gap-2 text-left"
                        classList={{
                            "text-destack-panel": props.activeIndex === index,
                            "text-destack-panel/45 hover:text-destack-panel/80":
                                props.activeIndex !== index,
                        }}
                        onClick={() => props.onSelect(index)}
                        type="button"
                    >
                        <span
                            class="size-2 shrink-0 rounded-full"
                            classList={{
                                "bg-destack-accent": props.activeIndex === index,
                                "bg-destack-panel/25 group-hover:bg-destack-panel/55":
                                    props.activeIndex !== index,
                            }}
                        />
                        <span
                            class="block whitespace-nowrap border-b-4"
                            classList={{
                                "border-destack-accent": props.activeIndex === index,
                                "border-transparent": props.activeIndex !== index,
                            }}
                        >
                            {label}
                        </span>
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
        <div class="flex gap-5 md:justify-end">
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
            class="group flex shrink-0 items-center gap-2 text-left"
            classList={{
                "text-destack-panel": props.isActive,
                "text-destack-panel/45 hover:text-destack-panel/80": !props.isActive,
            }}
            onClick={props.onClick}
            type="button"
        >
            <span
                class="size-2 shrink-0 rounded-full"
                classList={{
                    "bg-destack-accent": props.isActive,
                    "bg-destack-panel/25 group-hover:bg-destack-panel/55": !props.isActive,
                }}
            />
            <span
                class="block border-b-4"
                classList={{
                    "border-destack-accent": props.isActive,
                    "border-transparent": !props.isActive,
                }}
            >
                {props.label}
            </span>
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
