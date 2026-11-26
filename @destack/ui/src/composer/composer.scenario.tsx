import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { composerReply } from "./composer.example.tsx";
import { Composer } from "./composer.tsx";

/** Take back what a reply answers and the file it brings. */
export const composerTakeBack = defineScenario({
    of: Composer,
    interaction: viewInteraction,
    name: "takeBack",
    description: t`Take back what a reply answers and the file it brings.`,
    given: { examples: [composerReply] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Remove quote" } },
            then: [
                {
                    observe: { kind: "count", target: { css: "[data-slot=quote]" } },
                    is: 0,
                },
            ],
        },
        {
            step: { action: "click", target: { css: "[data-slot=file-item-remove]" } },
            then: [
                {
                    observe: { kind: "count", target: { css: "[data-slot=file-item]" } },
                    is: 0,
                },
            ],
        },
    ],
});
