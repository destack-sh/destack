import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { Sortable, SortableContainer, SortableHandle, SortableItem } from "./sortable.tsx";

/** A checklist whose steps a drag or the keyboard reorders. */
export const sortableChecklist = defineExample({
    of: Sortable,
    name: "checklist",
    description:
        "a checklist whose steps a drag or the keyboard reorders, the owner placing each moved step between its new neighbours",
    properties: {
        nesting: false,
    },
    render: (properties) => {
        const [steps, setSteps] = createSignal(["Pack", "Check in", "Board"]);

        return (
            <Sortable
                {...properties}
                onMove={(move) => {
                    // place the step after the one it lands after
                    const rest = steps().filter((step) => step !== move.id);
                    const index = move.previous === undefined ? 0 : rest.indexOf(move.previous) + 1;
                    setSteps([...rest.slice(0, index), move.id, ...rest.slice(index)]);
                }}
            >
                <SortableContainer id="steps" label="Steps">
                    <For each={steps()}>
                        {(step) => (
                            <SortableItem id={step}>
                                <SortableHandle /> {step}
                            </SortableItem>
                        )}
                    </For>
                </SortableContainer>
            </Sortable>
        );
    },
});

/** A checklist whose steps stay in their order. */
export const sortableChecklistFixed = defineExample({
    of: Sortable,
    name: "checklist-fixed",
    description: "a checklist whose steps stay in their order",
    render: () => {
        const [steps, setSteps] = createSignal(["Pack", "Check in", "Board"]);

        return (
            <Sortable
                onMove={(move) => {
                    // place the step after the one it lands after
                    const rest = steps().filter((step) => step !== move.id);
                    const index = move.previous === undefined ? 0 : rest.indexOf(move.previous) + 1;
                    setSteps([...rest.slice(0, index), move.id, ...rest.slice(index)]);
                }}
            >
                <SortableContainer id="steps" label="Steps">
                    <For each={steps()}>
                        {(step) => (
                            <SortableItem id={step} disabled>
                                <SortableHandle /> {step}
                            </SortableItem>
                        )}
                    </For>
                </SortableContainer>
            </Sortable>
        );
    },
});

/** A board of two columns whose cards move within and between the columns. */
export const sortableBoard = defineExample({
    of: Sortable,
    name: "board",
    description: "a board of two columns whose cards move within and between the columns",
    render: () => (
        <Sortable onMove={() => undefined}>
            <SortableContainer id="todo" label="To do">
                <SortableItem id="draft">
                    <SortableHandle /> Draft the post
                </SortableItem>
                <SortableItem id="review">
                    <SortableHandle /> Review the post
                </SortableItem>
            </SortableContainer>
            <SortableContainer id="done" label="Done">
                <SortableItem id="outline">
                    <SortableHandle /> Outline the post
                </SortableItem>
            </SortableContainer>
        </Sortable>
    ),
});
