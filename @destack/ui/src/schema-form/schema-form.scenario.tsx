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
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "File task" } },
            then: [
                { observe: { kind: "texts", target: { role: "alert" } }, is: ["Enter a value"] },
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "textbox", name: "Title" },
                        name: "aria-invalid",
                    },
                    is: "true",
                },
            ],
        },
        {
            step: { action: "fill", target: { role: "textbox", name: "Title" }, value: "Buy milk" },
            then: [
                { observe: { kind: "texts", target: { role: "alert" } }, is: [] },
                {
                    observe: {
                        kind: "attribute",
                        target: { role: "textbox", name: "Title" },
                        name: "aria-invalid",
                    },
                    is: null,
                },
            ],
        },
    ],
});
