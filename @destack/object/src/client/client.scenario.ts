import { defineScenario } from "@destack/package/declare";
import { clientInteraction } from "../scenario/interaction.ts";
import { notesExample } from "../test/note.ts";
import { ObjectClient } from "./client.ts";

/** Predict, confirm and read a person's notebooks through their client's replica. */
export const predictAndConfirm = defineScenario({
    of: ObjectClient,
    interaction: clientInteraction,
    name: "predict-and-confirm",
    description:
        "create notebooks through a person's client, waiting for the prediction or the confirmation, and read them back",
    given: { examples: [notesExample] },
    parameters: { calls: 10 },
    when: [
        {
            operation: "notebook.predict",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "notebook.create",
                as: "alice",
                input: { name: "predicted {n}" },
                until: "predicted",
                bind: "predicted",
            },
        },
        {
            operation: "notebook.confirm",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "notebook.create",
                as: "alice",
                input: { name: "confirmed {n}" },
                until: "confirmed",
                bind: "confirmed",
            },
        },
        {
            operation: "notebook.read",
            times: "{calls}",
            warmup: "{calls}",
            step: { query: "notebook", as: "alice" },
        },
    ],
});
