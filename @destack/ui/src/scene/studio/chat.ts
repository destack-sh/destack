import type { ObjectReference } from "@destack/sync";
import { type Accessor, createSignal, type Setter } from "@destack/view";
import { messageType, STUDIO } from "./object.example.ts";

/** The person at the keys. */
export const ME = "ada";

/** The member who answers by itself. */
export const ASSISTANT = "assistant";

/** The longest name a chat takes from its first words. */
const NAME = 40;

/** The longest break between messages of one person that stand together, in milliseconds. */
const TOGETHER = 5 * 60_000;

/** Something an answer uses. */
export interface Source {
    /** The address. */
    readonly address: string;
    /** The title. */
    readonly title: string;
    /** Who published it. */
    readonly site: string;
    /** What it says. */
    readonly excerpt: string;
}

/** Something the assistant did for an answer. */
export interface Step {
    /** The id. */
    readonly id: string;
    /** What it is. */
    readonly kind: "thought" | "search" | "read" | "ask";
    /** What it did or does. */
    readonly title: string;
    /** What came of it. */
    readonly text: string;
    /** Where it stands: waiting for a person's word, refused, under way or done. */
    readonly status: "running" | "waiting" | "done" | "denied";
}

/** One answer of the assistant to a question. */
export interface Answer {
    /** The id. */
    readonly id: string;
    /** What the assistant did for it, in order. */
    readonly steps: readonly Step[];
    /** The Markdown so far. */
    readonly text: string;
    /** What it uses. */
    readonly sources: readonly Source[];
    /** Where it stands. */
    readonly status: "working" | "writing" | "done" | "stopped";
    /** When it began, in milliseconds since the epoch. */
    readonly began: number;
    /** When it began to write, undefined before. */
    readonly wrote: number | undefined;
}

/** Something a member said. */
export interface ChatMessage {
    /** The id. */
    readonly id: string;
    /** Who said it. */
    readonly author: string;
    /** When, in milliseconds since the epoch. */
    readonly at: number;
    /** The Markdown, none for the assistant's, which its answers have. */
    readonly text: string;
    /** The message it is under, undefined for one of the conversation itself. */
    readonly thread: string | undefined;
    /** The message it answers, undefined for none. */
    readonly quote: string | undefined;
    /** The words of that message it answers, undefined for all of them. */
    readonly excerpt: string | undefined;
    /** Who reacted to it, by emoji. */
    readonly reactions: Readonly<Record<string, readonly string[]>>;
    /** Whether it was changed. */
    readonly isEdited: boolean;
    /** Whether it is pinned. */
    readonly isPinned: boolean;
    /** The assistant's answers, the one shown by its place. */
    readonly answers: readonly Answer[];
    /** The place of the answer that shows. */
    readonly shown: number;
}

/** A conversation of members. */
export interface Conversation {
    /** The id. */
    readonly id: string;
    /** The name. */
    readonly name: string;
    /** What it is: a channel, with one person, or with the assistant alone. */
    readonly kind: "channel" | "person" | "assistant";
    /** What it is about. */
    readonly topic: string;
    /** When the person read it last, in milliseconds since the epoch. */
    readonly read: number;
    /** What the person wrote and did not send. */
    readonly draft: string;
    /** The messages, the oldest first. */
    readonly messages: readonly ChatMessage[];
}

/** What stands in a conversation: a day, where the unread begin, or a message. */
export type Row =
    | { readonly kind: "day"; readonly key: string; readonly at: number }
    | { readonly kind: "unread"; readonly key: string }
    | {
          readonly kind: "message";
          readonly key: string;
          readonly message: ChatMessage;
          /** Whether it stands under another of the same person. */
          readonly isFollowing: boolean;
      };

/** Reference a message, as actions on messages take it. */
export function referenceOfMessage(id: string): ObjectReference {
    return messageType.reference(STUDIO, id);
}

/** Write a message as it begins. */
export function messageOf(
    message: Pick<ChatMessage, "id" | "author" | "at" | "text"> & Partial<ChatMessage>,
): ChatMessage {
    return {
        thread: undefined,
        quote: undefined,
        excerpt: undefined,
        reactions: {},
        isEdited: false,
        isPinned: false,
        answers: [],
        shown: 0,
        ...message,
    };
}

/** List the rows of messages, with each day, and where those unread since a time begin. */
export function rowsOf(messages: readonly ChatMessage[], read: number): readonly Row[] {
    // go through them in order
    const rows: Row[] = [];
    let before: ChatMessage | undefined;
    for (const message of messages) {
        // start a day, and what the person has not read
        const day = new Date(message.at).toDateString();
        const isDay = before === undefined || new Date(before.at).toDateString() !== day;
        if (isDay) {
            rows.push({ kind: "day", key: day, at: message.at });
        }
        const isUnread = message.at > read && message.author !== ME;
        const isFirst = isUnread && (before === undefined || before.at <= read);
        if (isFirst && before !== undefined) {
            rows.push({ kind: "unread", key: "unread" });
        }

        // put a message under the one before it by the same person
        const isFollowing =
            !isDay &&
            !isFirst &&
            before?.author === message.author &&
            message.at - before.at < TOGETHER &&
            message.quote === undefined;
        rows.push({ kind: "message", key: message.id, message, isFollowing });
        before = message;
    }

    return rows;
}

/** The studio's chat: its conversations, as a server would have them. */
export class Chat {
    /** The conversations. */
    readonly all: Accessor<readonly Conversation[]>;
    /** Replace the conversations. */
    readonly #setAll: Setter<readonly Conversation[]>;
    /** Read the time now, in milliseconds since the epoch. */
    readonly #now: () => number;
    /** The number of the next id. */
    #next = 0;

    /** Open a chat with its conversations. */
    constructor(chat: readonly Conversation[], now: () => number) {
        // follow them
        const [all, setAll] = createSignal(chat, { ownedWrite: true });
        this.all = all;
        this.#setAll = setAll;
        this.#now = now;
    }

    /** Read a conversation, undefined for an id of none. */
    find(id: string): Conversation | undefined {
        return this.all().find((each) => each.id === id);
    }

    /** Read the messages of a conversation itself, those in threads left out. */
    main(id: string): readonly ChatMessage[] {
        return (this.find(id)?.messages ?? []).filter(({ thread }) => thread === undefined);
    }

    /** Read a message and those under it. */
    thread(id: string, root: string): readonly ChatMessage[] {
        return (this.find(id)?.messages ?? []).filter(
            (message) => message.id === root || message.thread === root,
        );
    }

    /** Find a message, undefined for an id of none. */
    message(id: string): ChatMessage | undefined {
        return this.all()
            .flatMap(({ messages }) => messages)
            .find((message) => message.id === id);
    }

    /** Count the messages of a conversation the person has not read. */
    unread(id: string): number {
        const read = this.find(id)?.read ?? 0;

        return this.main(id).filter(({ at, author }) => at > read && author !== ME).length;
    }

    /** Open a conversation with the assistant, and return its id. */
    create(): string {
        // add it after the others, named until its first words name it
        const id = this.#id();
        const conversation: Conversation = {
            id,
            name: "New chat",
            kind: "assistant",
            topic: "",
            read: this.#now(),
            draft: "",
            messages: [],
        };
        this.#setAll((all) => [...all, conversation]);

        return id;
    }

    /** Change a conversation. */
    update(id: string, patch: Partial<Conversation>): void {
        this.#setAll((all) => all.map((each) => (each.id === id ? { ...each, ...patch } : each)));
    }

    /** Read a conversation to its end. */
    read(id: string): void {
        this.update(id, { read: this.#now() });
    }

    /** Read a conversation up to before a message. */
    unreadFrom(id: string, message: string): void {
        this.update(id, { read: (this.message(message)?.at ?? this.#now()) - 1 });
    }

    /** Say something, and return the message's id. */
    say(id: string, message: Pick<ChatMessage, "author" | "text"> & Partial<ChatMessage>): string {
        // add it, name a new chat by its first words, and read up to it
        const at = this.#now();
        const said = messageOf({ id: this.#id(), at, ...message });
        const isMine = message.author === ME;
        this.#setAll((all) =>
            all.map((each) =>
                each.id === id
                    ? {
                          ...each,
                          messages: [...each.messages, said],
                          ...(each.kind === "assistant" && each.messages.length === 0
                              ? { name: message.text.slice(0, NAME) }
                              : {}),
                          ...(isMine ? { read: at } : {}),
                      }
                    : each,
            ),
        );

        return said.id;
    }

    /** Change a message. */
    change(id: string, patch: (message: ChatMessage) => Partial<ChatMessage>): void {
        this.#setAll((all) =>
            all.map((each) =>
                each.messages.some((message) => message.id === id)
                    ? {
                          ...each,
                          messages: each.messages.map((message) =>
                              message.id === id ? { ...message, ...patch(message) } : message,
                          ),
                      }
                    : each,
            ),
        );
    }

    /** Change an answer of a message. */
    changeAnswer(id: string, answer: string, patch: (answer: Answer) => Partial<Answer>): void {
        this.change(id, ({ answers }) => ({
            answers: answers.map((each) =>
                each.id === answer ? { ...each, ...patch(each) } : each,
            ),
        }));
    }

    /** React to a message, or take the reaction back. */
    react(id: string, emoji: string, person = ME): void {
        this.change(id, ({ reactions }) => {
            // add the person, or take them off
            const before = reactions[emoji] ?? [];
            const people = before.includes(person)
                ? before.filter((each) => each !== person)
                : [...before, person];
            const others = Object.entries(reactions).filter(([each]) => each !== emoji);

            return {
                reactions: Object.fromEntries(
                    people.length === 0 ? others : [...others, [emoji, people]],
                ),
            };
        });
    }

    /** Delete a message and those under it. */
    remove(id: string): void {
        this.#setAll((all) =>
            all.map((each) => ({
                ...each,
                messages: each.messages.filter(
                    (message) => message.id !== id && message.thread !== id,
                ),
            })),
        );
    }

    /** Make up the next id. */
    #id(): string {
        this.#next += 1;

        return `new-${String(this.#next)}`;
    }
}
