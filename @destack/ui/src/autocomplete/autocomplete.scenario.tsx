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
    parameters: {},
    when: [
        {
            step: { action: "fill", target: { role: "combobox", name: "Fruit" }, value: "an" },
            then: [
                { observe: { kind: "texts", target: { role: "option" } }, is: ["Banana", "Mango"] },
                {
                    observe: { kind: "texts", target: { role: "option", selected: true } },
                    is: ["Banana"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "texts", target: { role: "option" } }, is: ["Banana", "Mango"] },
                {
                    observe: { kind: "texts", target: { role: "option", selected: true } },
                    is: ["Mango"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "texts", target: { role: "option" } }, is: ["Banana", "Mango"] },
                {
                    observe: { kind: "texts", target: { role: "option", selected: true } },
                    is: ["Mango"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "fill", target: { role: "combobox", name: "Fruit" }, value: "trop" },
            then: [
                { observe: { kind: "texts", target: { role: "option" } }, is: ["Mango"] },
                {
                    observe: { kind: "texts", target: { role: "option", selected: true } },
                    is: ["Mango"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "" },
            ],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [
                { observe: { kind: "texts", target: { role: "option" } }, is: ["Mango"] },
                {
                    observe: { kind: "texts", target: { role: "option", selected: true } },
                    is: ["Mango"],
                },
                { observe: { kind: "text", target: { role: "status" } }, is: "Mango" },
            ],
        },
    ],
});
