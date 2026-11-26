import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { EmojiPicker } from "./emoji-picker.tsx";
import { emojiPickerReaction } from "./emoji-picker.example.tsx";

/** The emoji the picker lists. */
const emoji = { css: "[data-slot=emoji-picker-emoji]" };

/** Narrow the emoji to a search, and show the note once nothing matches. */
export const emojiPickerSearch = defineScenario({
    of: EmojiPicker,
    interaction: viewInteraction,
    name: "search",
    description: t`Narrow the emoji to a search's matches, and show the note once nothing matches.`,
    given: { examples: [emojiPickerReaction] },
    parameters: {},
    when: [
        {
            step: { action: "fill", target: { label: "Search emoji" }, value: "rocket" },
            then: [{ observe: { kind: "texts", target: emoji }, is: ["🧑‍🚀", "👨‍🚀", "👩‍🚀", "🚀"] }],
        },
        {
            step: { action: "fill", target: { label: "Search emoji" }, value: "zzzz" },
            then: [
                { observe: { kind: "count", target: emoji }, is: 0 },
                {
                    observe: { kind: "visible", target: { css: "[data-slot=emoji-picker-empty]" } },
                    is: true,
                },
            ],
        },
    ],
});
