import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { feedConversation } from "./feed.example.tsx";
import { Feed } from "./feed.tsx";

/** The conversation. */
const feed = { css: "[data-slot=feed]" };

/** Read a conversation from its end, load older messages above and follow a new one. */
export const feedReadConversation = defineScenario({
    of: Feed,
    interaction: viewInteraction,
    name: "readConversation",
    description: t`Open a conversation at its newest message, load older ones without losing the place, and follow a new message only from the end.`,
    given: { examples: [feedConversation] },
    parameters: {},
    when: [
        {
            step: { action: "pause", milliseconds: 50 },
            then: [
                {
                    observe: { kind: "visible", target: { role: "article", name: "Message 1029" } },
                    is: true,
                },
            ],
        },
        { step: { action: "pointerMove", target: feed } },
        {
            step: { action: "wheel", deltaY: -3000 },
            then: [
                {
                    observe: { kind: "visible", target: { role: "article", name: "Message 1000" } },
                    is: true,
                },
                { observe: { kind: "attribute", target: feed, name: "aria-busy" }, is: "true" },
            ],
        },
        {
            step: { action: "pause", milliseconds: 700 },
            then: [
                { observe: { kind: "attribute", target: feed, name: "aria-busy" }, is: "false" },
                {
                    observe: { kind: "visible", target: { role: "article", name: "Message 1000" } },
                    is: true,
                },
                {
                    observe: { kind: "count", target: { role: "article", name: "Message 999" } },
                    is: 1,
                },
            ],
        },
        {
            step: { action: "click", target: { role: "button", name: "Send a message" } },
            then: [
                {
                    observe: { kind: "visible", target: { role: "article", name: "Message 1000" } },
                    is: true,
                },
            ],
        },
        { step: { action: "pointerMove", target: feed } },
        {
            step: { action: "wheel", deltaY: 9000 },
            then: [
                {
                    observe: { kind: "visible", target: { role: "article", name: "Message 1030" } },
                    is: true,
                },
            ],
        },
        {
            step: { action: "click", target: { role: "button", name: "Send a message" } },
            then: [
                {
                    observe: { kind: "visible", target: { role: "article", name: "Message 1031" } },
                    is: true,
                },
            ],
        },
    ],
});
