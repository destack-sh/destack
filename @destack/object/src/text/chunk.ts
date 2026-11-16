import { included, type ObjectReference, type Item, type Query } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { through } from "@destack/access";
import { type Transaction, type Snapshot, Table } from "@destack/db";
import { schema, canonicalize, Identifier, present } from "@destack/schema";

import { FractionalIndex } from "../field/fractional-index.ts";
import type { MethodContext } from "../method/context.ts";
import { method } from "../method/record.ts";
import { type ObjectDefinition, ObjectType } from "../object/object.ts";
import { Run, Sequence, SequenceEdit, type Element } from "../sequence/index.ts";
import { chunk, chunkRun, CHUNKS } from "./table.ts";
import { Execution } from "../method/execution.ts";

import type {} from "@destack/package/import-meta";
import { argument, plural, t } from "@destack/locale";

/** The most characters per chunk: a keystroke syncs one ~1 KiB row, 1 MB is ~2,000 rows. */
export const CHUNK_CHARACTERS = 512;

/** The last whitespace character of a text, where a chunk ends without cutting a word. */
const LAST_SPACE = /\s(?=\S*$)/u;

/** The characters left in each chunk after a split: three quarters, with room to type. */
const FILL_CHARACTERS = (CHUNK_CHARACTERS * 3) / 4;

/** The owner permission whose holders read its text. */
export const TEXT_READ = "text";

/** One chunk of a text. */
type Chunk = typeof chunk.$inferSelect;

/** The input of an edit: the text field and its edits in order. */
const EDIT_INPUT = schema.looseObject({
    field: schema.string(),
    edits: schema.array(SequenceEdit),
});

/** The columns of an included chunk that assemble its owner's text. */
const INCLUDED_CHUNK = schema.looseObject({
    field: schema.string(),
    position: schema.string(),
    runs: schema.array(Run),
});

/** The chunk rows a listed page includes for one owner. */
const INCLUDED_CHUNKS = schema.array(INCLUDED_CHUNK);

/** The columns identifying one text field of one owner. */
type Parent = Pick<Chunk, "scope" | "parentPackageId" | "parentType" | "parentId" | "field">;

/** Whether a text field has every element a query asks about. */
export const Containment = schema.object({
    /** Whether the field has every element. */
    isContained: schema.boolean(),
});

/** The result of an edit: the edits undoing it. */
export const Undo = schema.object({
    /** The edits undoing the edit, in order. */
    inverse: schema.array(SequenceEdit),
});
/** The result of an edit: the edits undoing it. */
export type Undo = schema.Infer<typeof Undo>;

/** Pieces of the texts of owners, read through their owner. */
export const Chunk = {
    /** Declare the chunk type of the owners served together. */
    object(owners: readonly ObjectType[]): ObjectType {
        // require owners in one kind of scope
        const first = present(owners[0], "an owner with text");
        const isOneScope = owners.every(
            (owner) =>
                owner.scopes.length === first.scopes.length &&
                owner.scopes.every((scope, index) => scope.same(first.scopes[index])),
        );
        if (!isOneScope) {
            throw new TypeError("objects with text must use one kind of scope per database");
        }

        // declare chunks read through their owner, stored in text's chunk table
        const methods = { list: method.list("read") };
        const definition: ObjectDefinition = {
            name: "chunk",
            plural: CHUNKS,
            title: t`${plural(argument("count"), { one: "Text chunk", other: "Text chunks" })}`,
            scope: first.scope,
            nested: { in: "any", add: TEXT_READ },
            relations: { parent: { subjects: [...owners], grantedBy: null } },
            permissions: { read: through("parent", TEXT_READ) },
            methods,
            mapping: {
                table: chunk,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {
                    parent: {
                        column: "parentId",
                        subject: {
                            packageId: "parentPackageId",
                            type: "parentType",
                            scope: "scope",
                        },
                    },
                },
            },
        };

        return ObjectType.assemble(import.meta.destack.package, {
            ...definition,
            table: chunk,
            methods,
            source: definition,
        });
    },

    /** Report whether an owner's text field contains every element. */
    async contains(
        view: Pick<Snapshot, "read">,
        owner: ObjectReference,
        field: string,
        elements: readonly Element[],
    ): Promise<boolean> {
        const parent: Parent = {
            scope: owner.scope,
            parentPackageId: owner.packageId,
            parentType: owner.type,
            parentId: owner.id,
            field,
        };
        const runs = await view.read(chunkRun).findMany({
            where: {
                ...parent,
                run: { in: elements.map((element) => element.run) },
            },
        });

        return elements.every((element) =>
            runs.some(
                (entry) =>
                    entry.run === element.run &&
                    entry.start <= element.offset &&
                    element.offset < entry.end,
            ),
        );
    },

    /** Read the texts of some owners, by owner identifier. */
    async texts(
        transaction: Transaction,
        owner: ObjectType,
        scope: string,
        ids: readonly string[],
    ): Promise<Map<string, Record<string, string>>> {
        // read the owners' chunks
        const rows =
            ids.length === 0
                ? []
                : await transaction
                      .view()
                      .read(chunk)
                      .findMany({
                          where: {
                              scope,
                              parentPackageId: owner.policy.definition.packageId,
                              parentType: owner.name,
                              parentId: { in: [...ids] },
                          },
                      });

        return new Map(
            ids.map((id) => [
                id,
                Chunk.text(
                    owner,
                    rows.filter((row) => row.parentId === id),
                ),
            ]),
        );
    },

    /** Read the chunk rows a listed page includes for one owner, in their JSON form. */
    included(value: unknown): readonly Pick<Chunk, "field" | "position" | "runs">[] {
        return INCLUDED_CHUNKS.parse(value ?? []);
    },

    /** Assemble an owner's visible texts from its chunks, by field. */
    text(
        owner: ObjectType,
        rows: readonly Pick<Chunk, "field" | "position" | "runs">[],
    ): Record<string, string> {
        return Object.fromEntries(
            owner.text.map((field) => [
                field,
                new Sequence(
                    ChunkLayout.ordered(rows.filter((row) => row.field === field)).flatMap(
                        (row) => row.runs,
                    ),
                ).text(),
            ]),
        );
    },

    /** Apply a call's edits to one text field, writing the changed chunks. */
    async edit(context: MethodContext, execution: Execution): Promise<Undo> {
        // read the chunks the edits touch
        const { field, edits } = EDIT_INPUT.parse(context.input);
        const owner = context.object;
        const parent: Parent = {
            scope: context.scope,
            parentPackageId: owner.policy.definition.packageId,
            parentType: owner.name,
            parentId: present(context.id, `the edited ${owner.name}'s identifier`),
            field,
        };
        const rows =
            execution.kind === "client"
                ? await ChunkLayout.read(context, parent, undefined)
                : await ChunkLayout.touched(context, parent, edits);

        // apply each edit, inverting it against the text before
        const { sequence, inverse } = ChunkLayout.applyEdits(rows, edits, field);

        // cut the runs back into the chunks and write and index them
        const chunks = await ChunkLayout.recut(context, parent, rows, sequence.runs);
        await ChunkLayout.writeChunks(context, execution, chunks);

        return { inverse };
    },
};

/** Rows with their chunks replaced by texts, each row and list presented once while unchanged. */
export class TextPresenter {
    /** The presented rows, by the read row they present. */
    readonly #rows: WeakMap<object, Item>;
    /** The presented lists, by the read list they present. */
    readonly #lists: WeakMap<object, readonly Item[]>;

    /** Present nothing yet. */
    constructor() {
        this.#rows = new WeakMap();
        this.#lists = new WeakMap();
    }

    /** Replace each row's chunks by its texts, reusing presented rows and lists if unchanged. */
    present(rows: readonly Item[], query: Query, objects: readonly ObjectType[]): readonly Item[] {
        // take a list presented before
        const known = this.#lists.get(rows);
        if (known !== undefined) {
            return known;
        }

        // present each row once
        const owner = objects.find((object) => object.table === query.table);
        const listed = rows.map((row) => {
            const presented = this.#rows.get(row) ?? this.#row(row, query, objects, owner);
            this.#rows.set(row, presented);

            return presented;
        });
        this.#lists.set(rows, listed);

        return listed;
    }

    /** Present one item's includes and texts. */
    #row(
        item: Item,
        query: Query,
        objects: readonly ObjectType[],
        owner: ObjectType | undefined,
    ): Item {
        // present each include's items, leaving out the chunks
        const presented = Object.fromEntries(
            Object.entries(item.with).flatMap(([name, items]) =>
                name === CHUNKS
                    ? []
                    : [[name, this.present(items, included(query, name).query, objects)]],
            ),
        );

        // assemble a text owner's texts from the chunks its query includes
        const row =
            owner === undefined || owner.text.length === 0
                ? item.row
                : {
                      ...item.row,
                      ...Chunk.text(
                          owner,
                          present(item.with[CHUNKS], `the chunks of ${owner.name}`).map((entry) =>
                              INCLUDED_CHUNK.parse(entry.row),
                          ),
                      ),
                  };

        return { row, with: presented, measures: item.measures };
    }
}

/** A chunk an edit writes: as read, null for a new chunk, and as written. */
interface ChunkChange {
    /** The chunk as read, null for a new chunk. */
    readonly before: Chunk | null;
    /** The chunk as written. */
    readonly after: Chunk;
}

/** The layout of a text in chunks: cutting, filling, recutting and reindexing them. */
const ChunkLayout = {
    /** Read a text field's chunks in position order, within positions when given. */
    async read(
        context: MethodContext,
        parent: Parent,
        within: { readonly from: string; readonly to: string } | undefined,
    ): Promise<Chunk[]> {
        const rows = await context.read(chunk).findMany({
            where: {
                ...parent,
                ...(within === undefined ? {} : { position: { gte: within.from, lte: within.to } }),
            },
        });

        return ChunkLayout.ordered(rows);
    },

    /** Read the contiguous chunks containing the elements some edits refer to. */
    async touched(
        context: MethodContext,
        parent: Parent,
        edits: readonly SequenceEdit[],
    ): Promise<Chunk[]> {
        // find the runs containing the referenced elements and any run the edits insert
        const { named, inserted, isAtStart } = ChunkLayout.editedElements(edits);
        const runs =
            named.length === 0 && inserted.size === 0
                ? []
                : await ChunkLayout.containing(context, parent, named, inserted);
        ChunkLayout.requireContained(runs, named, inserted, parent.field);

        // read the chunks with those runs, and the field's first chunk for an insertion at the start
        const ids = [...new Set(runs.map((entry) => entry.chunk))];
        const rows =
            ids.length === 0
                ? []
                : await context.read(chunk).findMany({ where: { id: { in: ids } } });
        const first = isAtStart ? await ChunkLayout.first(context, parent) : [];
        const found = ChunkLayout.ordered([
            ...rows,
            ...first.filter((row) => !ids.includes(row.id)),
        ]);

        // take one chunk as it is, else read every chunk from the first touched to the last
        const [from] = found;
        const to = found.at(-1);

        return found.length <= 1 || from === undefined || to === undefined
            ? found
            : ChunkLayout.read(context, parent, { from: from.position, to: to.position });
    },

    /** Collect the elements some edits refer to, without inserted runs, and any start insertion. */
    editedElements(edits: readonly SequenceEdit[]): {
        readonly named: Element[];
        readonly inserted: Set<string>;
        readonly isAtStart: boolean;
    } {
        // walk the edits in order
        const inserted = new Set<string>();
        const named: Element[] = [];
        let isAtStart = false;
        for (const edit of edits) {
            // collect the elements of runs inserted before
            const elements = ChunkLayout.namedElements(edit);
            named.push(...elements.filter((element) => !inserted.has(element.run)));

            // remember the inserted run
            if ("insert" in edit) {
                isAtStart ||= edit.after === undefined;
                inserted.add(edit.run);
            }
        }

        return { named, inserted, isAtStart };
    },

    /** Read the indexed runs containing some elements, and any run of some inserted ones. */
    async containing(
        context: MethodContext,
        parent: Parent,
        named: readonly Element[],
        inserted: ReadonlySet<string>,
    ) {
        return context.read(chunkRun).findMany({
            where: {
                ...parent,
                OR: [
                    ...named.map((element) => ({
                        run: element.run,
                        start: { lte: element.offset },
                        end: { gt: element.offset },
                    })),
                    ...(inserted.size === 0 ? [] : [{ run: { in: [...inserted] } }]),
                ],
            },
        });
    },

    /** Refuse a run the text already contains and an element no chunk contains. */
    requireContained(
        found: readonly { readonly run: string; readonly start: number; readonly end: number }[],
        named: readonly Element[],
        inserted: ReadonlySet<string>,
        field: string,
    ): void {
        // refuse a run the text already contains
        const duplicate = found.find((entry) => inserted.has(entry.run));
        if (duplicate !== undefined) {
            throw ObjectError.of(
                "RUN_INVALID",
                `run '${duplicate.run}' is invalid: '${field}' already has it`,
            );
        }

        // refuse an element no chunk contains
        const missing = named.find(
            (element) =>
                !found.some(
                    (entry) =>
                        entry.run === element.run &&
                        entry.start <= element.offset &&
                        element.offset < entry.end,
                ),
        );
        if (missing !== undefined) {
            throw ObjectError.of(
                "ELEMENT_INVALID",
                `element '${missing.run}' at offset ${missing.offset} is invalid: no chunk has it`,
            );
        }
    },

    /** Read a text field's first chunk, none for an empty field. */
    async first(context: MethodContext, parent: Parent): Promise<Chunk[]> {
        return context
            .read(chunk)
            .findMany({ where: { ...parent }, orderBy: { position: "asc" }, limit: 1 });
    },

    /** Apply edits to a text's runs, collecting the edits inverting them in reverse order. */
    applyEdits(
        rows: readonly Chunk[],
        edits: readonly SequenceEdit[],
        field: string,
    ): { readonly sequence: Sequence; readonly inverse: SequenceEdit[] } {
        // start from the text before
        let sequence = new Sequence(rows.flatMap((row) => row.runs));
        const inverses: SequenceEdit[][] = [];
        for (const edit of edits) {
            // refuse inserting a run the text already has
            if ("insert" in edit && sequence.runs.some((piece) => piece.run === edit.run)) {
                throw ObjectError.of(
                    "RUN_INVALID",
                    `run '${edit.run}' is invalid: '${field}' already has it`,
                );
            }

            // apply the edit after inverting it against the text before
            try {
                inverses.unshift(Sequence.inverse(edit, sequence));
                sequence = sequence.apply(edit);
            } catch (error) {
                if (error instanceof RangeError) {
                    throw ObjectError.of("TEXT_EDIT_INVALID", error.message);
                }
                throw error;
            }
        }

        return { sequence, inverse: inverses.flat() };
    },

    /** Cut edited runs back into a field's chunks, splitting full ones into new chunks. */
    async recut(
        context: MethodContext,
        parent: Parent,
        rows: readonly Chunk[],
        edited: readonly Run[],
    ): Promise<ChunkChange[]> {
        // cut at the starts of the chunks after the first
        const segments = ChunkLayout.cut(
            edited,
            rows.slice(1).map((row) => ChunkLayout.start(row)),
        );
        const changes: ChunkChange[] = [];
        for (const [index, segment] of segments.entries()) {
            // take a changed chunk's first group
            const current = rows[index];
            const [first, ...rest] = ChunkLayout.fill(segment);
            if (current !== undefined && canonicalize(first) !== canonicalize(current.runs)) {
                const after = { ...current, runs: first, text: new Sequence(first).text() };
                changes.push({ before: current, after });
            }

            // place new chunks between this one and the next
            const groups = current === undefined ? [first, ...rest] : rest;
            let before = current?.position;
            const isLast = index + 1 >= rows.length && !context.isPredicted;
            let after: string | undefined;
            if (groups.length > 0) {
                after = isLast
                    ? await ChunkLayout.next(context, parent, current?.position)
                    : rows[index + 1]?.position;
            }
            for (const runs of groups) {
                before = FractionalIndex.between(before, after);
                const created = Table.complete(chunk, {
                    ...parent,
                    id: Identifier.create("chunk", context.environment),
                    position: before,
                    runs,
                    text: new Sequence(runs).text(),
                });
                changes.push({ before: null, after: created });
            }
        }

        return changes;
    },

    /** Write the changed and new chunks, and index their runs on the server. */
    async writeChunks(
        context: MethodContext,
        execution: Execution,
        changes: readonly ChunkChange[],
    ): Promise<void> {
        // write the chunks over the rows the edit read
        await Execution.write(
            execution,
            context,
            ...changes.map(({ before, after }) => ({ table: chunk, before, after })),
        );

        // index the runs on the server
        if (execution.kind === "store") {
            await ChunkLayout.reindex(context, execution, changes);
        }
    },

    /** Read the position of the chunk after one, absent after the last. */
    async next(
        context: MethodContext,
        parent: Parent,
        position: string | undefined,
    ): Promise<string | undefined> {
        // take the first chunk after the position
        if (position === undefined) {
            return undefined;
        }
        const [after] = await context.read(chunk).findMany({
            where: { ...parent, position: { gt: position } },
            orderBy: { position: "asc" },
            limit: 1,
        });

        return after?.position;
    },

    /** Write the index rows a chunk's runs gain and lose, as derived from its runs' ranges. */
    async reindex(
        context: MethodContext,
        execution: Execution,
        changes: readonly ChunkChange[],
    ): Promise<void> {
        // derive each chunk's index rows before and after, by their ranges
        const writes = changes.flatMap(({ before, after }) => {
            const old = new Map(
                (before === null ? [] : ChunkLayout.indexRows(before)).map((row) => [
                    canonicalize(row),
                    row,
                ]),
            );
            const current = new Map(
                ChunkLayout.indexRows(after).map((row) => [canonicalize(row), row]),
            );

            // drop the rows the chunk lost and add the ones it gained
            return [
                ...[...old]
                    .filter(([key]) => !current.has(key))
                    .map(([, row]) => ({ table: chunkRun, before: row, after: null })),
                ...[...current]
                    .filter(([key]) => !old.has(key))
                    .map(([, row]) => ({ table: chunkRun, before: null, after: row })),
            ];
        });

        // write them together
        if (writes.length > 0) {
            await Execution.write(execution, context, ...writes);
        }
    },

    /** Derive a chunk's index rows from its runs' ranges. */
    indexRows(row: Chunk) {
        return ChunkLayout.ranges(row.runs).map((range) =>
            Table.complete(chunkRun, {
                chunk: row.id,
                scope: row.scope,
                parentPackageId: row.parentPackageId,
                parentType: row.parentType,
                parentId: row.parentId,
                field: row.field,
                ...range,
            }),
        );
    },

    /** List the element ranges of runs, merging continued pieces of one run. */
    ranges(runs: readonly Run[]): { run: string; start: number; end: number }[] {
        const merged: { run: string; start: number; end: number }[] = [];
        for (const piece of runs) {
            // extend the previous range when the piece continues it
            const previous = merged.at(-1);
            const end = piece.start + Sequence.length(piece);
            if (
                previous !== undefined &&
                previous.run === piece.run &&
                previous.end === piece.start
            ) {
                previous.end = end;
            } else {
                merged.push({ run: piece.run, start: piece.start, end });
            }
        }

        return merged;
    },

    /** Order chunks by position, as binary collations sort them. */
    ordered<Row extends Pick<Chunk, "position">>(rows: readonly Row[]): Row[] {
        return rows.toSorted((left, right) => (left.position < right.position ? -1 : 1));
    },

    /** Split runs into sections starting at each cut element. */
    cut(runs: readonly Run[], cuts: readonly Element[]): Run[][] {
        // walk the pieces toward the next cut
        let segment: Run[] = [];
        const segments = [segment];
        let next = 0;
        for (let piece of runs) {
            // start a section at each cut in the piece
            for (
                let cut = cuts[next];
                cut !== undefined && ChunkLayout.contains(piece, cut);
                cut = cuts[next]
            ) {
                const [head, tail] = Sequence.split(piece, cut.offset - piece.start);
                if (tail !== undefined) {
                    segment.push(head);
                    piece = tail;
                }
                segment = [];
                segments.push(segment);
                next += 1;
            }
            segment.push(piece);
        }

        return segments;
    },

    /** Split a section's runs into even groups of at most a chunk's characters. */
    fill(runs: readonly Run[]): [Run[], ...Run[][]] {
        // store a text that fits one chunk whole, and split a longer one evenly into partly full chunks
        const total = runs.reduce((sum, piece) => sum + ChunkLayout.characters(piece), 0);
        const count = total <= CHUNK_CHARACTERS ? 1 : Math.ceil(total / FILL_CHARACTERS);
        const capacity = Math.ceil(total / count);

        // fill each group, splitting a piece at a group's end
        let group: Run[] = [];
        const groups: [Run[], ...Run[][]] = [group];
        let filled = 0;
        for (let piece of runs) {
            while (ChunkLayout.characters(piece) > capacity - filled) {
                const room = ChunkLayout.wordEnd(piece, capacity - filled);
                if (room > 0) {
                    const [head, tail] = Sequence.split(piece, room);
                    group.push(head);
                    piece = present(tail, "the tail of a piece split before its end");
                }
                group = [];
                groups.push(group);
                filled = 0;
            }
            group.push(piece);
            filled += ChunkLayout.characters(piece);
        }

        return groups;
    },

    /** Find where a group ends in a piece's room, after a space in its second half. */
    wordEnd(piece: Run, room: number): number {
        const text = typeof piece.text === "string" ? piece.text.slice(0, room) : "";
        const space = text.search(LAST_SPACE);

        return space >= room / 2 ? space + 1 : room;
    },

    /** Count the characters a piece stores. */
    characters(piece: Run): number {
        return typeof piece.text === "string" ? piece.text.length : 0;
    },

    /** Decide whether a piece contains an element. */
    contains(piece: Run, element: Element): boolean {
        return (
            piece.run === element.run &&
            element.offset >= piece.start &&
            element.offset < piece.start + Sequence.length(piece)
        );
    },

    /** Read the element a chunk starts at, the start of its section. */
    start(row: Chunk): Element {
        const first = present(row.runs[0], `the first run of chunk ${row.id}`);

        return { run: first.run, offset: first.start };
    },

    /** List the elements an edit refers to: an insertion's anchor, or each named piece's ends. */
    namedElements(edit: SequenceEdit): Element[] {
        // take an insertion's anchor, else the first and last element of each deleted or restored piece
        if ("insert" in edit) {
            return edit.after === undefined ? [] : [edit.after];
        } else {
            return ("delete" in edit ? edit.delete : edit.restore).flatMap((piece) => [
                { run: piece.run, offset: piece.start },
                { run: piece.run, offset: piece.start + Sequence.length(piece) - 1 },
            ]);
        }
    },
};
