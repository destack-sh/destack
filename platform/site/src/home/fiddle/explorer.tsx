import { createEffect, createSignal } from "solid-js";

import { exampleAreas, type ExampleArea } from "../../examples";

const areaLabels = exampleAreas.map((area) => `${area.label.toLowerCase()}/`);

export type ExplorerProps = {
    areaIndex: number;
    categoryIndex: number | undefined;
    onAreaChange: (index: number) => void;
    onCategoryChange: (index: number) => void;
    onTopicChange: (index: number) => void;
    topicIndex: number | undefined;
};

export function Explorer(props: ExplorerProps) {
    const [openAreas, setOpenAreas] = createSignal<readonly number[]>([0]);
    const [openCategories, setOpenCategories] = createSignal<readonly string[]>(["0/0"]);

    const isAreaOpen = (areaIndex: number) => openAreas().includes(areaIndex);
    const isCategoryOpen = (areaIndex: number, categoryIndex: number) =>
        openCategories().includes(categoryKey(areaIndex, categoryIndex));
    const openArea = (areaIndex: number) => {
        setOpenAreas(addIndex(openAreas(), areaIndex));
    };
    const openCategory = (areaIndex: number, categoryIndex: number) => {
        setOpenAreas(addIndex(openAreas(), areaIndex));
        setOpenCategories(addIndex(openCategories(), categoryKey(areaIndex, categoryIndex)));
    };
    const toggleArea = (areaIndex: number) => {
        setOpenAreas(toggleIndex(openAreas(), areaIndex));
    };
    const toggleCategory = (areaIndex: number, categoryIndex: number) => {
        setOpenAreas(addIndex(openAreas(), areaIndex));
        setOpenCategories(toggleIndex(openCategories(), categoryKey(areaIndex, categoryIndex)));
    };
    createEffect(() => {
        setOpenAreas((values) => addIndex(values, props.areaIndex));

        if (props.categoryIndex != undefined) {
            const key = categoryKey(props.areaIndex, props.categoryIndex);
            setOpenCategories((values) => addIndex(values, key));
        }
    });

    return (
        <aside class="min-h-0 min-w-0 overflow-auto bg-neutral-50/40 px-1.5 py-2">
            <nav class="grid text-sm font-extrabold leading-5 lowercase">
                {exampleAreas.map((area, areaIndex) => (
                    <TreeArea
                        area={area}
                        areaIndex={areaIndex}
                        categoryIndex={
                            props.areaIndex === areaIndex ? props.categoryIndex : undefined
                        }
                        isExpanded={isAreaOpen(areaIndex)}
                        isActive={props.areaIndex === areaIndex && props.categoryIndex == undefined}
                        isCategoryExpanded={isCategoryOpen}
                        onAreaExpand={toggleArea}
                        onAreaChange={props.onAreaChange}
                        onCategoryExpand={toggleCategory}
                        onCategoryChange={props.onCategoryChange}
                        onOpenArea={openArea}
                        onOpenCategory={openCategory}
                        onTopicChange={props.onTopicChange}
                        topicIndex={props.areaIndex === areaIndex ? props.topicIndex : undefined}
                    />
                ))}
            </nav>
        </aside>
    );
}

type TreeAreaProps = {
    area: ExampleArea;
    areaIndex: number;
    categoryIndex: number | undefined;
    isCategoryExpanded: (areaIndex: number, categoryIndex: number) => boolean;
    isActive: boolean;
    isExpanded: boolean;
    onAreaChange: (index: number) => void;
    onAreaExpand: (index: number) => void;
    onCategoryChange: (index: number) => void;
    onCategoryExpand: (areaIndex: number, categoryIndex: number) => void;
    onOpenArea: (areaIndex: number) => void;
    onOpenCategory: (areaIndex: number, categoryIndex: number) => void;
    onTopicChange: (index: number) => void;
    topicIndex: number | undefined;
};

function TreeArea(props: TreeAreaProps) {
    const selectAreaReadme = () => {
        props.onOpenArea(props.areaIndex);
        props.onAreaChange(props.areaIndex);
    };
    const selectCategory = (categoryIndex: number) => {
        props.onOpenCategory(props.areaIndex, categoryIndex);
        props.onAreaChange(props.areaIndex);
        props.onCategoryChange(categoryIndex);
    };
    const selectTopic = (categoryIndex: number, topicIndex: number) => {
        props.onOpenCategory(props.areaIndex, categoryIndex);
        props.onAreaChange(props.areaIndex);
        props.onCategoryChange(categoryIndex);
        props.onTopicChange(topicIndex);
    };

    return (
        <div class="grid">
            <TreeFolder
                isActive={props.isActive}
                isExpanded={props.isExpanded}
                label={areaLabels[props.areaIndex]}
                onClick={() => props.onAreaExpand(props.areaIndex)}
            />

            {props.isExpanded && (
                <div class="grid pl-3">
                    <TreeFile
                        isActive={props.isActive}
                        label="README.md"
                        onClick={selectAreaReadme}
                    />

                    {props.area.categories.map((category, categoryIndex) => {
                        const isCategoryExpanded = props.isCategoryExpanded(
                            props.areaIndex,
                            categoryIndex,
                        );
                        const isCategoryReadme =
                            props.categoryIndex === categoryIndex && props.topicIndex == undefined;

                        return (
                            <div class="grid">
                                <TreeFolder
                                    isActive={isCategoryReadme}
                                    isExpanded={isCategoryExpanded}
                                    label={`${category.label.toLowerCase()}/`}
                                    onClick={() =>
                                        props.onCategoryExpand(props.areaIndex, categoryIndex)
                                    }
                                />

                                {isCategoryExpanded && (
                                    <div class="grid pl-3">
                                        <TreeFile
                                            isActive={isCategoryReadme}
                                            label="README.md"
                                            onClick={() => selectCategory(categoryIndex)}
                                        />
                                        {category.topics.map((topic, topicIndex) => (
                                            <TreeFile
                                                isActive={props.topicIndex === topicIndex}
                                                label={`${topic.label}.ds`}
                                                onClick={() =>
                                                    selectTopic(categoryIndex, topicIndex)
                                                }
                                            />
                                        ))}
                                    </div>
                                )}
                            </div>
                        );
                    })}
                </div>
            )}
        </div>
    );
}

function categoryKey(areaIndex: number, categoryIndex: number) {
    return `${areaIndex}/${categoryIndex}`;
}

function addIndex<T>(values: readonly T[], value: T) {
    if (values.includes(value)) {
        return values;
    }

    return [...values, value];
}

function toggleIndex<T>(values: readonly T[], value: T) {
    if (!values.includes(value)) {
        return [...values, value];
    }

    return values.filter((item) => item !== value);
}

type TreeFolderProps = {
    isActive: boolean;
    isExpanded: boolean;
    label: string;
    onClick: () => void;
};

function TreeFolder(props: TreeFolderProps) {
    return (
        <button
            class="group grid min-h-6 min-w-0 grid-cols-[0.875rem_2px_minmax(0,1fr)] items-center gap-1 text-left"
            classList={{
                "bg-destack-panel text-neutral-950": props.isActive,
                "text-neutral-600 hover:bg-destack-panel hover:text-neutral-950": !props.isActive,
            }}
            onClick={props.onClick}
            type="button"
        >
            <span
                aria-hidden="true"
                class="text-center text-xs text-neutral-400 group-hover:text-neutral-700"
            >
                {props.isExpanded ? "▾" : "▸"}
            </span>
            <span
                aria-hidden="true"
                class="h-3.5 w-0.5"
                classList={{
                    "bg-destack-accent": props.isActive,
                    "bg-transparent": !props.isActive,
                }}
            />
            <span class="min-w-0 truncate">{props.label}</span>
        </button>
    );
}

type TreeFileProps = {
    isActive: boolean;
    label: string;
    onClick: () => void;
};

function TreeFile(props: TreeFileProps) {
    return (
        <button
            class="group grid min-h-6 min-w-0 grid-cols-[0.875rem_2px_minmax(0,1fr)] items-center gap-1 text-left"
            classList={{
                "bg-destack-panel text-neutral-950": props.isActive,
                "text-neutral-500 hover:bg-destack-panel hover:text-neutral-950": !props.isActive,
            }}
            onClick={props.onClick}
            type="button"
        >
            <span
                aria-hidden="true"
                class="mx-auto size-1.5 rounded-full"
                classList={{
                    "bg-destack-accent": props.isActive,
                    "bg-neutral-300 group-hover:bg-neutral-500": !props.isActive,
                }}
            />
            <span
                aria-hidden="true"
                class="h-3.5 w-0.5"
                classList={{
                    "bg-destack-accent": props.isActive,
                    "bg-transparent": !props.isActive,
                }}
            />
            <span class="min-w-0 truncate">{props.label}</span>
        </button>
    );
}
