import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { schemaFormTaskInput } from "./schema-form.example.tsx";
import { SchemaForm } from "./schema-form.tsx";

/** Refuse a task without a title on submit, then clear the refusal as the person types one. */
export const schemaFormRefuseMissingTitle = defineScenario({
    of: SchemaForm,
    interaction: viewInteraction,
    name: "refuse-missing-title",
    description:
        "refuse a task without a title when the person submits it, then clear the refusal as they type one",
    given: { examples: [schemaFormTaskInput] },
    when: [
        { action: "click", target: { role: "button", name: "File task" } },
        { action: "fill", target: { role: "textbox", name: "Title" }, value: "Buy milk" },
    ],
    then: {
        observe: {
            refusals: { kind: "texts", target: { role: "alert" } },
            invalid: {
                kind: "attribute",
                target: { role: "textbox", name: "Title" },
                name: "aria-invalid",
            },
        },
        each: [
            { refusals: ["Enter a value"], invalid: "true" },
            { refusals: [], invalid: null },
        ],
    },
});
