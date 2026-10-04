import { v7 } from "uuid";
import type { ObjectOf, ObjectType } from "../object/index.ts";
import { Sequence, type Run, type TextChange } from "../sequence/index.ts";
import { Undo } from "../text/chunk.ts";
import { chunk } from "../text/table.ts";
import type { ObjectClient, Submission } from "./client.ts";

/** One text field of an object, followed as its sequence and changed by visible offsets. */
export interface LiveText {
    /** Settles once the copy has the text. */
    readonly ready: Promise<void>;
    /** Read the text's sequence, predictions included. */
    read(): Promise<Sequence>;
    /** Yield the sequence, then again after each change. */
    watch(signal: AbortSignal): AsyncGenerator<Sequence>;
    /** Replace the text between two offsets, after every earlier change, as one predicted edit. */
    change(change: TextChange): Promise<Submission<Undo>>;
    /** Stop following the text. */
    close(): Promise<void>;
}

/** Follow text fields on a client. */
export const LiveText = {
    /** Follow one text field of an object as its sequence, changing it by visible offsets. */
    open(client: ObjectClient, object: ObjectType, id: string, field: string): LiveText {
        // follow the field's chunks in position order
        const chunks = client.objects.find(isChunkType);
        if (chunks === undefined || !object.text.includes(field)) {
            throw new TypeError(`object ${object.name} has no text field ${field}`);
        }
        const query = client
            .queryOf(chunks)
            .findMany({
                where: { parentType: object.name, parentId: id, field: field },
                orderBy: { position: "asc" },
            })
            .subscribe();

        // apply changes in order against the text of the previous change
        let previous: Promise<unknown> = Promise.resolve();
        const change = (replaced: TextChange) => {
            const next = previous.then(async () => {
                // translate the offsets against the current text, edit, and wait for the prediction
                const edits = sequence(await query.read()).change(replaced, v7());
                const editing = client.call(object, "edit", { id, field, edits });
                const predicted = editing.predicted.then((value) => Undo.parse(value));
                await predicted;

                return { predicted, confirmed: editing.confirmed };
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

/** Report whether an object type keeps its rows in the chunk table, which guards no field. */
function isChunkType(type: ObjectType): type is ObjectOf<{ table: typeof chunk; fields: {} }> {
    return type.table === chunk;
}

/** Assemble a text's sequence from its chunks in position order. */
function sequence(rows: readonly { readonly runs: readonly Run[] }[]): Sequence {
    return new Sequence(rows.flatMap((row) => row.runs));
}
