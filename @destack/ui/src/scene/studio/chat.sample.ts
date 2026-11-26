import { type Conversation, type ChatMessage, messageOf } from "./chat.ts";

/** The milliseconds of a minute. */
const MINUTE = 60_000;

/** The milliseconds of a day. */
const DAY = 24 * 60 * MINUTE;

/** What the members of the printing channel said, the minutes before now first. */
const PRINTING: readonly (readonly [number, string, string, Partial<ChatMessage>?])[] = [
    [3_100, "grace", "The proof is back. **Colors are right**, the margins are not."],
    [3_099, "grace", "Page 4 has `3 mm` on the left. It needs `5 mm`."],
    [
        3_060,
        "jonas",
        "I can move the block to the right:\n\n```ts\nexport const margin = { left: 5, right: 5 };\n```",
        { reactions: { "👍": ["grace", "ada"] } },
    ],
    [3_055, "ada", "Please do. I will tell the printer it is coming."],
    [1_500, "layla", "هل يمكننا طباعة النسخة العربية في نفس اليوم؟"],
    [1_490, "max", "If both are in one order, yes. Alone it costs the set-up twice."],
    [
        1_400,
        "grace",
        "Before Friday we need:\n\n- [x] the Portuguese text\n- [ ] the Arabic text\n- [ ] one more look at the index",
        { isPinned: true, reactions: { "🙏": ["max"] } },
    ],
    [95, "jonas", "New file is up: https://example.com/flyer-v4.pdf"],
    [94, "jonas", "And two quotes came in. One is cheaper, the other is faster."],
    [40, "grace", "Which of the two do we take?", { id: "which" }],
    [38, "max", "The cheaper one, if Friday is early enough", { thread: "which" }],
    [35, "grace", "Friday works", { thread: "which", reactions: { "🎉": ["max", "jonas"] } }],
    [12, "grace", "The printer closes at six today, by the way."],
];

/** What the members of the launch channel said. */
const LAUNCH: readonly (readonly [number, string, string, Partial<ChatMessage>?])[] = [
    [400, "max", "The page is live on the test address."],
    [390, "ada", "Looks good. The price list still has last year's prices."],
    [20, "max", "Fixed."],
    [19, "max", "Also: who has the picture of the harbour at dusk?"],
];

/** What people say by the thousand. */
const SAID = [
    "Done.",
    "I will look at it after lunch.",
    "The **index** is ready for another read, all forty pages of it, with the new names in.",
    "Can somebody check page 12?",
    "The shelf labels arrived, two boxes short.",
    "See https://example.com/notes for the list.",
];

/** The people of the long conversation. */
const PEOPLE = ["grace", "jonas", "max", "ada", "layla"];

/** Write the messages of a list, each some minutes before now. */
function written(
    id: string,
    list: readonly (readonly [number, string, string, Partial<ChatMessage>?])[],
    now: number,
): readonly ChatMessage[] {
    return list.map(([minutes, author, text, rest], index) =>
        messageOf({
            id: `${id}-${String(index)}`,
            at: now - minutes * MINUTE,
            author,
            text,
            ...rest,
        }),
    );
}

/** Build the studio's conversations, the archive with a count of messages. */
export function sampleConversations(count: number, now: number): readonly Conversation[] {
    const base = { topic: "", read: now, draft: "" };

    return [
        {
            ...base,
            id: "printing",
            name: "printing",
            kind: "channel",
            topic: "The flyer, the index and the labels",
            read: now - 60 * MINUTE,
            messages: written("printing", PRINTING, now),
        },
        {
            ...base,
            id: "launch",
            name: "launch",
            kind: "channel",
            topic: "Everything for the fair in Lisbon",
            read: now - 30 * MINUTE,
            messages: written("launch", LAUNCH, now),
        },
        {
            ...base,
            id: "archive",
            name: "archive",
            kind: "channel",
            topic: "What the studio said since it opened",
            messages: [
                ...Array.from({ length: count }, (_, index) =>
                    messageOf({
                        id: `archive-${String(index)}`,
                        author: PEOPLE[Math.floor(index / 3) % PEOPLE.length] ?? "ada",
                        at: now - DAY - (count - index) * 7 * MINUTE,
                        text: `${SAID[(index * 5) % SAID.length] ?? ""} (${String(index + 1)})`,
                    }),
                ),
                messageOf({
                    id: "archive-answer",
                    author: "grace",
                    at: now - 5 * MINUTE,
                    text: "Still true, a year later.",
                    quote: "archive-100",
                }),
            ],
        },
        {
            ...base,
            id: "grace",
            name: "Grace Hopper",
            kind: "person",
            messages: written(
                "grace",
                [
                    [300, "grace", "Do you have five minutes before the call?"],
                    [298, "ada", "Yes, at ten."],
                ],
                now,
            ),
        },
        { ...base, id: "new", name: "New chat", kind: "assistant", messages: [] },
    ];
}
