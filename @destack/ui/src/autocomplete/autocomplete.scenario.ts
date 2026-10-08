import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { autocompleteFruit } from "./autocomplete.example.tsx";
import { Autocomplete } from "./autocomplete.tsx";

/** Filter a list box by typing, move its focus from the search and choose with Enter. */
export const autocompleteFilterMoveAndChoose = defineScenario({
    of: Autocomplete,
    interaction: viewInteraction,
    name: "filter-move-and-choose",
    description:
        "filter a list box by the typed text and its keywords, move its focus from the search with the arrow keys, and choose with Enter",
    given: { examples: [autocompleteFruit] },
    when: [
        { action: "fill", target: { role: "combobox", name: "Fruit" }, value: "an" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "ArrowDown" },
        { action: "fill", target: { role: "combobox", name: "Fruit" }, value: "trop" },
        { action: "press", key: "Enter" },
    ],
    then: {
        observe: {
            options: { kind: "texts", target: { role: "option" } },
            focused: { kind: "texts", target: { role: "option", selected: true } },
            chosen: { kind: "text", target: { role: "status" } },
        },
        each: [
            { options: ["Banana", "Mango"], focused: ["Banana"], chosen: "" },
            { options: ["Banana", "Mango"], focused: ["Mango"], chosen: "" },
            { options: ["Banana", "Mango"], focused: ["Mango"], chosen: "" },
            { options: ["Mango"], focused: ["Mango"], chosen: "" },
            { options: ["Mango"], focused: ["Mango"], chosen: "Mango" },
        ],
    },
});
