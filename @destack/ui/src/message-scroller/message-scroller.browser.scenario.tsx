import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { messageScrollerAssistant, messageScrollerOutline } from "./message-scroller.example.tsx";
import { MessageScroller } from "./message-scroller.tsx";

/** The element that scrolls. */
const viewport = { css: "[data-slot=message-scroller-viewport]" };

/** The button that jumps to the latest message. */
const latest = { css: "[data-slot=message-scroller-button][data-direction=end]" };

/** Read the ends the conversation scrolls toward. */
const scrollable = { kind: "attribute", target: viewport, name: "data-scrollable" } as const;

/** Ask, read the answer grow below the question, go up while it is written, and come back. */
export const messageScrollerStream = defineScenario({
    of: MessageScroller,
    interaction: viewInteraction,
    name: "stream",
    description: t`Ask, read the answer grow below the question, go up while it is written, and come back.`,
    given: { examples: [messageScrollerAssistant] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "button", name: "Send" } },
            then: [
                {
                    observe: {
                        kind: "visible",
                        target: { text: "What should we see on Saturday?" },
                    },
                    is: true,
                },
                { observe: scrollable, is: "start" },
            ],
        },
        // hold the question in view while the answer has room below it
        {
            step: { action: "pause", milliseconds: 1500 },
            then: [
                {
                    observe: {
                        kind: "visible",
                        target: { text: "What should we see on Saturday?" },
                    },
                    is: true,
                },
                { observe: scrollable, is: "start" },
            ],
        },
        // go up, and stay there
        { step: { action: "pause", milliseconds: 2500 } },
        { step: { action: "pointerMove", target: viewport } },
        {
            step: { action: "wheel", deltaY: -200 },
            then: [
                { observe: scrollable, is: "start end" },
                {
                    observe: { kind: "attribute", target: latest, name: "data-active" },
                    is: "true",
                },
            ],
        },
        {
            step: { action: "pause", milliseconds: 1000 },
            then: [{ observe: scrollable, is: "start end" }],
        },
        // come back, and follow to the end
        { step: { action: "wheel", deltaY: 9000 } },
        {
            step: { action: "pause", milliseconds: 1500 },
            then: [
                { observe: scrollable, is: "start" },
                {
                    observe: { kind: "attribute", target: latest, name: "data-active" },
                    is: "false",
                },
            ],
        },
    ],
});

/** Jump to a question from outside, show earlier messages in place, and step by the keys. */
export const messageScrollerNavigate = defineScenario({
    of: MessageScroller,
    interaction: viewInteraction,
    name: "navigate",
    description: t`Jump to a question from outside, go to the start, show earlier messages in place, and step.`,
    given: { examples: [messageScrollerOutline] },
    parameters: {},
    when: [
        { step: { action: "click", target: { role: "button", name: "Question 5" } } },
        {
            step: { action: "pause", milliseconds: 1200 },
            then: [
                {
                    observe: { kind: "text", target: { css: "[data-slot=current]" } },
                    is: "line-8",
                },
                { observe: scrollable, is: "start end" },
            ],
        },
        { step: { action: "click", target: { role: "button", name: "First" } } },
        {
            step: { action: "pause", milliseconds: 1200 },
            then: [{ observe: scrollable, is: "end" }],
        },
        // show what came before, the first message still in view
        {
            step: { action: "click", target: { role: "button", name: "Show earlier messages" } },
            then: [
                {
                    observe: { kind: "visible", target: { css: "[data-message-id=line-0]" } },
                    is: true,
                },
                { observe: scrollable, is: "start end" },
            ],
        },
        { step: { action: "focus", target: { css: "[data-message-id=line-4]" } } },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                {
                    observe: {
                        kind: "attribute",
                        target: { css: "[data-message-id][tabindex='0']" },
                        name: "data-message-id",
                    },
                    is: "line-5",
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [{ observe: scrollable, is: "start" }],
        },
    ],
});
