import type { Change } from "../verb/apps";
import { present } from "@destack/schema";

/** The milliseconds a beat of the task takes. */
export const BEAT = 250;

/** The characters you type a beat, about 32 a second, quick but slow enough to see each land. */
export const CHARACTERS_PER_BEAT = 8;

/** The words the assistant streams a beat, about 16 a second as the apps stream. */
export const WORDS_PER_BEAT = 4;

/** The beats the assistant thinks between your message landing and its first call. */
const THINK_BEATS = 3;

/** The beats a call runs before it lands. */
const RUN_BEATS = 4;

/** The beats between one landed item and the next. */
const REST_BEATS = 1;

/** The beats you read an approval before you press it. */
const DECIDE_BEATS = 6;

/** The beats an app shows before its note lands under it. */
const BUILD_BEATS = 6;

/** The beats an app holds once its note shows. */
const SHOWN_BEATS = 6;

/** The beats the finished task holds before it runs again. */
const HOLD_BEATS = 20;

/** One item of the chat: a question, call, reply, approval, or app. */
export type Item =
    | { kind: "ask"; text: string }
    | {
          kind: "call";
          server: string;
          running: string;
          title: string;
          request: string;
          response: string;
          isFailed: boolean;
          isExpanded: boolean;
      }
    | { kind: "reply"; text: string }
    | { kind: "approval"; text: string; outcome: string; grant: Grant }
    | {
          kind: "app";
          title: string;
          badge: string;
          rows: readonly Lead[];
          note: string;
          isLive: boolean;
      };

/** How long an approval holds: for this one call, or for the rest of the chat. */
export type Grant = "once" | "chat";

/** A row of the agent's board: who asked, what they asked, and where it stands. */
export type Lead = readonly [name: string, asked: string, state: string];

/** One step of the task: what the chat adds, the page it touches, and how far your apps got. */
export type Step = { item: Item; row: number; change: Change };

/** One step placed on the task's clock: the beat it starts at and the beats it takes. */
type Cue = { step: Step; start: number; beats: number };

/** What the chat shows at one beat of the task. */
export type Moment = {
    /** The step the task has reached. */
    step: Step;
    /** The items landed in the conversation, in order. */
    items: readonly Item[];
    /** Your message as typed so far, while you type it into the composer. */
    typing: string | undefined;
    /** The reply under way and how many of its words have streamed. */
    streaming: { item: Item; words: number } | undefined;
    /** The call running now. */
    running: Item | undefined;
    /** The approval you are reading now. */
    deciding: Item | undefined;
    /** The approval you are about to press. */
    pressed: Item | undefined;
    /** The app still building, its note not yet shown. */
    building: Item | undefined;
    /** Whether the assistant is working, which turns send into stop. */
    isWorking: boolean;
};

/** A task told a step at a time on a clock of beats, from your message to its last reply. */
export class Scene {
    /** The steps, placed on the clock. */
    readonly cues: readonly Cue[];
    /** The beats one run takes, the hold included. */
    readonly length: number;

    /** Return a conversation of items, each its own step on the first row. */
    static of(items: readonly Item[]): Scene {
        return new Scene(items.map((item) => ({ item, row: 0, change: "start" })));
    }

    /** Place each step after the one before it. */
    constructor(steps: readonly Step[]) {
        // start each step where the one before it ends
        let start = 0;
        this.cues = steps.map((step) => {
            const cue = { step, start, beats: beatsOf(step.item) };
            start += cue.beats;

            return cue;
        });
        this.length = start + HOLD_BEATS;
    }

    /** Return what the chat shows some beats into a run, or the finished task without a beat. */
    at(beat: number | undefined): Moment {
        // find the steps reached and the one under way
        const at = beat ?? this.length;
        const reached = this.cues.filter((cue) => cue.start <= at);
        const cue = present(reached.at(-1), "reached cue");
        const progress = at - cue.start;
        const item = cue.step.item;

        // type your message into the composer a beat's characters at a time, then send it
        const typed = item.kind === "ask" ? typeBeatsOf(item.text) : 0;
        const typing =
            item.kind === "ask" && progress < typed
                ? item.text.slice(0, (progress + 1) * CHARACTERS_PER_BEAT)
                : undefined;
        const items = reached
            .map((candidate) => candidate.step.item)
            .filter((candidate) => !(candidate === item && typing !== undefined));

        // stream the reply under way a beat's words at a time
        const words = item.kind === "reply" ? item.text.split(" ").length : 0;
        const streaming =
            item.kind === "reply" && (progress + 1) * WORDS_PER_BEAT < words
                ? { item, words: (progress + 1) * WORDS_PER_BEAT }
                : undefined;

        // run the call under way, read then press the approval, and build the app
        const running = item.kind === "call" && progress < RUN_BEATS ? item : undefined;
        const deciding = item.kind === "approval" && progress < DECIDE_BEATS ? item : undefined;
        const pressed = deciding !== undefined && progress >= DECIDE_BEATS - 2 ? item : undefined;
        const building = item.kind === "app" && progress < BUILD_BEATS ? item : undefined;

        // work from your message landing until the reply has streamed
        const isUnderWay = progress < cue.beats;
        const isWorking =
            isUnderWay &&
            typing === undefined &&
            (item.kind === "ask" ||
                item.kind === "call" ||
                item.kind === "reply" ||
                building !== undefined);

        return {
            step: cue.step,
            items,
            typing,
            streaming,
            running,
            deciding,
            pressed,
            building,
            isWorking,
        };
    }
}

/** Return the beats you take to type a message into the composer and send it. */
function typeBeatsOf(text: string): number {
    return Math.ceil(text.length / CHARACTERS_PER_BEAT) + 1;
}

/** Return the beats an item takes to land before the next one starts. */
function beatsOf(item: Item): number {
    // your message: typed, sent, then thought about
    if (item.kind === "ask") {
        return typeBeatsOf(item.text) + THINK_BEATS;
    }
    // a call: run, then land
    else if (item.kind === "call") {
        return RUN_BEATS + REST_BEATS;
    }
    // the reply: streamed word by word
    else if (item.kind === "reply") {
        return Math.ceil(item.text.split(" ").length / WORDS_PER_BEAT) + REST_BEATS;
    }
    // an approval: read and pressed, then its outcome
    else if (item.kind === "approval") {
        return DECIDE_BEATS + REST_BEATS;
    }

    // an app: built, then its note
    return BUILD_BEATS + SHOWN_BEATS;
}
