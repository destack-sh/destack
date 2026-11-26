import type { Clock } from "@destack/environment";
import type { Chat } from "./chat.ts";
import { type Answer, ASSISTANT, type Source, type Step } from "./chat.ts";

/** The milliseconds a step takes. */
const STEP = 1_400;

/** The pieces an answer comes in, uneven as a network brings them. */
const PIECES = [
    { length: 18, milliseconds: 60 },
    { length: 41, milliseconds: 140 },
    { length: 9, milliseconds: 40 },
    { length: 64, milliseconds: 260 },
    { length: 22, milliseconds: 90 },
] as const;

/** What the assistant's answers use. */
const SOURCES: readonly Source[] = [
    {
        address: "https://example.com/nordpapier",
        title: "Price list, autumn 2026",
        site: "Nordpapier",
        excerpt: "Uncoated 120 g, 1,000 sheets: €84. Delivery in three days.",
    },
    {
        address: "https://example.com/druckhaus",
        title: "Flyers and posters",
        site: "Druckhaus Elbe",
        excerpt: "Uncoated 150 g, 1,000 sheets: €97. Proof within a day.",
    },
];

/** What the assistant does, and answers. */
const COMPARISON = {
    steps: [
        {
            kind: "thought",
            title: "Comparing the two quotes",
            text: "Both are without tax, so the totals compare as they are.",
        },
        { kind: "read", title: "Read quotes.pdf", text: "Two offers for 1,000 sheets." },
        {
            kind: "search",
            title: "Searched for delivery times",
            text: "Found the price list of Nordpapier and the page of Druckhaus Elbe.",
        },
    ],
    text: [
        "**Nordpapier is cheaper, and Druckhaus Elbe is faster.**",
        "Nordpapier prints 1,000 sheets for €84. [Nordpapier](https://example.com/nordpapier) " +
            "Druckhaus Elbe asks €97, with a proof within a day. " +
            "[Druckhaus Elbe](https://example.com/druckhaus)",
        "| | Paper | 1,000 sheets | Ready |\n| --- | --- | ---: | --- |\n" +
            "| Nordpapier | 120 g | €84 | Friday |\n| Druckhaus Elbe | 150 g | €97 | Wednesday |",
        "What I would do:\n\n1. Take **Nordpapier** if Friday is early enough\n" +
            "2. Ask both for a sample first\n3. Keep the margins at `5 mm`",
    ].join("\n\n"),
} as const;

/** What the assistant asks before it does it. */
const MAIL = {
    question: {
        kind: "ask",
        title: "Send a mail to Nordpapier",
        text: "Hello, we would like to order 1,000 sheets of the 120 g paper for Friday.",
    },
    sent: "Sent. Nordpapier usually answers within the hour, and I will tell you here.",
    kept: "I left it. The mail is still here if you want to send it later.",
} as const;

/** The assistant of the studio, who answers by rote, a piece at a time. */
export class Assistant {
    /** The chat it answers in. */
    readonly #chat: Chat;
    /** The clock it takes its time by. */
    readonly #clock: Clock;
    /** What stops each answer under way, by its id. */
    readonly #running = new Map<string, AbortController>();
    /** The number of the next answer. */
    #next = 0;

    /** Answer in a chat's conversations. */
    constructor(chat: Chat, clock: Clock) {
        this.#chat = chat;
        this.#clock = clock;
    }

    /** Answer a question below it, and return the message's id. */
    answer(conversation: string, question: string, thread: string | undefined): string {
        const id = this.#chat.say(conversation, { author: ASSISTANT, text: "", thread });
        this.#begin(id, question);

        return id;
    }

    /** Answer a message's question once more, beside the answers before. */
    retry(message: string, question: string): void {
        this.#begin(message, question);
    }

    /** Stop what a message's answer is at. */
    stop(message: string): void {
        for (const { id } of this.#chat.message(message)?.answers ?? []) {
            this.#running.get(id)?.abort();
        }
    }

    /** Stop everything. */
    close(): void {
        for (const running of this.#running.values()) {
            running.abort();
        }
    }

    /** Do what was asked about, or leave it. */
    decide(message: string, answer: string, isAllowed: boolean): void {
        this.#chat.changeAnswer(message, answer, ({ steps }) => ({
            steps: steps.map((step) =>
                step.status === "waiting"
                    ? { ...step, status: isAllowed ? ("done" as const) : ("denied" as const) }
                    : step,
            ),
        }));
        this.#run(message, answer, (signal) =>
            this.#write(message, answer, isAllowed ? MAIL.sent : MAIL.kept, signal),
        );
    }

    /** Begin an answer, and show it. */
    #begin(message: string, question: string): void {
        // add it after the others
        this.#next += 1;
        const answer: Answer = {
            id: `answer-${String(this.#next)}`,
            steps: [],
            text: "",
            sources: SOURCES,
            status: "working",
            began: this.#clock.now(),
            wrote: undefined,
        };
        this.#chat.change(message, ({ answers }) => ({
            answers: [...answers, answer],
            shown: answers.length,
        }));

        // ask before a mail, else compare
        const isMail = /mail|send/iu.test(question);
        this.#run(message, answer.id, async (signal) => {
            if (isMail) {
                this.#step(message, answer.id, { ...MAIL.question, status: "waiting" });

                return;
            }
            for (const step of COMPARISON.steps) {
                this.#step(message, answer.id, { ...step, status: "running" });
                await this.#clock.sleep(STEP, signal);
                this.#finish(message, answer.id);
            }
            await this.#write(message, answer.id, COMPARISON.text, signal);
        });
    }

    /** Run what an answer is at, until it ends or is stopped. */
    #run(message: string, answer: string, work: (signal: AbortSignal) => Promise<void>): void {
        const controller = new AbortController();
        this.#running.set(answer, controller);
        work(controller.signal)
            .catch((error: unknown) => {
                if (!controller.signal.aborted) {
                    reportError(error);
                }
                this.#finish(message, answer);
                this.#chat.changeAnswer(message, answer, () => ({ status: "stopped" }));
            })
            .finally(() => this.#running.delete(answer));
    }

    /** Add a step. */
    #step(message: string, answer: string, step: Omit<Step, "id">): void {
        this.#chat.changeAnswer(message, answer, ({ steps }) => ({
            steps: [...steps, { ...step, id: `${answer}-${String(steps.length)}` }],
        }));
    }

    /** Finish the step under way. */
    #finish(message: string, answer: string): void {
        this.#chat.changeAnswer(message, answer, ({ steps }) => ({
            steps: steps.map((step) =>
                step.status === "running" ? { ...step, status: "done" as const } : step,
            ),
        }));
    }

    /** Write a text a piece at a time. */
    async #write(
        message: string,
        answer: string,
        text: string,
        signal: AbortSignal,
    ): Promise<void> {
        // write it as fast as a network brings it
        const wrote = this.#clock.now();
        this.#chat.changeAnswer(message, answer, () => ({ status: "writing", wrote }));
        for (let from = 0, turn = 0; from < text.length; turn += 1) {
            const { length, milliseconds } = PIECES[turn % PIECES.length] ?? PIECES[0];
            await this.#clock.sleep(milliseconds, signal);
            from += length;
            this.#chat.changeAnswer(message, answer, () => ({
                text: text.slice(0, from),
            }));
        }
        this.#chat.changeAnswer(message, answer, () => ({ status: "done" }));
    }
}
