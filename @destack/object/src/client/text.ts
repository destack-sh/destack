import { Condition } from "@destack/db/query";
import { v7 } from "uuid";
import type { ObjectType } from "../object/index.ts";
import { Sequence, type Run, type TextChange } from "../sequence/index.ts";
import type { Edited } from "../text/chunk.ts";
import { chunk } from "../text/table.ts";
import type { ObjectClient, Submission } from "./client.ts";

/** One text field of an object, followed as its sequence and changed by visible offsets. */
export interface LiveText {
    /** Settles once the copy holds the text. */
    readonly ready: Promise<void>;
    /** Read the text's sequence, predictions included. */
    read(): Promise<Sequence>;
    /** Yield the sequence, then again after each change. */
    watch(signal: AbortSignal): AsyncGenerator<Sequence>;
    /** Replace the text between two offsets, after every earlier change, as one predicted edit. */
    change(change: TextChange): Promise<Submission<Edited>>;
    /** Stop following the text. */
    close(): Promise<void>;
}

/** Follow text fields on a client. */
export const LiveText = {
    /** Follow one text field of an object as its sequence, changing it by visible offsets. */
    open(client: ObjectClient, object: ObjectType, id: string, field: string): LiveText {
        // follow the field's chunks in position order
        const chunks = client.objects.find((held) => held.table === chunk);
        if (chunks === undefined || !object.text.includes(field)) {
            throw new TypeError(`object ${object.name} holds no text field ${field}`);
        }
        const query = client.subscribe(chunks, {
            where: Condition.all(
                Condition.eq("parentType", object.name),
                Condition.eq("parentId", id),
                Condition.eq("field", field),
            ),
            order: [{ column: "position", direction: "asc" }],
        });
        const sequence = (rows: readonly Readonly<Record<string, unknown>>[]) =>
            new Sequence(rows.flatMap((row) => row.runs as readonly Run[]));

        // apply changes in order against the text of the previous change
        let previous: Promise<unknown> = Promise.resolve();
        const change = (replaced: TextChange) => {
            const next = previous.then(async () => {
                // translate the offsets against the current text, edit, and wait for the prediction
                const edits = sequence(await query.read()).change(replaced, v7());
                const mutator = client.mutate(object) as unknown as Readonly<
                    Record<string, (input: object) => Submission<Edited>>
                >;
                const editing = mutator.edit!({ id, field, edits });
                await editing.predicted;

                return editing;
            });
            previous = next.catch(() => {});

            return next;
        };

        return {
            ready: query.ready,
            read: async () => sequence(await query.read()),
            watch: async function* (signal) {
                for await (const rows of query.watch(signal)) {
                    yield sequence(rows);
                }
            },
            change,
            close: () => query.close(),
        };
    },
};
