import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { ConversationPage } from "./conversation.tsx";
import { studioAssistant, studioChannel } from "./studio.example.tsx";

/** The thread beside the channel. */
const thread = { css: "aside[aria-label=Thread]" };

/** The last message of the channel, apart from the thread. */
const last = { css: "[data-message-id]:last-child:not(aside *)" };

/** What the last message of the channel answers. */
const quoted = {
    css: "[data-message-id]:last-child:not(aside *) [data-slot=quote]",
};

/** The actions on a message, as its menu lists them. */
const MENU = [
    "React",
    "👍",
    "❤️",
    "🎉",
    "👀",
    "🙏",
    "Reply in threadT",
    "QuoteQ",
    "Copy textC",
    "PinP",
    "Mark unread from hereU",
];

/** Open a thread and ask the assistant in it, quote a message by keys, and open a menu. */
export const chatChannel = defineScenario({
    of: ConversationPage,
    interaction: viewInteraction,
    name: "channel-by-keys",
    description: "open a thread by a key, ask the assistant there, quote by a key, open a menu",
    given: { examples: [studioChannel] },
    parameters: {},
    when: [
        // open the thread of the message under the pointer
        { step: { action: "pointerMove", target: { css: "[data-message-id=which]" } } },
        {
            step: { action: "press", key: "t" },
            then: [{ observe: { kind: "count", target: thread }, is: 1 }],
        },

        // ask the assistant in it, which works and answers
        {
            step: {
                action: "fill",
                target: { label: "Reply…" },
                value: "@assistant compare the two quotes",
            },
        },
        {
            step: { action: "press", key: "Enter", target: { label: "Reply…" } },
            then: [
                {
                    observe: { kind: "count", target: { css: "aside [data-message-id]" } },
                    is: 5,
                },
            ],
        },
        {
            step: { action: "pause", milliseconds: 9000 },
            then: [
                {
                    observe: {
                        kind: "text",
                        target: { css: "aside [data-message-id]:last-child summary" },
                    },
                    is: "Worked for 4 s",
                },
            ],
        },

        // quote the last message of the channel by a key, and answer it
        { step: { action: "focus", target: last } },
        { step: { action: "press", key: "q" } },
        {
            step: {
                action: "fill",
                target: { label: "Message #printing" },
                value: "Going at five",
            },
        },
        {
            step: { action: "press", key: "Enter", target: { label: "Message #printing" } },
            then: [
                {
                    observe: { kind: "text", target: quoted },
                    is: "The printer closes at six today, by the way.",
                },
            ],
        },

        // open the menu of a message by keys
        { step: { action: "focus", target: { css: "[data-message-id=which]:not(aside *)" } } },
        {
            step: { action: "press", key: "Shift+F10" },
            then: [
                {
                    observe: { kind: "texts", target: { css: ":popover-open [role^=menuitem]" } },
                    is: MENU,
                },
            ],
        },
    ],
});

/** Ask the assistant to send a mail, which it asks about first, and let it. */
export const chatApproval = defineScenario({
    of: ConversationPage,
    interaction: viewInteraction,
    name: "approve-a-mail",
    description: "ask the assistant to send a mail, which it asks about first, and let it",
    given: { examples: [studioAssistant] },
    parameters: {},
    when: [
        {
            step: {
                action: "click",
                target: { role: "button", name: "Mail Nordpapier that we order on Friday" },
            },
            then: [
                {
                    observe: { kind: "visible", target: { role: "button", name: "Send the mail" } },
                    is: true,
                },
            ],
        },
        {
            step: { action: "click", target: { role: "button", name: "Send the mail" } },
        },
        {
            step: { action: "pause", milliseconds: 4000 },
            then: [
                {
                    observe: {
                        kind: "text",
                        target: { css: "[data-message-id]:last-child [data-slot=prose]" },
                    },
                    is: "Sent. Nordpapier usually answers within the hour, and I will tell you here.",
                },
            ],
        },
    ],
});
