import {
    and,
    asc,
    defineTable,
    desc,
    eq,
    integer,
    inArray,
    json,
    max,
    TABLE,
    text,
    type DatabaseConnection,
    type Select,
} from "@destack/db";

import { schema, Version } from "@destack/schema";

import { RequestId } from "@destack/service/request";

import * as sync from "@destack/sync";

import type { Step } from "../method/step.ts";

import type { ObjectClient, Submission } from "./client.ts";

/** Each party's undoable mutations as steps. */
export const undoEntry = defineTable("undo_entry", {
    /** The entry, named after the mutation that first applied its steps. */
    id: text("id").primaryKey(),
    /** The party the entry belongs to. */
    origin: text("origin").notNull(),
    /** The entry's place in its party's stack. */
    sequence: integer("sequence").notNull(),
    /** Whether the entry is on the undo or the redo stack. */
    state: text("state", { enum: ["done", "undone"] }).notNull(),
    /** The steps it last applied. */
    steps: json("steps", schema.json()).notNull(),
    /** The mutation that last applied them. */
    mutationId: text("mutation_id").notNull(),
});

/** A stored step with its object type. */
export type StoredStep = Omit<Step, "object" | "current"> & {
    /** The object type's name. */
    readonly object: string;
    /** The release of the object type's package the step was taken at. */
    readonly release: Version;
};

/** The steps an entry keeps, as their JSON reads back. */
const STORED_STEPS: schema.Schema<StoredStep[]> = schema.array(
    schema.looseObject({
        object: schema.string(),
        release: Version,
        name: schema.string(),
        input: schema.record(schema.string(), schema.json()),
        result: schema.unknown().exactOptional(),
        before: schema.record(schema.string(), schema.json()).exactOptional(),
        after: schema.record(schema.string(), schema.json()).exactOptional(),
    }),
);

/** A party's undo and redo stacks of mutations, as steps their inverses undo. */
export class UndoStack {
    /** The client whose mutations the stacks keep. */
    readonly client: ObjectClient;
    /** The latest toggle, which the next one waits for. */
    #toggling: Promise<unknown> = Promise.resolve();

    /** Keep a client's undo and redo stacks. */
    constructor(client: ObjectClient) {
        this.client = client;
    }

    /** Fill an executed mutation's missing step results from the server. */
    async settle(mutationId: string, value: unknown): Promise<void> {
        // read the mutation's entry
        const [entry] = await this.client.database
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
        await this.client.database
            .update(undoEntry)
            .set({ steps: schema.json().parse(steps) })
            .where(eq(undoEntry.id, entry.id));
    }

    /** Push an undoable mutation onto its party's undo stack and clear the redo stack. */
    async remember(
        database: DatabaseConnection,
        id: string,
        steps: readonly StoredStep[],
    ): Promise<void> {
        // clear the redo stack
        const ownEntries = eq(undoEntry.origin, this.client.origin);
        await database.delete(undoEntry).where(and(ownEntries, eq(undoEntry.state, "undone")));

        // require an inverse for every call
        const isUndoable =
            steps.length > 0 &&
            steps.every(
                (step) =>
                    this.client.method(`${step.object}.${step.name}`).method.inverse !== undefined,
            );
        if (!isUndoable) {
            return;
        }
        const [last] = await database
            .select({ sequence: max(undoEntry.sequence) })
            .from(undoEntry)
            .where(ownEntries);
        await database.insert(undoEntry).values({
            id,
            origin: this.client.origin,
            sequence: (last?.sequence ?? 0) + 1,
            state: "done",
            steps: schema.json().parse(steps),
            mutationId: id,
        });
    }

    /** Invert the party's latest done or undone entry as one new mutation. */
    toggle(from: "done" | "undone", to: "done" | "undone"): Submission<boolean> {
        // chain after the previous toggle
        const mutationId = RequestId.create();
        const predicted = this.#toggling.then(() => this.#invert(from, to, mutationId));
        this.#toggling = predicted.catch(() => {});
        const confirmed = predicted.then(async (isApplied) => {
            if (isApplied) {
                await this.client.outcome(mutationId);
            }
        });
        confirmed.catch(() => {});

        return { predicted, confirmed };
    }

    /** Submit the inverse of the party's latest entry in a state, returning whether one existed. */
    async #invert(
        from: "done" | "undone",
        to: "done" | "undone",
        mutationId: string,
    ): Promise<boolean> {
        while (true) {
            // take the latest entry
            const entry = await this.#latest(from);
            if (entry === undefined) {
                return false;
            }

            // invert its steps, dropping an entry with nothing to invert
            const calls = await this.#inverseCalls(entry);
            if (calls.length === 0) {
                await this.client.database.delete(undoEntry).where(eq(undoEntry.id, entry.id));
                continue;
            }

            // submit the inverse as one mutation
            await this.#submitInverse(entry, calls, to, mutationId);

            return true;
        }
    }

    /** Read the party's latest entry in a state: the last done, or the first undone. */
    async #latest(from: "done" | "undone"): Promise<Select<typeof undoEntry> | undefined> {
        const [entry] = await this.client.database
            .select()
            .from(undoEntry)
            .where(and(eq(undoEntry.origin, this.client.origin), eq(undoEntry.state, from)))
            .orderBy(from === "done" ? desc(undoEntry.sequence) : asc(undoEntry.sequence))
            .limit(1);

        return entry;
    }

    /** Invert an entry's steps in reverse order, none once a step's type changed release. */
    async #inverseCalls(entry: Select<typeof undoEntry>): Promise<sync.Call[]> {
        const calls: sync.Call[] = [];
        for (const stored of STORED_STEPS.parse(entry.steps).toReversed()) {
            // refuse a step recorded by another release
            const { object, name, method } = this.client.method(`${stored.object}.${stored.name}`);
            if (stored.release !== object.package.version) {
                return [];
            }

            // invert the step against the object's current row
            const id = stored.after?.["id"] ?? stored.input["id"];
            const row =
                typeof id === "string"
                    ? await this.client.row(this.client.database, object, id)
                    : undefined;
            const current = row && object.table[TABLE].encode(row);
            const step: Step = {
                ...stored,
                object,
                name,
                ...(current === undefined ? {} : { current }),
            };
            calls.push(...(method.inverse?.(step) ?? []));
        }

        return calls;
    }

    /** Submit an entry's inverse calls as one mutation, moving the entry to a state. */
    async #submitInverse(
        entry: Select<typeof undoEntry>,
        calls: readonly sync.Call[],
        to: "done" | "undone",
        mutationId: string,
    ): Promise<void> {
        await this.client.prediction.add(
            this.client.database,
            mutationId,
            this.client.origin,
            async (database) => {
                // predict each inverse call
                const steps: StoredStep[] = [];
                for (const call of calls) {
                    const { object, name } = this.client.method(call.method);
                    steps.push(
                        (await this.client.step(database, object, name, call.input, true)).step,
                    );
                }

                // record the inverse steps under the entry's new state
                await database
                    .update(undoEntry)
                    .set({ steps: schema.json().parse(steps), state: to, mutationId })
                    .where(eq(undoEntry.id, entry.id));

                return { calls, result: undefined };
            },
        );
    }

    /** Drop the entries of mutations the server rejected. */
    async forget(transaction: DatabaseConnection, mutations: readonly string[]): Promise<void> {
        await transaction.delete(undoEntry).where(inArray(undoEntry.mutationId, [...mutations]));
    }
}
