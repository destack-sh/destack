import { Uuid } from "@destack/schema";
import type { ObjectType } from "../object/index.ts";
import { TextRope, TextSequence, type TextChange, type TextEdit } from "../sequence/index.ts";
import { textChunk } from "../text/object.ts";
import type { textChunkTable } from "../text/table.ts";
import type { LiveQuery, Submission } from "./access.ts";
import type { ObjectNode } from "./node.ts";
import { Observable } from "./observable.ts";

/** A chunk row of the replica. */
type TextRow = Pick<typeof textChunkTable.$inferSelect, "runs" | "marks">;

/** One text field of an object as an editor edits it. */
export class TextDocument {
    /** Settles once the replica has the text. */
    readonly ready: Promise<void>;
    /** The client editing the text. */
    readonly #client: ObjectNode;
    /** The object type with the text. */
    readonly #object: ObjectType;
    /** The object's identifier. */
    readonly #id: string;
    /** The text field. */
    readonly #field: string;
    /** The run of the elements this document types, renewed after a refusal. */
    #site: string;
    /** The offset of the next element this document types. */
    #clock = 0;
    /** The live query of the field's chunks. */
    readonly #query: LiveQuery<readonly TextRow[]>;
    /** The sequence of each chunk row of the replica, by row. */
    #rows = new Map<TextRow, TextSequence>();
    /** The edit calls whose prediction the replica lacks, in order. */
    #pending: (readonly TextEdit[])[] = [];
    /** The text with the pending edits. */
    readonly #rope = new Observable(new TextRope());
    /** Stops following the replica. */
    readonly #stop = new AbortController();
    /** Settles once the document stops following the replica. */
    readonly #following: Promise<void>;

    /** Follow one text field of an object on a client. */
    constructor(client: ObjectNode, object: ObjectType, id: string, field: string) {
        // require a text field
        if (!object.text.includes(field)) {
            throw new TypeError(`object '${object.name}' has no text field '${field}'`);
        }
        this.#client = client;
        this.#object = object;
        this.#id = id;
        this.#field = field;
        this.#site = Uuid.v7(client.environment);

        // follow the field's chunks in position order
        const query = client
            .queryType(textChunk)
            .findMany({
                where: { parentType: object.name, parentId: id, field },
                orderBy: { position: "asc" },
            })
            .subscribe();
        this.#query = query;
        this.ready = query.ready.then(async () => this.#rebase(await query.read()));

        // take every change the replica sees until closed
        this.#following = (async () => {
            for await (const rows of query.watch(this.#stop.signal)) {
                this.#rebase(rows);
            }
        })();
    }

    /** The text as it stands here, with the pending edits. */
    get rope(): TextRope {
        return this.#rope.value;
    }

    /** Yield the text now and again after each change, local or from the replica. */
    watch(signal: AbortSignal): AsyncGenerator<TextRope> {
        return this.#rope.watch(signal);
    }

    /** Replace visible characters between two offsets. */
    change(change: TextChange): Submission<void> {
        const edits = this.rope.change(change, { run: this.#site, offset: this.#clock });
        this.#clock += change.insert.length;

        return this.edit(edits);
    }

    /** Apply edits here at once and call the object's edit method with them, none for no edits. */
    edit(edits: readonly TextEdit[]): Submission<void> {
        // send nothing for no edits
        if (edits.length === 0) {
            return { predicted: Promise.resolve(), confirmed: Promise.resolve() };
        }

        // apply the edits here
        this.#pending.push(edits);
        this.#rope.set(edits.reduce((rope, edit) => rope.apply(edit), this.rope));

        // predict and push them as one call
        const submission = this.#client.call(this.#object, "edit", {
            id: this.#id,
            field: this.#field,
            edits,
        });

        // drop them once their prediction lands or fails, and renew the site after a refusal
        const landed = async () => {
            this.#pending = this.#pending.filter((pending) => pending !== edits);
            this.#rebase(await this.#query.read());
        };
        void submission.predicted.then(landed, landed);
        submission.confirmed.catch(() => this.#renew());

        return { predicted: submission.predicted.then(() => {}), confirmed: submission.confirmed };
    }

    /** Stop following the text. */
    async close(): Promise<void> {
        // stop the follow loop, then the query and the rope
        this.#stop.abort();
        await this.#following;
        await this.#query.close();
        this.#rope.close();
    }

    /** Type into a new run, after the store refused elements of the current one. */
    #renew(): void {
        this.#site = Uuid.v7(this.#client.environment);
        this.#clock = 0;
    }

    /** Take the replica's chunks and apply the pending edits over them. */
    #rebase(rows: readonly TextRow[]): void {
        // read each new row's sequence once
        const known = this.#rows;
        this.#rows = new Map(
            rows.map((row) => [row, known.get(row) ?? TextDocument.sequence(row)]),
        );

        // apply the edits the replica lacks
        const replica = new TextRope([...this.#rows.values()]);
        this.#rope.set(this.#pending.flat().reduce((rope, edit) => rope.apply(edit), replica));
    }

    /** Read a chunk row's sequence. */
    static sequence(row: TextRow): TextSequence {
        return new TextSequence(row.runs, row.marks);
    }
}
