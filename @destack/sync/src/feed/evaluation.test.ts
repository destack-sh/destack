import { expect, test } from "@destack/test";
import { type Change, eq, type Row, type Table } from "@destack/db";
import { View } from "../dataflow/view.ts";
import { first, note, open, project, START } from "../test/fixture.ts";
import { type Audience, Watch } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import { Feed } from "./feed.ts";
import { ConditionAudience } from "./test/audience.ts";
import { SimulatedClock, Simulation } from "@destack/environment/simulation";

/** An audience deciding by some conditions, then by others once it decides again. */
class SwitchingAudience implements Audience {
    /** The tables whose changes decide visibility: none. */
    readonly watches = [];
    /** The conditions deciding now. */
    #current: ConditionAudience;
    /** The conditions deciding once the audience decides again. */
    readonly #next: ConditionAudience;

    /** Decide by some conditions, then by others. */
    constructor(current: ConditionAudience, next: ConditionAudience) {
        this.#current = current;
        this.#next = next;
    }

    /** The name of the conditions deciding now. */
    get key(): string {
        return this.#current.key;
    }

    /** Match the visible rows of a table by the current conditions. */
    where(table: Table) {
        return this.#current.where(table);
    }

    /** Show the rows the current conditions allow. */
    visible(table: Table, rows: readonly Row[]) {
        return this.#current.visible(table, rows);
    }

    /** List no concealed columns. */
    concealable(): readonly string[] {
        return [];
    }

    /** Conceal nothing. */
    async conceals(_table: Table, rows: readonly Row[]): Promise<readonly (readonly string[])[]> {
        return rows.map(() => []);
    }

    /** Decide the same at every time. */
    async until(): Promise<number | undefined> {
        return undefined;
    }

    /** Decide by the next conditions. */
    async refresh(): Promise<void> {
        this.#current = this.#next;
    }

    /** Decide nothing beyond rows themselves. */
    async decides(_change: Change): Promise<boolean> {
        return false;
    }
}

/** An audience seeing open notes until a moment and done notes once deciding after it. */
class TimedAudience implements Audience {
    /** The tables whose changes decide visibility: none. */
    readonly watches = [];
    /** The name of the audience. */
    readonly key = "timed";
    /** The clock telling the time. */
    readonly #clock: SimulatedClock;
    /** The moment the audience sees done notes from. */
    readonly #moment: number;
    /** The conditions deciding before the moment. */
    readonly #before = new ConditionAudience(new Map([[note, { title: "Open" }]]));
    /** The conditions deciding from the moment on. */
    readonly #after = new ConditionAudience(new Map([[note, { title: "Done" }]]));
    /** The conditions decided last. */
    #current: ConditionAudience;

    /** Decide by a clock with a moment the decisions change at. */
    constructor(clock: SimulatedClock, moment: number) {
        this.#clock = clock;
        this.#moment = moment;
        this.#current = this.#decided();
    }

    /** Match the visible rows of a table as of now. */
    where(table: Table) {
        return this.#current.where(table);
    }

    /** Show the rows visible now. */
    visible(table: Table, rows: readonly Row[]) {
        return this.#current.visible(table, rows);
    }

    /** List no concealed columns. */
    concealable(): readonly string[] {
        return [];
    }

    /** Conceal nothing. */
    async conceals(_table: Table, rows: readonly Row[]): Promise<readonly (readonly string[])[]> {
        return rows.map(() => []);
    }

    /** Read the moment the decisions change, absent once decided after it. */
    async until(): Promise<number | undefined> {
        return this.#current === this.#before ? this.#moment : undefined;
    }

    /** Decide as of now. */
    async refresh(): Promise<void> {
        this.#current = this.#decided();
    }

    /** Decide nothing beyond rows themselves. */
    async decides(_change: Change): Promise<boolean> {
        return false;
    }

    /** Read the conditions deciding as of now. */
    #decided(): ConditionAudience {
        return this.#clock.now() < this.#moment ? this.#before : this.#after;
    }
}

/** An audience showing every note, deciding again on each project change. */
class ProjectAudience extends ConditionAudience {
    /** The project changes, which decide visibility. */
    override readonly watches = [Watch.of(project, ["inbox"])];

    /** Decide again on a project change. */
    override async decides(change: Change): Promise<boolean> {
        return change.table === project;
    }
}

test("send a row a transaction changes beside a change of access, deciding every row again", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note, project]);
    await database.transaction(
        async (transaction) => await transaction.insert(note).values({ ...first, title: "Open" }),
    );

    // collect the note under an audience deciding again on project changes
    const evaluation = new Evaluation(
        feed,
        { notes: { table: note, scopes: ["inbox"] } },
        new ProjectAudience(new Map()),
    );
    const position = await database.log.position();
    await evaluation.collect(new View(database, position, feed));

    // change access and retitle the note in one transaction
    await database.transaction(async (transaction) => {
        await transaction.insert(project).values({ id: "p", scope: "inbox", name: "Launch" });
        await transaction.update(note).set({ title: "Done" }).where(eq(note.id, "a"));
    });
    const pages = [];
    for await (const page of evaluation.advance(position, true, new AbortController().signal)) {
        pages.push(page);
    }

    // send the retitled note with the decisions made again
    expect(
        pages.flatMap((page) =>
            page.changes.map((change) => [change.operation, change.row["id"], change.row["title"]]),
        ),
    ).toEqual([["update", "a", "Done"]]);
});

test("send the rows an audience shows after deciding again at the same position", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values([
                { ...first, id: "a", title: "Open" },
                { ...first, id: "b", title: "Done" },
            ]),
    );

    // collect the open note through the feed's shared reads
    const audience = new SwitchingAudience(
        new ConditionAudience(new Map([[note, { title: "Open" }]])),
        new ConditionAudience(new Map([[note, { title: "Done" }]])),
    );
    const evaluation = new Evaluation(
        feed,
        { notes: { table: note, scopes: ["inbox"] } },
        audience,
    );
    const position = await database.log.position();
    await evaluation.collect(new View(database, position, feed));

    // swap the open note for the done note once the audience decides again
    const page = await evaluation.redecide(position, true, []);
    expect(page.changes.map((change) => [change.operation, change.row["id"]])).toEqual([
        ["insert", "b"],
        ["delete", "a"],
    ]);
});

test("read shared windows again once their decisions expire, under one key", async () => {
    const simulation = new Simulation({ seed: 1, start: 1000 });
    const { clock } = simulation;
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values([
                { ...first, id: "a", title: "Open" },
                { ...first, id: "b", title: "Done" },
            ]),
    );

    // collect the open note through the feed's shared reads before the moment
    const evaluation = new Evaluation(
        feed,
        { notes: { table: note, scopes: ["inbox"] } },
        new TimedAudience(clock, 2000),
    );
    const position = await database.log.position();
    await evaluation.collect(new View(database, position, feed));

    // swap the open note for the done note once the moment passed
    clock.set(2000);
    const page = await evaluation.redecide(position, true, []);
    expect(page.changes.map((change) => [change.operation, change.row["id"]])).toEqual([
        ["insert", "b"],
        ["delete", "a"],
    ]);
});

test("wake every stream sharing an evaluation at its expiry, after another stream stopped waiting", async () => {
    const simulation = new Simulation({ seed: 1, start: 1000 });
    const { clock } = simulation;
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const evaluation = new Evaluation(
        feed,
        { notes: { table: note, scopes: ["inbox"] } },
        new TimedAudience(clock, 2000),
    );
    const { sequence } = await database.log.position();

    // wait from two streams, the first stopping halfway to the expiry
    const woken: string[] = [];
    const stopping = new AbortController();
    const waits = [
        evaluation.wait(sequence, stopping.signal).then(() => woken.push(`first ${clock.now()}`)),
        evaluation
            .wait(sequence, new AbortController().signal)
            .then(() => woken.push(`second ${clock.now()}`)),
    ];
    void clock.sleep(500).then(() => stopping.abort());
    await simulation.drive(Promise.all(waits));

    // wake the second stream at the expiry the first one no longer waits for
    expect(woken).toEqual(["first 1500", "second 2000"]);
});
