import { expect, test, TestClock } from "@destack/test";
import { Clock } from "@destack/schema";
import { TEST_DIALECTS } from "@destack/db/test";
import type { Change, Row, Table } from "@destack/db";
import { View } from "../dataflow/view.ts";
import { first, note, open } from "../test/fixture.ts";
import type { Audience } from "./audience.ts";
import { Evaluation } from "./evaluation.ts";
import { Feed } from "./feed.ts";
import { ConditionAudience } from "./test/audience.ts";

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

    /** Admit the rows the current conditions admit. */
    admits(table: Table, rows: readonly Row[]) {
        return this.#current.admits(table, rows);
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
    readonly #clock: TestClock;
    /** The moment the audience sees done notes from. */
    readonly #moment: number;
    /** The conditions deciding before the moment. */
    readonly #before = new ConditionAudience(new Map([[note, { title: "Open" }]]));
    /** The conditions deciding from the moment on. */
    readonly #after = new ConditionAudience(new Map([[note, { title: "Done" }]]));
    /** The conditions decided last. */
    #current: ConditionAudience;

    /** Decide by a clock with a moment the decisions change at. */
    constructor(clock: TestClock, moment: number) {
        this.#clock = clock;
        this.#moment = moment;
        this.#current = this.#decided();
    }

    /** Match the visible rows of a table as of now. */
    where(table: Table) {
        return this.#current.where(table);
    }

    /** Admit the rows visible now. */
    admits(table: Table, rows: readonly Row[]) {
        return this.#current.admits(table, rows);
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

test.for(TEST_DIALECTS)(
    "send the rows an audience admits after deciding again at the same position on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note], { clock: Clock.system });
        await database.insert(note).values([
            { ...first, id: "a", title: "Open" },
            { ...first, id: "b", title: "Done" },
        ]);

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
        const page = await evaluation.redecide(position, true);
        expect(page.changes.map((change) => [change.operation, change.row["id"]])).toEqual([
            ["insert", "b"],
            ["delete", "a"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "read shared windows again once the decisions they hold expire, under one key, on %s",
    async (dialect) => {
        const database = await open(dialect);
        const clock = new TestClock(1000);
        const feed = new Feed(database, [note], { clock });
        await database.insert(note).values([
            { ...first, id: "a", title: "Open" },
            { ...first, id: "b", title: "Done" },
        ]);

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
        const page = await evaluation.redecide(position, true);
        expect(page.changes.map((change) => [change.operation, change.row["id"]])).toEqual([
            ["insert", "b"],
            ["delete", "a"],
        ]);
    },
);
