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
    text,
    type DatabaseConnection,
} from "@destack/db";

import { schema, type Version } from "@destack/schema";

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
        const steps = (entry.steps as StoredStep[]).map((step, position) =>
            step.result === undefined && value[position] !== undefined
                ? { ...step, result: value[position] }
                : step,
        );
        await this.client.database
            .update(undoEntry)
            .set({ steps: steps as never })
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
            steps.every((step) => this.client.method(`${step.object}.${step.name}`).method.inverse);
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
            steps: steps as never,
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
            const [entry] = await this.client.database
                .select()
                .from(undoEntry)
                .where(and(eq(undoEntry.origin, this.client.origin), eq(undoEntry.state, from)))
                .orderBy(from === "done" ? desc(undoEntry.sequence) : asc(undoEntry.sequence))
                .limit(1);
            if (entry === undefined) {
                return false;
            }

            // invert its steps in reverse order
            const calls: sync.Call[] = [];
            for (const stored of [...(entry.steps as StoredStep[])].reverse()) {
                const { object, name, method } = this.client.method(
                    `${stored.object}.${stored.name}`,
                );
                if (stored.release !== object.package.version) {
                    calls.length = 0;
                    break;
                }
                const id = stored.after?.id ?? stored.input.id;
                const current =
                    typeof id === "string"
                        ? await this.client.row(this.client.database, object, id)
                        : undefined;
                const step: Step = {
                    ...stored,
                    object,
                    name,
                    ...(current === undefined ? {} : { current }),
                };
                calls.push(...(method.inverse?.(step) ?? []));
            }
            if (calls.length === 0) {
                await this.client.database.delete(undoEntry).where(eq(undoEntry.id, entry.id));
                continue;
            }

            // submit the inverse as one mutation
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
                    await database
                        .update(undoEntry)
                        .set({ steps: steps as never, state: to, mutationId })
                        .where(eq(undoEntry.id, entry.id));

                    return { calls, result: undefined };
                },
            );

            return true;
        }
    }

    /** Drop the entries of mutations the server rejected. */
    async forget(transaction: DatabaseConnection, mutations: readonly string[]): Promise<void> {
        await transaction.delete(undoEntry).where(inArray(undoEntry.mutationId, [...mutations]));
    }
}
