import { rowOf } from "../verb/space";
import type { Change } from "../verb/apps";
import { present } from "@destack/schema";

/** The milliseconds a beat of the task takes. */
export const BEAT = 250;

/** The characters you type a beat, about 64 a second, a quick typist's burst. */
export const CHARACTERS_PER_BEAT = 16;

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

/** The beats each left-over item holds before the next one shows, a second each. */
const LEFT_BEATS = 4;

/** The beats the finished task holds before it runs again. */
const HOLD_BEATS = 20;

/** One item of the chat: a question, call, reply, approval, app, left-overs or tally. */
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
      }
    | { kind: "stale"; title: string; copies: readonly Copy[] }
    | { kind: "tally"; text: string; isWrong: boolean };

/** How long an approval holds: for this one call, or for the rest of the chat. */
export type Grant = "once" | "chat";

/** A row of the agent's board: who asked, what they asked, and where it stands. */
export type Lead = readonly [name: string, asked: string, state: string];

/** Something left for you to do by hand: the app's mark, where it is, and what to do. */
export type Copy = readonly [server: string, place: string, value: string];

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
    /** Your message while you type it into the composer. */
    typing: string | undefined;
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

/** A task told a step at a time on a clock of beats, from your message to the tally. */
export class Scene {
    /** The steps, placed on the clock. */
    readonly cues: readonly Cue[];
    /** The beats one run takes, the hold included. */
    readonly length: number;

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

        // hold your message in the composer while you type it
        const typed = item.kind === "ask" ? typeBeatsOf(item.text) : 0;
        const typing = item.kind === "ask" && progress < typed ? item.text : undefined;
        const items = reached
            .map((candidate) => candidate.step.item)
            .filter((candidate) => !(candidate === item && typing !== undefined));

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

        return { step: cue.step, items, typing, running, deciding, pressed, building, isWorking };
    }
}

/** The row of Databases in fig. 1, where the stacked waitlist sits in Airtable. */
const TABLES_ROW = rowOf("tables");

/** The row of the waitlist in fig. 1, where your space keeps the signups. */
const WAITLIST_ROW = rowOf("builder");

/** The row of Chat in fig. 1, where people ask in #launch. */
const CHAT_ROW = rowOf("chat");

/** The row of Workflows in fig. 1, where your agent's workflow lands. */
const WORKFLOWS_ROW = rowOf("workflows");

/** The first ask: who is waiting and asked. */
const waiting: Item = {
    kind: "ask",
    text: "who from the launch post is still waiting, and did any of them ask in #launch?",
};

/** The second ask: a board for Jonas, with a rule. */
const board: Item = { kind: "ask", text: "make it a board for Jonas, but he can't email them" };

/** The third ask: keep it going, and tidy. */
const keep: Item = {
    kind: "ask",
    text: "add anyone new who asks, and clear out the ones we've answered",
};

/** The fourth ask, once the mistake shows. */
const oops: Item = {
    kind: "ask",
    text: "ah shit, not Maya, Jonas was halfway through. undo that",
};

/** The people waiting who asked in #launch, as one query finds them. */
const askers: readonly Lead[] = [
    ["Maya Chen", "“is there a team plan?”", "Replied · Jonas"],
    ["Sam Ortiz", "“API rate limits?”", "Answered"],
    ["Noor Haddad", "“student pricing?”", "Answered"],
    ["Leo Martin", "“EU hosting?”", "Open"],
    ["Priya Nair", "“does it work offline?”", "Open"],
    ["Ada Park", "“can I self-host?”", "Open"],
    ["Tom Becker", "“import from Notion?”", "Open"],
    ["Lena Okafor", "“SSO?”", "Open"],
    ["Ravi Shah", "“mobile app?”", "Open"],
];

/** The same people as the connectors match them by email, three missed under other addresses. */
const matched: readonly Lead[] = askers.slice(0, 6);

/** The task through Airtable and Slack: a partial match, no trigger, and a lost row. */
export const stacked = new Scene([
    { item: waiting, row: TABLES_ROW, change: "start" },
    {
        item: {
            kind: "call",
            server: "airtable",
            running: "Listing the waitlist",
            title: "Listed the waitlist",
            request: 'list_records({ table: "Signups", pageSize: 100 }) × 19',
            response: "1,840 signups · 312 from the launch post, waiting",
            isFailed: false,
            isExpanded: false,
        },
        row: TABLES_ROW,
        change: "start",
    },
    {
        item: {
            kind: "call",
            server: "slack",
            running: "Reading #launch",
            title: "Read #launch and its posters",
            request: 'conversations.history({ channel: "#launch" }) × 6\nusers.info({ user }) × 58',
            response: "58 posters · 6 match a waiting signup by email",
            isFailed: false,
            isExpanded: false,
        },
        row: CHAT_ROW,
        change: "start",
    },
    {
        item: {
            kind: "reply",
            text: "312 are waiting, and 6 of them asked in #launch. Some posters' Slack emails don't match any signup, so I may have missed a few.",
        },
        row: CHAT_ROW,
        change: "start",
    },
    { item: board, row: TABLES_ROW, change: "start" },
    {
        item: {
            kind: "app",
            title: "Launch askers",
            badge: "Artifact · Airtable, Slack",
            rows: matched,
            note: "Reads and writes as each viewer · Jonas connects his own",
            isLive: false,
        },
        row: TABLES_ROW,
        change: "start",
    },
    {
        item: {
            kind: "reply",
            text: "Here it is. Jonas needs his own Airtable and Slack connected to open it, and it acts with his access, so I can't stop him emailing them.",
        },
        row: TABLES_ROW,
        change: "start",
    },
    { item: keep, row: TABLES_ROW, change: "start" },
    {
        item: {
            kind: "approval",
            text: "wants to use Airtable: delete records.",
            outcome: "Allowed for this chat · Airtable",
            grant: "chat",
        },
        row: TABLES_ROW,
        change: "start",
    },
    {
        item: {
            kind: "call",
            server: "airtable",
            running: "Clearing the answered",
            title: "Cleared the answered",
            request: "delete_records({ ids }) · 3",
            response: "Maya Chen, Sam Ortiz, Noor Haddad",
            isFailed: false,
            isExpanded: false,
        },
        row: TABLES_ROW,
        change: "start",
    },
    {
        item: {
            kind: "reply",
            text: "Cleared 3. New askers I can't add on my own: nothing wakes me when someone posts in #launch.",
        },
        row: CHAT_ROW,
        change: "start",
    },
    { item: oops, row: TABLES_ROW, change: "start" },
    {
        item: {
            kind: "call",
            server: "airtable",
            running: "Bringing Maya back",
            title: "Brought Maya back",
            request: 'create_records({ fields: { Name: "Maya Chen", … } })',
            response: "new record · rec8Lq2",
            isFailed: false,
            isExpanded: false,
        },
        row: TABLES_ROW,
        change: "start",
    },
    {
        item: {
            kind: "reply",
            text: "Maya's back, but as a new row without Jonas's notes: Airtable's API can't restore deleted records.",
        },
        row: TABLES_ROW,
        change: "start",
    },
    {
        item: {
            kind: "stale",
            title: "Left for you",
            copies: [["airtable", "Maya's original row", "in the base's Trash for 7 days"]],
        },
        row: TABLES_ROW,
        change: "start",
    },
    {
        item: {
            kind: "tally",
            text: "84 calls · 6 of 9 found · no trigger · Jonas's notes lost",
            isWrong: true,
        },
        row: TABLES_ROW,
        change: "start",
    },
]);

/** The task in your space: one query, a board with a rule, a workflow, and a clean undo. */
export const destacked = new Scene([
    { item: waiting, row: WAITLIST_ROW, change: "start" },
    {
        item: {
            kind: "call",
            server: "destack",
            running: "Querying your space",
            title: "Queried your space",
            request:
                'client.query.signup.findMany({\n  where: { source: "launch post", state: "waiting" },\n  with: { person: { with: { message: { where: { channel: "launch" } } } } },\n})',
            response: "312 waiting · 9 asked in #launch · 18 ms",
            isFailed: false,
            isExpanded: false,
        },
        row: WAITLIST_ROW,
        change: "start",
    },
    {
        item: {
            kind: "reply",
            text: "312 are waiting, and 9 of them asked in #launch. Three asked from a different address than they signed up with.",
        },
        row: CHAT_ROW,
        change: "start",
    },
    { item: board, row: WAITLIST_ROW, change: "start" },
    {
        item: {
            kind: "app",
            title: "Launch askers",
            badge: "Live · your space",
            rows: askers,
            note: "Shared with Jonas · can work the board · can't send mail",
            isLive: true,
        },
        row: WAITLIST_ROW,
        change: "boarded",
    },
    { item: keep, row: WAITLIST_ROW, change: "boarded" },
    {
        item: {
            kind: "approval",
            text: "wants to add a workflow and edit the board.",
            outcome: "Allowed for this chat · workflow, board",
            grant: "chat",
        },
        row: WORKFLOWS_ROW,
        change: "boarded",
    },
    {
        item: {
            kind: "call",
            server: "destack",
            running: "Adding the workflow",
            title: "Added asked-to-board, cleared 3",
            request:
                'defineWorkflow({ name: "asked-to-board", triggers: { asked: { change: { object: message } } } })\nclient.mutate.lead.delete × 3',
            response: "runs on every new #launch message · run-7f2c",
            isFailed: false,
            isExpanded: false,
        },
        row: WORKFLOWS_ROW,
        change: "automated",
    },
    {
        item: {
            kind: "reply",
            text: "Cleared 3, and anyone new who asks lands on the board as they post.",
        },
        row: WORKFLOWS_ROW,
        change: "automated",
    },
    { item: oops, row: WAITLIST_ROW, change: "automated" },
    {
        item: {
            kind: "call",
            server: "destack",
            running: "Bringing Maya back",
            title: "Brought Maya back",
            request: 'client.revert({ run: "run-7f2c", about: "Maya Chen" })',
            response: "1 change undone · same row, Jonas's notes and thread",
            isFailed: false,
            isExpanded: false,
        },
        row: WAITLIST_ROW,
        change: "automated",
    },
    {
        item: { kind: "reply", text: "Maya's back, with Jonas's notes and the thread." },
        row: WAITLIST_ROW,
        change: "automated",
    },
    {
        item: {
            kind: "tally",
            text: "1 API · 9 of 9 · runs on its own · nothing lost",
            isWrong: false,
        },
        row: WAITLIST_ROW,
        change: "automated",
    },
]);

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
    else if (item.kind === "app") {
        return BUILD_BEATS + SHOWN_BEATS;
    }
    // the left-overs: read one by one
    else if (item.kind === "stale") {
        return item.copies.length * LEFT_BEATS;
    }

    // the tally
    return REST_BEATS;
}
