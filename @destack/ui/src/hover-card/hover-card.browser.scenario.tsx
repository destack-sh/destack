import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { HoverCard } from "./hover-card.tsx";
import { hoverCardMention } from "./hover-card.example.tsx";

/** The profile the mention's card shows. */
const profile = { text: "Ada Lovelace" };

/** Open a hover card once the pointer rests on its link, and close it once the pointer leaves. */
export const hoverCardRestAndLeave = defineScenario({
    of: HoverCard,
    interaction: viewInteraction,
    name: "restAndLeave",
    description: t`Open a hover card once the pointer rests on its link past the delay, and close it after.`,
    given: { examples: [hoverCardMention] },
    parameters: {},
    when: [
        {
            step: { action: "pointerMove", target: { role: "link", name: "@ada" } },
            then: [{ observe: { kind: "visible", target: profile }, is: false }],
        },
        {
            step: { action: "pause", milliseconds: 900 },
            then: [{ observe: { kind: "visible", target: profile }, is: true }],
        },
        {
            step: {
                action: "pointerMove",
                target: { role: "link", name: "@ada" },
                position: { x: 400, y: -40 },
            },
        },
        {
            step: { action: "pause", milliseconds: 600 },
            then: [{ observe: { kind: "visible", target: profile }, is: false }],
        },
    ],
});
