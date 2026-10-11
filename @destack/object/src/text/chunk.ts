import { included, type ObjectReference, type Item, type Query } from "@destack/sync";
import { through } from "@destack/access";
import { TABLE, type Transaction, type Snapshot } from "@destack/db";
import { schema, canonicalize, Identifier, present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { FractionalIndex } from "../field/fractional-index.ts";
import { ObjectReader } from "../method/reader.ts";
import type { Call } from "../method/call.ts";
import { method } from "../method/record.ts";
import { ObjectType, type ObjectSource } from "../object/object.ts";
import { INTRINSIC } from "../object/intrinsic.ts";
import { Run, Sequence, SequenceEdit, type Element } from "../sequence/index.ts";
import { chunk, chunkRun } from "./table.ts";

/** The most characters one chunk has: a keystroke rewrites and syncs one ~1 KiB row, 1 MB is ~2,000 rows. */
export const CHUNK_CHARACTERS = 512;

/** The characters a split leaves in each chunk: three quarters full, with room to type into one row. */
const FILL_CHARACTERS = (CHUNK_CHARACTERS * 3) / 4;

/** The rows presented before, by the read row, which a dataflow keeps while it is unchanged. */
const PRESENTED_ROWS = new WeakMap<object, Item>();

/** The lists presented before, by the read list, which a dataflow keeps while every row is unchanged. */
const PRESENTED_LISTS = new WeakMap<object, readonly Item[]>();

/** The owner permission whose holders read its text. */
export const TEXT_READ = "text";

/** The include with each owner's chunks. */
export const CHUNKS = "chunks";

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
            throw new TypeError("objects with text live in one kind of scope per database");
        }

        // declare chunks read through their owner, kept in the chunk table text owns
        const methods = { list: method.list("read") };
        const definition: ObjectSource = {
            name: "chunk",
            plural: CHUNKS,
            scope: first.scope,
            nested: { in: "any", receive: TEXT_READ },
            relations: { parent: { subjects: [...owners], grantedBy: null } },
            permissions: { read: through("parent", TEXT_READ) },
            methods,
            [INTRINSIC]: {
                table: chunk,
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
        view: Transaction | Snapshot,
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
        const runs = await new ObjectReader(chunkRun, view).rows({
            ...parent,
            run: { in: elements.map((element) => element.run) },
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
                : await transaction.images(chunk, {
                      scope,
                      parentPackageId: owner.policy.definition.packageId,
                      parentType: owner.name,
                      parentId: { in: [...ids] },
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
                    ordered(rows.filter((row) => row.field === field)).flatMap((row) => row.runs),
                ).text(),
            ]),
        );
    },

    /** Replace each row's chunks by its texts, through a query's includes, presenting an unchanged row or list as before. */
    present(rows: readonly Item[], query: Query, objects: readonly ObjectType[]): readonly Item[] {
        // take a list presented before
        const known = PRESENTED_LISTS.get(rows);
        if (known !== undefined) {
            return known;
        }

        // present each row once
        const owner = objects.find((object) => object.table === query.table);
        const listed = rows.map((row) => {
            const presented = PRESENTED_ROWS.get(row) ?? presentRow(row, query, objects, owner);
            PRESENTED_ROWS.set(row, presented);

            return presented;
        });
        PRESENTED_LISTS.set(rows, listed);

        return listed;
    },

    /** Apply a call's edits to one text field, writing the changed chunks. */
    async edit(call: Call): Promise<Undo> {
        // read the chunks the edits touch
        const { field, edits } = EDIT_INPUT.parse(call.input);
        const owner = call.object;
        const parent: Parent = {
            scope: call.scope,
            parentPackageId: owner.policy.definition.packageId,
            parentType: owner.name,
            parentId: present(call.id, `the edited ${owner.name}'s identifier`),
            field,
        };
        const rows = call.isPredicted
            ? await read(call, parent, undefined)
            : await touched(call, parent, edits);

        // apply each edit, inverting it against the text before
        const { sequence, inverse } = applyEdits(rows, edits, field);

        // cut the runs back into the chunks and write and index them
        const chunks = await recut(call, parent, rows, sequence.runs);
        await writeChunks(call, chunks);

        return { inverse };
    },
};

/** The chunks an edit changes and creates, with the changed ones whose run ranges moved. */
interface ChunkChange {
    /** The existing chunks with changed runs. */
    readonly changed: Chunk[];
    /** The changed chunks whose run ranges the server reindexes. */
    readonly reranged: Chunk[];
    /** The new chunks. */
    readonly created: Chunk[];
}

/** Read a text field's chunks in position order, within positions when given. */
async function read(
    call: Call,
    parent: Parent,
    within: { readonly from: string; readonly to: string } | undefined,
): Promise<Chunk[]> {
    const rows = await call.read(chunk).rows({
        ...parent,
        ...(within === undefined ? {} : { position: { gte: within.from, lte: within.to } }),
    });

    return ordered(rows);
}

/** Read the contiguous chunks containing the elements some edits refer to. */
async function touched(
    call: Call,
    parent: Parent,
    edits: readonly SequenceEdit[],
): Promise<Chunk[]> {
    // find the chunks containing the referenced elements and any run the edits insert
    const { named, inserted, isAtStart } = editedElements(edits);
    const found =
        named.length === 0 && inserted.size === 0
            ? []
            : await containing(call, parent, named, inserted);
    requireContained(found, named, inserted, parent.field);

    // add the field's first chunk for an insertion at the start
    const positions = found.map((entry) => entry.position);
    if (isAtStart) {
        positions.push(...(await firstPosition(call, parent)));
    }

    // read the range from the first touched chunk to the last
    const sorted = positions.toSorted();
    const [from] = sorted;
    const to = sorted.at(-1);

    return from === undefined || to === undefined ? [] : read(call, parent, { from, to });
}

/** Collect the elements some edits refer to, skipping the runs they insert, and whether one inserts at the start. */
function editedElements(edits: readonly SequenceEdit[]): {
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
        const elements = namedElements(edit);
        named.push(...elements.filter((element) => !inserted.has(element.run)));

        // remember the inserted run
        if ("insert" in edit) {
            isAtStart ||= edit.after === undefined;
            inserted.add(edit.run);
        }
    }

    return { named, inserted, isAtStart };
}

/** Read the indexed runs containing some elements, and any run of some inserted ones. */
async function containing(
    call: Call,
    parent: Parent,
    named: readonly Element[],
    inserted: ReadonlySet<string>,
) {
    // read the runs containing the named elements and the inserted runs
    const runs = await call.read(chunkRun).rows({
        ...parent,
        OR: [
            ...named.map((element) => ({
                run: element.run,
                start: { lte: element.offset },
                end: { gt: element.offset },
            })),
            ...(inserted.size === 0 ? [] : [{ run: { in: [...inserted] } }]),
        ],
    });

    // place each run at its chunk's position
    const chunks = await call.read(chunk).rows({ id: { in: runs.map((entry) => entry.chunk) } });
    const positions = new Map(chunks.map((row) => [row.id, row.position]));

    return runs.map((entry) => ({
        run: entry.run,
        start: entry.start,
        end: entry.end,
        position: present(positions.get(entry.chunk), `the chunk of run ${entry.run}`),
    }));
}

/** Refuse a run the text already contains and an element no chunk contains. */
function requireContained(
    found: readonly { readonly run: string; readonly start: number; readonly end: number }[],
    named: readonly Element[],
    inserted: ReadonlySet<string>,
    field: string,
): void {
    // refuse a run the text already contains
    const duplicate = found.find((entry) => inserted.has(entry.run));
    if (duplicate !== undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: `${field} already holds run ${duplicate.run}`,
        });
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
        throw new ServiceError("BAD_REQUEST", {
            message: `no chunk holds element ${missing.run}:${missing.offset}`,
        });
    }
}

/** Read the position of a text field's first chunk, none for an empty field. */
async function firstPosition(call: Call, parent: Parent): Promise<string[]> {
    const [first] = await call
        .read(chunk)
        .rows({ ...parent }, { orderBy: { position: "asc" }, limit: 1 });

    return first === undefined ? [] : [first.position];
}

/** Apply edits to a text's runs, collecting the edits inverting them in reverse order. */
function applyEdits(
    rows: readonly Chunk[],
    edits: readonly SequenceEdit[],
    field: string,
): { readonly sequence: Sequence; readonly inverse: SequenceEdit[] } {
    // start from the text before
    let sequence = new Sequence(rows.flatMap((row) => row.runs));
    const inverses: SequenceEdit[][] = [];
    for (const edit of edits) {
        // refuse inserting a run the text holds
        if ("insert" in edit && sequence.runs.some((piece) => piece.run === edit.run)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${field} already holds run ${edit.run}`,
            });
        }

        // apply the edit after inverting it against the text before
        try {
            inverses.unshift(Sequence.inverse(edit, sequence));
            sequence = sequence.apply(edit);
        } catch (error) {
            if (error instanceof RangeError) {
                throw new ServiceError("BAD_REQUEST", { message: error.message });
            }
            throw error;
        }
    }

    return { sequence, inverse: inverses.flat() };
}

/** Cut edited runs back into a field's chunks, splitting full ones into new chunks between them. */
async function recut(
    call: Call,
    parent: Parent,
    rows: readonly Chunk[],
    edited: readonly Run[],
): Promise<ChunkChange> {
    // cut at the boundaries of the chunks after the first
    const parts = cut(edited, rows.slice(1).map(start));
    const chunks: ChunkChange = { changed: [], reranged: [], created: [] };
    for (const [index, part] of parts.entries()) {
        // keep a changed chunk's first group, noting one whose ranges the server reindexes
        const current = rows[index];
        const [first, ...rest] = fill(part);
        if (current !== undefined && canonicalize(first) !== canonicalize(current.runs)) {
            const row = { ...current, runs: first };
            chunks.changed.push(row);
            if (
                !call.isPredicted &&
                canonicalize(ranges(first)) !== canonicalize(ranges(current.runs))
            ) {
                chunks.reranged.push(row);
            }
        }

        // place new chunks between this one and the next
        const groups = current === undefined ? [first, ...rest] : rest;
        let before = current?.position;
        const isLast = index + 1 >= rows.length && !call.isPredicted;
        let after: string | undefined;
        if (groups.length > 0) {
            after = isLast
                ? await next(call, parent, current?.position)
                : rows[index + 1]?.position;
        }
        for (const runs of groups) {
            before = FractionalIndex.between(before, after);
            chunks.created.push({
                ...parent,
                id: Identifier.create("chunk"),
                position: before,
                runs,
            });
        }
    }

    return chunks;
}

/** Write the changed and new chunks, and index their runs on the server. */
async function writeChunks(call: Call, chunks: ChunkChange): Promise<void> {
    // read the changed chunks at once
    const changed = chunks.changed.map((row) => row.id);
    const before = new Map(
        (changed.length === 0 ? [] : await call.read(chunk).rows({ id: { in: changed } })).map(
            (row) => [row.id, row],
        ),
    );

    // write the changed and new chunks together
    await call.write(
        ...chunks.changed.map((row) => {
            const current = present(before.get(row.id), `the chunk ${row.id}`);

            return { table: chunk, before: current, after: { ...current, ...row } };
        }),
        ...chunks.created.map((row) => ({
            table: chunk,
            before: null,
            after: chunk[TABLE].complete(row),
        })),
    );

    // index the runs of changed and new chunks on the server
    if (!call.isPredicted) {
        await reindex(call, [...chunks.reranged, ...chunks.created]);
    }
}

/** Read the position of the chunk after one, absent after the last. */
async function next(
    call: Call,
    parent: Parent,
    position: string | undefined,
): Promise<string | undefined> {
    // take the first chunk after the position
    if (position === undefined) {
        return undefined;
    }
    const [after] = await call
        .read(chunk)
        .rows(
            { ...parent, position: { gt: position } },
            { orderBy: { position: "asc" }, limit: 1 },
        );

    return after?.position;
}

/** Replace the index rows of some chunks with their runs' ranges. */
async function reindex(call: Call, rows: readonly Chunk[]): Promise<void> {
    // read the chunks' old ranges
    if (rows.length === 0) {
        return;
    }
    const old = await call.read(chunkRun).rows({ chunk: { in: rows.map((row) => row.id) } });

    // replace them with their current ranges together
    await call.write(
        ...old.map((range) => ({ table: chunkRun, before: range, after: null })),
        ...rows.flatMap((row) =>
            ranges(row.runs).map((range) => ({
                table: chunkRun,
                before: null,
                after: chunkRun[TABLE].complete({
                    chunk: row.id,
                    scope: row.scope,
                    parentPackageId: row.parentPackageId,
                    parentType: row.parentType,
                    parentId: row.parentId,
                    field: row.field,
                    ...range,
                }),
            })),
        ),
    );
}

/** List the element ranges of runs, merging continued pieces of one run. */
function ranges(runs: readonly Run[]): { run: string; start: number; end: number }[] {
    const merged: { run: string; start: number; end: number }[] = [];
    for (const piece of runs) {
        // extend the previous range when the piece continues it
        const previous = merged.at(-1);
        const end = piece.start + Sequence.length(piece);
        if (previous !== undefined && previous.run === piece.run && previous.end === piece.start) {
            previous.end = end;
        } else {
            merged.push({ run: piece.run, start: piece.start, end });
        }
    }

    return merged;
}

/** Order chunks by position, as binary collations sort them. */
function ordered<Row extends Pick<Chunk, "position">>(rows: readonly Row[]): Row[] {
    return rows.toSorted((left, right) => (left.position < right.position ? -1 : 1));
}

/** Split runs into sections starting at each boundary element. */
function cut(runs: readonly Run[], boundaries: readonly Element[]): Run[][] {
    // walk the pieces toward the next boundary
    let part: Run[] = [];
    const parts = [part];
    let reached = 0;
    for (let piece of runs) {
        // start a section at each boundary in the piece
        for (
            let boundary = boundaries[reached];
            boundary !== undefined && contains(piece, boundary);
            boundary = boundaries[reached]
        ) {
            const [head, tail] = Sequence.split(piece, boundary.offset - piece.start);
            if (tail !== undefined) {
                part.push(head);
                piece = tail;
            }
            part = [];
            parts.push(part);
            reached += 1;
        }
        part.push(piece);
    }

    return parts;
}

/** Split a section's runs into even groups of at most a chunk's characters. */
function fill(runs: readonly Run[]): [Run[], ...Run[][]] {
    // keep a text that fits one chunk whole, and split a longer one evenly into partly full chunks
    const total = runs.reduce((sum, piece) => sum + characters(piece), 0);
    const count = total <= CHUNK_CHARACTERS ? 1 : Math.ceil(total / FILL_CHARACTERS);
    const capacity = Math.ceil(total / count);

    // fill each group, splitting a piece at a group's end
    let group: Run[] = [];
    const groups: [Run[], ...Run[][]] = [group];
    let filled = 0;
    for (let piece of runs) {
        while (characters(piece) > capacity - filled) {
            const room = capacity - filled;
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
        filled += characters(piece);
    }

    return groups;
}

/** Count the characters a piece stores. */
function characters(piece: Run): number {
    return typeof piece.text === "string" ? piece.text.length : 0;
}

/** Decide whether a piece contains an element. */
function contains(piece: Run, element: Element): boolean {
    return (
        piece.run === element.run &&
        element.offset >= piece.start &&
        element.offset < piece.start + Sequence.length(piece)
    );
}

/** Present one item's includes and texts. */
function presentRow(
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
                : [[name, Chunk.present(items, included(query, name).query, objects)]],
        ),
    );

    // assemble the owner's texts from the chunks
    const chunks = (item.with[CHUNKS] ?? []).map((entry) => INCLUDED_CHUNK.parse(entry.row));
    const row =
        owner === undefined || owner.text.length === 0
            ? item.row
            : { ...item.row, ...Chunk.text(owner, chunks) };

    return { row, with: presented, measures: item.measures };
}

/** Read the element a chunk starts at, the boundary of its section. */
function start(row: Chunk): Element {
    const first = present(row.runs[0], `the first run of chunk ${row.id}`);

    return { run: first.run, offset: first.start };
}

/** List the elements an edit refers to: an insertion's anchor, or a deletion's or restoration's ends. */
function namedElements(edit: SequenceEdit): Element[] {
    // take an insertion's anchor
    if ("insert" in edit) {
        return edit.after === undefined ? [] : [edit.after];
    }
    // take a deletion's ends
    else if ("delete" in edit) {
        return [edit.delete.from, edit.delete.to];
    }
    // take a restoration's ends
    else {
        return [edit.restore.from, edit.restore.to];
    }
}
