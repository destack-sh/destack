import { present } from "@destack/schema";

/** The milliseconds the trace draws across: the transaction and the runs it set off. */
export const TRACE_LENGTH = 1400;

/** The milliseconds between one triggered run and the next, as the queue takes them. */
const RUN_GAP = 135;

/** The moment the first triggered run starts, once the transaction commits. */
const FIRST_RUN = 44;

/** The span key of the transaction. */
export const ROOT_SPAN = 0;

/** The span key of Ines's duplicate calendar hold, the write your agent selects. */
export const INES_HOLD = 1003;

/** A view of the run: the duplicates as they happened, the replay on the branch, or undone. */
export type Lane = "original" | "replay" | "undone";

/** The app of your space a span ran in, which sets its colour. */
export type Service = "tables" | "workflows" | "tasks" | "chat" | "calendar";

/** What a span is: the transaction, its row update, a triggered run, or one of a run's writes. */
export type SpanKind = "transaction" | "update" | "run" | "write";

/** One event of the trace: when, whether it duplicated a write, and what happened. */
export type Event = readonly [at: number, isDuplicate: boolean, text: string];

/** How a span ended: fine, a duplicate write, or a duplicate undone. */
export type Status = "ok" | "duplicate" | "undone";

/** One span of the trace. */
export type Span = {
    /** The span's key. */
    id: number;
    /** The span it ran under. */
    parent: number | undefined;
    /** What the span is. */
    kind: SpanKind;
    /** What ran. */
    name: string;
    /** The app it ran in. */
    service: Service;
    /** When it started, in milliseconds after the transaction began. */
    start: number;
    /** When it ended. */
    end: number;
    /** How it ended. */
    status: Status;
    /** The asker it ran for. */
    asker: number | undefined;
    /** The line of the workflow it ran, or 0 for the transaction's spans. */
    line: number;
};

/** A person who asked for a demo, handled earlier in the week: name, company, task and demo. */
type Asker = readonly [name: string, company: string, task: string, demo: string];

/** The nine askers the workflow had already handled before Thursday. */
export const askers: readonly Asker[] = [
    ["Ines", "Orbit Labs", "DST-31", "Wed 11:00"],
    ["Jonas", "Kettle", "DST-32", "Thu 14:00"],
    ["Priya", "Nair Studio", "DST-33", "Thu 15:00"],
    ["Leo", "Brightpath", "DST-34", "Fri 10:00"],
    ["Noor", "Papertrail", "DST-35", "Fri 11:00"],
    ["Maya", "Northwind", "DST-36", "Fri 14:00"],
    ["Kai", "Quietly", "DST-37", "Mon 10:00"],
    ["Tara", "Fieldnotes", "DST-38", "Mon 11:00"],
    ["Omar", "Lind", "DST-39", "Mon 14:00"],
];

/** The workflow's code: line number and text. */
const code: readonly (readonly [number: number, text: string])[] = [
    [1, 'export const launchAskers = defineWorkflow({ name: "launch-askers" })'],
    [2, '    .on(signup.on("update"), (signup) => signup.asksForDemo)'],
    [3, "    .implement(async (step, signup) => {"],
    [4, '        await step.call("task", tasks.create(taskOf(signup)));'],
    [5, '        await step.call("post", chat.post("#launch", askedOf(signup)));'],
    [6, '        await step.call("hold", calendar.hold(demoOf(signup)));'],
];

/** The trigger once the fix runs the workflow for new signups only. */
const FIXED_TRIGGER = '    .on(signup.on("insert"), (signup) => signup.asksForDemo)';

/** The bulk edit in the Signups table that started it all. */
const BULK_EDIT = 'signup.where({ status: "waiting" }).update({ status: "approved" })';

/** The transaction that approved 400 signups and every run it set off, as a trace. */
export class Trace {
    /** The lane the trace shows. */
    readonly lane: Lane;
    /** Every span, parents before children. */
    readonly spans: readonly Span[];

    /** Build the trace of a lane. */
    constructor(lane: Lane) {
        this.lane = lane;
        this.spans = spansOf(lane);
    }

    /** Return a span by its key. */
    span(id: number): Span {
        return present(
            this.spans.find((candidate) => candidate.id === id),
            `trace span ${id}`,
        );
    }

    /** Return the triggered runs. */
    runs(): readonly Span[] {
        return this.spans.filter((span) => span.kind === "run");
    }

    /** Return the first span an app ran, or the transaction when the app ran nothing. */
    first(service: Service): Span {
        return this.spans.find((span) => span.service === service) ?? this.span(ROOT_SPAN);
    }

    /** Return the spans a waterfall lists: every span but the writes of closed runs. */
    rows(open: ReadonlySet<number>): readonly Span[] {
        return this.spans.filter(
            (span) => span.kind !== "write" || (span.parent !== undefined && open.has(span.parent)),
        );
    }

    /** Return the events of the trace in order: the approval, then each run and its writes. */
    events(): readonly Event[] {
        // tell what each span did, a duplicate unless it was undone
        const events = this.spans.map((span): Event => [
            span.start,
            span.status === "duplicate",
            eventOf(span, this.lane),
        ]);
        const last = this.spans.at(-1);

        // close with the runs' outcome, or say nothing ran on the branch
        return last?.kind === "write"
            ? [...events, [last.end, last.status === "duplicate", outcomeOf(this.lane)]]
            : [...events, [FIRST_RUN, false, "on insert · no new signups · no runs"]];
    }

    /** Return the line a span ran: its file, its text, and its value as a debugger shows it. */
    lineOf(span: Span): readonly [file: string, text: string, value: string] {
        // show the bulk edit for the transaction, and the workflow's line for the rest
        if (span.line === 0) {
            return [
                "Signups · bulk edit",
                BULK_EDIT,
                span.parent === undefined ? "Florian" : "400 rows",
            ];
        }
        const text = span.line === 2 && this.lane === "replay" ? FIXED_TRIGGER : textOf(span.line);

        return [`launch.ts:${span.line}`, text.trim(), valueOf(span)];
    }
}

/** Return the run a span belongs to: the run itself, or the run a write sits under. */
export function runOf(span: Span): number | undefined {
    return span.kind === "run" ? span.id : span.kind === "write" ? span.parent : undefined;
}

/** Return how deep a span sits: the transaction, then its update and runs, then the writes. */
export function depthOf(span: Span): number {
    return span.kind === "transaction" ? 0 : span.kind === "write" ? 2 : 1;
}

/** Return a span's duration as a trace view prints it. */
export function durationOf(span: Span): string {
    return `${Math.round(span.end - span.start)} ms`;
}

/** Return a moment as the wall clock read it, from 09:00:00. */
export function stampOf(time: number): string {
    const milliseconds = Math.floor(time);
    const seconds = Math.floor(milliseconds / 1000);

    return `09:00:0${seconds}.${String(milliseconds % 1000).padStart(3, "0")}`;
}

/** Return a span's value on its line: the asker and what the step wrote for them. */
function valueOf(span: Span): string {
    // read the asker the span ran for
    const asker = present(askers[present(span.asker, "span asker")], "asker");
    if (span.line === 2) {
        return `${asker[0]} · boarded on Monday as ${asker[2]}`;
    } else if (span.line === 4) {
        return `second ${asker[2]}`;
    } else if (span.line === 5) {
        return `second post · ${asker[0]}`;
    }

    return `second hold · ${asker[3]}`;
}

/** Return what a span did, as the event at its start tells it. */
function eventOf(span: Span, lane: Lane): string {
    // tell the transaction's spans, then the runs and writes for their asker
    if (span.kind === "transaction") {
        return lane === "replay"
            ? "replay of tx-91c on fix/askers-insert · nothing written"
            : "Florian approves 400 signups";
    } else if (span.kind === "update") {
        return "tx-91c updates 400 rows";
    }
    const [name, , task, demo] = present(askers[present(span.asker, "span asker")], "asker");
    if (span.kind === "run") {
        return `launch-askers runs for ${name} again`;
    } else if (span.line === 4) {
        return `${task} created again on the Launch board`;
    } else if (span.line === 5) {
        return `posted to #launch again · ${name}`;
    }

    return `${name} held again · ${demo}`;
}

/** Return the closing event of a lane whose runs ran. */
function outcomeOf(lane: Lane): string {
    return lane === "undone"
        ? "9 runs undone at 09:14 · 27 writes reverted"
        : "9 runs · 27 duplicate writes";
}

/** Return a line of the workflow's code. */
function textOf(line: number): string {
    return present(
        code.find(([number]) => number === line),
        `code line ${line}`,
    )[1];
}

/** Return a lane's spans: the transaction, then the nine runs unless the fix stops them. */
function spansOf(lane: Lane): readonly Span[] {
    // approve 400 signups in one transaction
    const transaction: Span[] = [
        {
            ...transactionSpan,
            id: ROOT_SPAN,
            parent: undefined,
            kind: "transaction",
            name: "tx-91c · approve 400",
            end: 40,
        },
        {
            ...transactionSpan,
            id: 1,
            parent: ROOT_SPAN,
            kind: "update",
            name: "update signups × 400",
            start: 2,
            end: 36,
        },
    ];

    // run the workflow again for every asker it had handled, unless the fix stops it
    const status: Status = lane === "undone" ? "undone" : "duplicate";
    const runs = askers.flatMap(([name, , task, demo], asker): Span[] => {
        // queue each run after the one before it, its writes under it
        const start = FIRST_RUN + asker * RUN_GAP;
        const run = 100 + asker;
        const shared = { parent: run, status, asker };
        const labels = [`tasks.create · ${task}`, "chat.post · #launch", `calendar.hold · ${demo}`];

        return [
            {
                id: run,
                parent: ROOT_SPAN,
                kind: "run",
                name: `launch-askers · ${name}`,
                service: "workflows",
                start,
                end: start + 118,
                status,
                asker,
                line: 2,
            },
            ...writes.map(([offset, service, from, to], index): Span => ({
                ...shared,
                kind: "write",
                id: 1000 + asker * 10 + offset,
                name: present(labels[index], "write label"),
                service,
                start: start + from,
                end: start + to,
                line: 3 + offset,
            })),
        ];
    });

    return lane === "replay" ? transaction : [...transaction, ...runs];
}

/** The fields the transaction's spans share. */
const transactionSpan = {
    service: "tables",
    start: 0,
    status: "ok",
    asker: undefined,
    line: 0,
} as const;

/** A run's three writes: their offset, app, and start and end after the run starts. */
const writes: readonly (readonly [offset: number, service: Service, from: number, to: number])[] = [
    [1, "tasks", 4, 36],
    [2, "chat", 40, 62],
    [3, "calendar", 66, 114],
];
