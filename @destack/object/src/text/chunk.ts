import { type ObjectReference, through } from "@destack/access";
import {
    and,
    asc,
    eq,
    gt,
    gte,
    inArray,
    lte,
    or,
    type DatabaseConnection,
    type SQL,
} from "@destack/db";
import { schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import { v7 } from "uuid";
import { Position } from "../field/field.ts";
import type { Call } from "../method/call.ts";
import { method } from "../method/method.ts";
import { ObjectType } from "../object/object.ts";
import { Run, Sequence, SequenceEdit, type Element } from "../sequence/index.ts";
import { chunk, chunkRun } from "./table.ts";

/** The most characters one chunk holds: a keystroke rewrites and syncs one ~1 KiB row, 1 MB is ~2,000 rows. */
export const CHUNK_CHARACTERS = 512;

/** The characters a split leaves in each chunk: three quarters full, so typing there rewrites one row for a while. */
const FILL_CHARACTERS = (CHUNK_CHARACTERS * 3) / 4;

/** The owner permission whose holders read its text. */
export const TEXT_READ = "text";

/** The include holding each owner's chunks. */
export const CHUNKS = "chunks";

/** One chunk row. */
type ChunkRow = typeof chunk.$inferSelect;

/** The columns naming one text field of one owner. */
type Parent = Pick<ChunkRow, "scope" | "parentPackageId" | "parentType" | "parentId" | "field">;

/** The result of an edit: the edits undoing it. */
export const Edited = schema.object({
    /** The edits undoing the edit, in order. */
    inverse: schema.array(SequenceEdit),
});
/** The result of an edit: the edits undoing it. */
export type Edited = schema.Infer<typeof Edited>;

/** Pieces of the texts owners hold, read through their owner. */
export const Chunk = {
    /** Declare the chunk type of the owners served together. */
    object(owners: readonly ObjectType[]): ObjectType {
        // require one kind of scope
        const [first] = owners;
        const isOneScope = owners.every(
            (owner) =>
                owner.scopes.length === first!.scopes.length &&
                owner.scopes.every((scope, index) => scope.same(first!.scopes[index])),
        );
        if (!isOneScope) {
            throw new TypeError("objects holding text live in one kind of scope per database");
        }

        // declare chunks read through their owner
        return new ObjectType(
            import.meta.destack.package,
            {
                name: "chunk",
                plural: CHUNKS,
                scope: first!.scope,
                table: chunk,
                nested: { in: "any", receive: TEXT_READ },
                relations: { parent: { subjects: [...owners], grantedBy: null } },
                permissions: { read: through("parent", TEXT_READ) },
                methods: { list: method.list("read") },
            },
            [],
            {
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
        );
    },

    /** Report whether an owner's text field holds every element. */
    async holds(
        database: DatabaseConnection,
        owner: ObjectReference,
        field: string,
        elements: readonly Element[],
    ): Promise<boolean> {
        const parent: Parent = {
            scope: owner.scope,
            parentPackageId: owner.packageId as Parent["parentPackageId"],
            parentType: owner.type,
            parentId: owner.id,
            field,
        };
        const held = await database
            .select({ run: chunkRun.run, start: chunkRun.start, end: chunkRun.end })
            .from(chunkRun)
            .where(
                and(
                    scoped(chunkRun, parent),
                    inArray(
                        chunkRun.run,
                        elements.map((element) => element.run),
                    ),
                ),
            );

        return elements.every((element) =>
            held.some(
                (entry) =>
                    entry.run === element.run &&
                    entry.start <= element.offset &&
                    element.offset < entry.end,
            ),
        );
    },

    /** Read the texts of some owners, by owner identifier. */
    async texts(
        database: DatabaseConnection,
        owner: ObjectType,
        scope: string,
        ids: readonly string[],
    ): Promise<Map<string, Record<string, string>>> {
        // read the owners' chunks
        const rows =
            ids.length === 0
                ? []
                : await database
                      .select()
                      .from(chunk)
                      .where(
                          and(
                              eq(chunk.scope, scope),
                              eq(chunk.parentPackageId, owner.policy.definition.packageId),
                              eq(chunk.parentType, owner.name),
                              inArray(chunk.parentId, [...ids]),
                          ),
                      );

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

    /** Assemble an owner's visible texts from its chunks, by field. */
    text(
        owner: ObjectType,
        rows: readonly Pick<ChunkRow, "field" | "position" | "runs">[],
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

    /** Replace each row's chunks by its texts, through a query's includes. */
    present(
        rows: readonly Readonly<Record<string, unknown>>[],
        query: Presented,
        objects: readonly ObjectType[],
    ): Record<string, unknown>[] {
        const owner = objects.find((object) => object.table === query.table);

        return rows.map((row) => {
            // present each nested include's rows
            const presented: Record<string, unknown> = { ...row };
            for (const [name, include] of Object.entries(query.include ?? {})) {
                const value = row[name];
                if (name === CHUNKS || include.aggregate !== undefined || value === null) {
                    continue;
                }
                presented[name] = Array.isArray(value)
                    ? Chunk.present(value as Record<string, unknown>[], include, objects)
                    : Chunk.present([value as Record<string, unknown>], include, objects)[0];
            }

            // assemble the owner's texts
            if (owner === undefined || owner.text.length === 0) {
                return presented;
            }
            const { [CHUNKS]: chunks, ...rest } = presented;

            return { ...rest, ...Chunk.text(owner, chunks as ChunkRow[]) };
        });
    },

    /** Apply a call's edits to one text field, writing the changed chunks. */
    async edit(call: Call): Promise<Edited> {
        // read the chunks the edits touch
        const { field, edits } = call.input as {
            readonly field: string;
            readonly edits: readonly SequenceEdit[];
        };
        const owner = call.object;
        const parent: Parent = {
            scope: call.scope,
            parentPackageId: owner.policy.definition.packageId,
            parentType: owner.name,
            parentId: call.id!,
            field,
        };
        const rows = call.isPredicted
            ? await read(call.database, parent, undefined)
            : await touched(call.database, parent, edits);

        // apply each edit, inverting it against the text before
        let sequence = new Sequence(rows.flatMap((row) => row.runs));
        const inverses: SequenceEdit[][] = [];
        for (const edit of edits) {
            if ("insert" in edit && sequence.runs.some((piece) => piece.run === edit.run)) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `${field} already holds run ${edit.run}`,
                });
            }
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

        // cut the runs back into the chunks and split full ones
        const parts = cut(
            sequence.runs,
            rows.slice(1).map((row) => ({ run: row.runs[0]!.run, offset: row.runs[0]!.start })),
        );
        const changed: ChunkRow[] = [];
        const created: ChunkRow[] = [];
        for (const [index, part] of parts.entries()) {
            // keep a changed chunk's first group
            const held = rows[index];
            const [first, ...rest] = fill(part);
            if (held !== undefined && canonicalize(first) !== canonicalize(held.runs)) {
                changed.push({ ...held, runs: first! });
            }

            // place new chunks between this one and the next
            const groups = held === undefined ? [first!, ...rest] : rest;
            let before = held?.position;
            const after =
                groups.length === 0
                    ? undefined
                    : index + 1 < rows.length || call.isPredicted
                      ? rows[index + 1]?.position
                      : await next(call.database, parent, held?.position);
            for (const runs of groups) {
                before = Position.between(before, after);
                created.push({ ...parent, id: `chunk-${v7()}` as never, position: before, runs });
            }
        }

        // write the changed and new chunks
        for (const row of changed) {
            await call.database.update(chunk).set({ runs: row.runs }).where(eq(chunk.id, row.id));
        }
        if (created.length > 0) {
            await call.database.insert(chunk).values(created);
        }

        // index the runs of changed and new chunks on the server
        if (!call.isPredicted) {
            const indexed = [
                ...changed.filter(
                    (row) =>
                        canonicalize(ranges(row.runs)) !==
                        canonicalize(ranges(rows.find((held) => held.id === row.id)!.runs)),
                ),
                ...created,
            ];
            await reindex(call.database, indexed);
        }

        return { inverse: inverses.flat() };
    },
};

/** Match one text field's rows of a chunk or index table. */
function scoped(table: typeof chunk | typeof chunkRun, parent: Parent): SQL {
    return and(
        eq(table.scope, parent.scope),
        eq(table.parentPackageId, parent.parentPackageId),
        eq(table.parentType, parent.parentType),
        eq(table.parentId, parent.parentId),
        eq(table.field, parent.field),
    )!;
}

/** Read a text field's chunks in position order, within positions when given. */
async function read(
    database: DatabaseConnection,
    parent: Parent,
    within: { readonly from: string; readonly to: string } | undefined,
): Promise<ChunkRow[]> {
    const rows = await database
        .select()
        .from(chunk)
        .where(
            and(
                scoped(chunk, parent),
                within === undefined ? undefined : gte(chunk.position, within.from),
                within === undefined ? undefined : lte(chunk.position, within.to),
            ),
        );

    return ordered(rows);
}

/** Read the contiguous chunks holding the elements some edits name. */
async function touched(
    database: DatabaseConnection,
    parent: Parent,
    edits: readonly SequenceEdit[],
): Promise<ChunkRow[]> {
    // collect the named elements, skipping runs the edits insert
    const inserted = new Set<string>();
    const named: Element[] = [];
    let isAtStart = false;
    for (const edit of edits) {
        const elements =
            "insert" in edit
                ? edit.after === undefined
                    ? []
                    : [edit.after]
                : "delete" in edit
                  ? [edit.delete.from, edit.delete.to]
                  : [edit.restore.from, edit.restore.to];
        named.push(...elements.filter((element) => !inserted.has(element.run)));
        if ("insert" in edit) {
            isAtStart ||= edit.after === undefined;
            inserted.add(edit.run);
        }
    }

    // find the chunks holding them, and any holding a run the edits insert
    const held =
        named.length === 0 && inserted.size === 0
            ? []
            : await database
                  .select({
                      run: chunkRun.run,
                      start: chunkRun.start,
                      end: chunkRun.end,
                      position: chunk.position,
                  })
                  .from(chunkRun)
                  .innerJoin(chunk, eq(chunk.id, chunkRun.chunk))
                  .where(
                      and(
                          scoped(chunkRun, parent),
                          or(
                              ...named.map((element) =>
                                  and(
                                      eq(chunkRun.run, element.run),
                                      lte(chunkRun.start, element.offset),
                                      gt(chunkRun.end, element.offset),
                                  ),
                              ),
                              inserted.size === 0
                                  ? undefined
                                  : inArray(chunkRun.run, [...inserted]),
                          ),
                      ),
                  );

    // refuse a run the text already holds
    const duplicate = held.find((entry) => inserted.has(entry.run));
    if (duplicate !== undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: `${parent.field} already holds run ${duplicate.run}`,
        });
    }

    // refuse an element no chunk holds
    const missing = named.find(
        (element) =>
            !held.some(
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

    // add the field's first chunk for an insertion at the start
    const positions = held.map((entry) => entry.position);
    if (isAtStart) {
        const [first] = await database
            .select({ position: chunk.position })
            .from(chunk)
            .where(scoped(chunk, parent))
            .orderBy(asc(chunk.position))
            .limit(1);
        positions.push(...(first === undefined ? [] : [first.position]));
    }

    // read the range from the first touched chunk to the last
    const sorted = [...positions].sort();

    return sorted.length === 0
        ? []
        : read(database, parent, { from: sorted[0]!, to: sorted.at(-1)! });
}

/** Read the position of the chunk after one, absent after the last. */
async function next(
    database: DatabaseConnection,
    parent: Parent,
    position: string | undefined,
): Promise<string | undefined> {
    // take the first chunk after the position
    if (position === undefined) {
        return undefined;
    }
    const [after] = await database
        .select({ position: chunk.position })
        .from(chunk)
        .where(and(scoped(chunk, parent), gt(chunk.position, position)))
        .orderBy(asc(chunk.position))
        .limit(1);

    return after?.position;
}

/** Replace the index rows of some chunks with their runs' ranges. */
async function reindex(database: DatabaseConnection, rows: readonly ChunkRow[]): Promise<void> {
    // drop the chunks' old ranges
    if (rows.length === 0) {
        return;
    }
    await database.delete(chunkRun).where(
        inArray(
            chunkRun.chunk,
            rows.map((row) => row.id),
        ),
    );

    // insert their current ranges at once
    await database.insert(chunkRun).values(
        rows.flatMap((row) =>
            ranges(row.runs).map((range) => ({
                chunk: row.id,
                scope: row.scope,
                parentPackageId: row.parentPackageId,
                parentType: row.parentType,
                parentId: row.parentId,
                field: row.field,
                ...range,
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

/** A query or include whose rows hold chunks. */
type Presented = {
    /** The table holding the rows. */
    readonly table: unknown;
    /** The includes of the rows, by name. */
    readonly include?: Readonly<Record<string, Presented & { readonly aggregate?: unknown }>>;
};

/** Order chunks by position, as binary collations sort them. */
function ordered<Row extends Pick<ChunkRow, "position">>(rows: readonly Row[]): Row[] {
    return [...rows].sort((left, right) => (left.position < right.position ? -1 : 1));
}

/** Split runs into parts starting at each boundary element. */
function cut(runs: readonly Run[], boundaries: readonly Element[]): Run[][] {
    // walk the pieces toward the next boundary
    const parts: Run[][] = [[]];
    let next = 0;
    for (let piece of runs) {
        // start a part at each boundary the piece holds
        while (next < boundaries.length && holds(piece, boundaries[next]!)) {
            const [head, tail] = Sequence.split(piece, boundaries[next]!.offset - piece.start);
            if (tail !== undefined) {
                parts.at(-1)!.push(head);
                piece = tail;
            }
            parts.push([]);
            next += 1;
        }
        parts.at(-1)!.push(piece);
    }

    return parts;
}

/** Split a part's runs into even groups of at most a chunk's characters. */
function fill(runs: readonly Run[]): Run[][] {
    // keep a text that fits one chunk whole, and split a longer one evenly into partly full chunks
    const total = runs.reduce((sum, piece) => sum + characters(piece), 0);
    const count = total <= CHUNK_CHARACTERS ? 1 : Math.ceil(total / FILL_CHARACTERS);
    const capacity = Math.ceil(total / count);

    // fill each group, splitting a piece at a group's end
    const groups: Run[][] = [[]];
    let held = 0;
    for (let piece of runs) {
        while (characters(piece) > capacity - held) {
            const room = capacity - held;
            if (room > 0) {
                const [head, tail] = Sequence.split(piece, room);
                groups.at(-1)!.push(head);
                piece = tail!;
            }
            groups.push([]);
            held = 0;
        }
        groups.at(-1)!.push(piece);
        held += characters(piece);
    }

    return groups;
}

/** Count the characters a piece stores. */
function characters(piece: Run): number {
    return typeof piece.text === "string" ? piece.text.length : 0;
}

/** Decide whether a piece holds an element. */
function holds(piece: Run, element: Element): boolean {
    return (
        piece.run === element.run &&
        element.offset >= piece.start &&
        element.offset < piece.start + Sequence.length(piece)
    );
}
