import {
    and,
    asc,
    boolean,
    defineTable,
    desc,
    eq,
    index,
    integer,
    inArray,
    json,
    lt,
    ne,
    TABLE,
    text,
    type Select,
    textEnum,
} from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";

import { schema, Version } from "@destack/schema";

import { RequestId, ObjectTypeReference } from "@destack/sync";

import * as sync from "@destack/sync";

import type { Step } from "../method/step.ts";

import type { ObjectClient } from "../device/client.ts";
import type { Submission } from "./access.ts";

/** How long an undo group stays open in ms, as ProseMirror's newGroupDelay. */
const UNDO_DELAY = 500;

/** The most undo groups a party stores, as ProseMirror's history depth. */
const UNDO_DEPTH = 100;

/** Each client's undoable mutations as steps, in undo groups. */
export const undoEntry = defineTable(
    "undo_entry",
    {
        /** The entry, named after the mutation that first applied its steps. */
        id: text("id").primaryKey(),
        /** The scope whose stacks hold the entry. */
        scope: text("scope").notNull(),
        /** The entry's place in the client's stacks. */
        sequence: integer("sequence").notNull(),
        /** The undo group, named after its first entry. */
        group: text("group").notNull(),
        /** Whether the entry is on the undo or the redo stack. */
        state: textEnum("state", ["done", "undone"]).notNull(),
        /** Whether the next mutation may join the entry's group. */
        isOpen: boolean("is_open").notNull(),
        /** The steps it last applied. */
        steps: json("steps", schema.json()).notNull(),
        /** The mutation that last applied them. */
        mutationId: text("mutation_id").notNull(),
        /** When the steps were last applied, in UTC epoch milliseconds. */
        time: integer("time").notNull(),
    },
    {
        constraints: (columns) => [
            index("undo_entry_sequence").on(columns.scope, columns.sequence),
            index("undo_entry_mutation").on(columns.mutationId),
        ],
    },
);

/** A stored step with its object type. */
export type StoredStep = Omit<Step, "object" | "current"> & {
    /** The package-qualified object type. */
    readonly object: ObjectTypeReference;
    /** The release of the object type's package the step was taken at. */
    readonly release: Version;
};

/** The steps of an entry, as their JSON reads back. */
const STORED_STEPS: schema.Schema<StoredStep[]> = schema.array(
    schema.looseObject({
        object: ObjectTypeReference,
        release: Version,
        name: schema.string(),
        input: schema.jsonObject(),
        result: schema.unknown().exactOptional(),
        before: schema.jsonObject().exactOptional(),
        after: schema.jsonObject().exactOptional(),
    }),
);

/** A client's undo and redo stacks of mutations, grouped while they continue each other. */
export class UndoStack {
    /** The client whose mutations the stacks record. */
    readonly client: ObjectClient;
    /** The latest toggle, which the next one waits for. */
    #toggling: Promise<unknown> = Promise.resolve();

    /** Create a client's undo and redo stacks. */
    constructor(client: ObjectClient) {
        this.client = client;
    }

    /** Undo the client's latest group, predicting false when none is left. */
    undo(): Submission<boolean> {
        return this.#toggle("done", "undone");
    }

    /** Redo the client's latest undone group, predicting false when none is left. */
    redo(): Submission<boolean> {
        return this.#toggle("undone", "done");
    }

    /** Close the open group, so the next mutation starts a new one. */
    async close(): Promise<void> {
        await this.client.database.transaction(async (transaction) => {
            await transaction
                .update(undoEntry)
                .set({ isOpen: false })
                .where(and(eq(undoEntry.scope, this.client.scope), eq(undoEntry.isOpen, true)));
        });
    }

    /** Read the client's latest entry in a state: the last done, or the first undone. */
    async latest(state: "done" | "undone"): Promise<Select<typeof undoEntry> | undefined> {
        const [entry] = await this.client.database
            .select()
            .from(undoEntry)
            .where(and(eq(undoEntry.scope, this.client.scope), eq(undoEntry.state, state)))
            .orderBy(state === "done" ? desc(undoEntry.sequence) : asc(undoEntry.sequence))
            .limit(1);

        return entry;
    }

    /** Yield whether the party can undo and redo, now and after each change, until aborted. */
    async *watch(
        signal: AbortSignal,
    ): AsyncGenerator<{ readonly canUndo: boolean; readonly canRedo: boolean }> {
        // yield the stacks now
        let last = await this.#availability();
        yield last;

        // yield them again after each commit that changes them
        const isSame = (next: typeof last) =>
            next.canUndo === last.canUndo && next.canRedo === last.canRedo;
        while (
            await this.client.database.log.waitUntil(
                async () => !isSame(await this.#availability()),
                signal,
            )
        ) {
            last = await this.#availability();
            yield last;
        }
    }

    /** Push an undoable mutation, returning composed steps when it composes into the latest. */
    async remember(
        database: DatabaseConnection,
        id: string,
        steps: readonly StoredStep[],
        composable: string | undefined,
    ): Promise<readonly StoredStep[] | undefined> {
        // clear the redo stack
        const scoped = eq(undoEntry.scope, this.client.scope);
        await database.delete(undoEntry).where(and(scoped, eq(undoEntry.state, "undone")));

        // close the latest entry
        const [latest] = await database
            .select()
            .from(undoEntry)
            .where(scoped)
            .orderBy(desc(undoEntry.sequence))
            .limit(1);
        if (latest?.isOpen === true) {
            await database
                .update(undoEntry)
                .set({ isOpen: false })
                .where(eq(undoEntry.id, latest.id));
        }

        // require an inverse for every call
        const isUndoable =
            steps.length > 0 &&
            steps.every(
                (step) =>
                    this.client.method({ type: step.object, method: step.name }).method.inverse !==
                    undefined,
            );
        if (!isUndoable) {
            return undefined;
        }

        // join the latest entry's group while open, within the delay, when every step composes into its steps
        const time = this.client.environment.clock.now();
        const composed =
            latest !== undefined && latest.isOpen && time - latest.time < UNDO_DELAY
                ? this.#composeStored(latest.steps, steps)
                : undefined;

        // compose into the latest entry while its mutation is the composable one
        if (latest !== undefined && composed !== undefined && latest.mutationId === composable) {
            await database
                .update(undoEntry)
                .set({ steps: schema.json().parse(composed), isOpen: true, time })
                .where(eq(undoEntry.id, latest.id));

            return composed;
        }

        // push a new entry, in the latest group when joining
        const isJoining = latest !== undefined && composed !== undefined;
        await database.insert(undoEntry).values({
            id,
            scope: this.client.scope,
            sequence: (latest?.sequence ?? 0) + 1,
            group: isJoining ? latest.group : id,
            state: "done",
            isOpen: true,
            steps: schema.json().parse(steps),
            mutationId: id,
            time,
        });

        // drop the groups past the depth after starting one
        if (!isJoining) {
            await this.#trim(database);
        }

        return undefined;
    }

    /** Fill an executed mutation's missing step results from the server. */
    static async settle(
        transaction: DatabaseConnection,
        mutationId: string,
        value: unknown,
    ): Promise<void> {
        // read the mutation's entry
        const [entry] = await transaction
            .select()
            .from(undoEntry)
            .where(eq(undoEntry.mutationId, mutationId));
        if (entry === undefined || !Array.isArray(value)) {
            return;
        }

        // fill missing results in call order
        const results: readonly unknown[] = value;
        const steps = STORED_STEPS.parse(entry.steps).map((step, position) =>
            step.result === undefined && results[position] !== undefined
                ? { ...step, result: results[position] }
                : step,
        );
        await transaction
            .update(undoEntry)
            .set({ steps: schema.json().parse(steps) })
            .where(eq(undoEntry.id, entry.id));
    }

    /** Drop the entries of mutations the server rejected. */
    static async forget(
        transaction: DatabaseConnection,
        mutations: readonly string[],
    ): Promise<void> {
        await transaction.delete(undoEntry).where(inArray(undoEntry.mutationId, [...mutations]));
    }

    /** Read whether the party has a group to undo and one to redo. */
    async #availability(): Promise<{ readonly canUndo: boolean; readonly canRedo: boolean }> {
        const [done, undone] = await Promise.all([this.latest("done"), this.latest("undone")]);

        return { canUndo: done !== undefined, canRedo: undone !== undefined };
    }

    /** Drop the client's groups past the depth. */
    async #trim(database: DatabaseConnection): Promise<void> {
        // find the oldest group the depth allows, by its first entry
        const [oldest] = await database
            .select({ sequence: undoEntry.sequence })
            .from(undoEntry)
            .where(and(eq(undoEntry.scope, this.client.scope), eq(undoEntry.id, undoEntry.group)))
            .orderBy(desc(undoEntry.sequence))
            .limit(1)
            .offset(UNDO_DEPTH - 1);

        // drop the entries before it
        if (oldest !== undefined) {
            await database
                .delete(undoEntry)
                .where(
                    and(
                        eq(undoEntry.scope, this.client.scope),
                        lt(undoEntry.sequence, oldest.sequence),
                    ),
                );
        }
    }

    /** Invert the client's latest group in a state as one new mutation. */
    #toggle(from: "done" | "undone", to: "done" | "undone"): Submission<boolean> {
        // chain after the previous toggle
        const mutationId = RequestId.create(this.client.environment);
        const predicted = this.#toggling.then(() => this.#invert(from, to, mutationId));
        // go on toggling after a failed one, and answer its failure in the returned prediction
        this.#toggling = predicted.catch(() => {});
        const confirmed = predicted.then(async (isApplied) => {
            if (isApplied) {
                await this.client.outcome(mutationId);
            }
        });

        // mark the confirmation handled, and answer its failure in the returned submission
        confirmed.catch(() => {});

        return { predicted, confirmed };
    }

    /** Submit the inverse of the latest group in a state, answering whether one existed. */
    async #invert(
        from: "done" | "undone",
        to: "done" | "undone",
        mutationId: string,
    ): Promise<boolean> {
        while (true) {
            // take the latest group
            const entries = await this.#group(from);
            const [first] = entries;
            if (first === undefined) {
                return false;
            }

            // invert its steps, dropping a group with nothing to invert
            const calls = await this.#inverseCalls(entries);
            if (calls.length === 0) {
                await this.client.database.transaction(async (transaction) => {
                    await transaction.delete(undoEntry).where(eq(undoEntry.group, first.group));
                });
                continue;
            }

            // submit the inverse as one mutation
            await this.#submitInverse(first, calls, to, mutationId);

            return true;
        }
    }

    /** Read the entries of the client's latest group in a state, in sequence order. */
    async #group(from: "done" | "undone"): Promise<Select<typeof undoEntry>[]> {
        const latest = await this.latest(from);
        if (latest === undefined) {
            return [];
        }

        return this.client.database
            .select()
            .from(undoEntry)
            .where(eq(undoEntry.group, latest.group))
            .orderBy(asc(undoEntry.sequence));
    }

    /** Invert a group's composed steps in reverse order, none once a type changed release. */
    async #inverseCalls(entries: readonly Select<typeof undoEntry>[]): Promise<sync.Call[]> {
        // compose the entries' steps, keeping apart steps that stay apart
        let composed: Step[] = [];
        for (const entry of entries) {
            const steps = this.#steps(entry.steps);
            if (steps === undefined) {
                return [];
            }
            composed = this.#compose(composed, steps) ?? [...composed, ...steps];
        }

        // invert each step against the object's current row
        const calls: sync.Call[] = [];
        for (const step of composed.toReversed()) {
            const id = step.after?.["id"] ?? step.input["id"];
            const row =
                typeof id === "string"
                    ? await this.client.row(this.client.database, step.object, id)
                    : undefined;
            const current = row && step.object.table[TABLE].encode(row);
            const method = step.object.method(step.name);
            calls.push(
                ...(method.inverse?.(current === undefined ? step : { ...step, current }) ?? []),
            );
        }

        return calls;
    }

    /** Compose stored steps pairwise into an entry's, absent when any pair stays apart. */
    #composeStored(previous: unknown, next: readonly StoredStep[]): StoredStep[] | undefined {
        // read both with their object types
        const [earlier, later] = [this.#steps(previous), this.#steps(next)];
        if (earlier === undefined || later === undefined) {
            return undefined;
        }

        // store the composed steps by their types' references and releases
        const composed = this.#compose(earlier, later);

        return composed?.map(({ object, current: _current, ...step }) => ({
            ...step,
            object: object.typeReference,
            release: object.package.version,
        }));
    }

    /** Compose steps pairwise into the steps before them, absent when any pair stays apart. */
    #compose(previous: readonly Step[], next: readonly Step[]): Step[] | undefined {
        // compose only steps of the same shape
        if (previous.length === 0 || previous.length !== next.length) {
            return undefined;
        }

        // compose each pair on one method of one object type
        const composed: Step[] = [];
        for (const [position, step] of next.entries()) {
            const before = previous[position];
            const joined =
                before !== undefined && before.object.same(step.object) && before.name === step.name
                    ? step.object.method(step.name).compose?.(before, step)
                    : undefined;
            if (joined === undefined) {
                return undefined;
            }
            composed.push(joined);
        }

        return composed;
    }

    /** Read stored steps with their object types, absent once a type changed release. */
    #steps(stored: unknown): Step[] | undefined {
        const steps: Step[] = [];
        for (const step of STORED_STEPS.parse(stored)) {
            // refuse a step recorded by another release
            const { object, name } = this.client.method({ type: step.object, method: step.name });
            if (step.release !== object.package.version) {
                return undefined;
            }
            steps.push({ ...step, object, name });
        }

        return steps;
    }

    /** Submit a group's inverse calls as one mutation, leaving it one entry in a state. */
    async #submitInverse(
        first: Select<typeof undoEntry>,
        calls: readonly sync.Call[],
        to: "done" | "undone",
        mutationId: string,
    ): Promise<void> {
        await this.client.prediction.add(
            this.client.database,
            mutationId,
            this.client.party,
            async (database) => {
                // predict each inverse call
                const steps: StoredStep[] = [];
                for (const call of calls) {
                    const { object, name } = this.client.method(call);
                    steps.push(
                        (await this.client.step(database, object, name, call.input, true)).step,
                    );
                }

                // fold the group into its first entry, holding the inverse steps in the new state
                await database
                    .delete(undoEntry)
                    .where(and(eq(undoEntry.group, first.group), ne(undoEntry.id, first.id)));
                await database
                    .update(undoEntry)
                    .set({
                        steps: schema.json().parse(steps),
                        state: to,
                        isOpen: false,
                        mutationId,
                        time: this.client.environment.clock.now(),
                    })
                    .where(eq(undoEntry.id, first.id));

                return { kind: "add", calls, result: undefined };
            },
        );
    }
}
