import type { ObjectReference } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { through } from "@destack/access";
import { Table } from "@destack/db";
import type { Transaction, Snapshot } from "@destack/db/database";
import { schema, canonicalize, Identifier, present } from "@destack/schema";

import { FractionalIndex } from "../field/fractional-index.ts";
import type { MethodContext } from "../method/context.ts";
import { Step } from "../method/step.ts";
import { method } from "../method/builder.ts";
import { type ObjectDefinition, ObjectType } from "../object/object.ts";
import {
    TextRope,
    TextRun,
    TextSequence,
    TextEdit,
    TextMark,
    type TextElement,
    type TextSchema,
    type TextSpan,
} from "../sequence/index.ts";
import { TextField } from "../field/field.ts";
import { counter } from "../trait/numbered.ts";
import { chunk, CHUNKS, runEntry } from "./table.ts";
import { Execution } from "../method/execution.ts";

import type {} from "@destack/package/import-meta";
import { argument, plural, t } from "@destack/locale";

/** The owner permission whose holders read its text. */
export const TEXT_READ = "text";

/** Segments text into the characters a person reads. */
const GRAPHEMES = new Intl.Segmenter(undefined, { granularity: "grapheme" });

/** One chunk of a text. */
type Chunk = typeof chunk.$inferSelect;

/** The input of an edit: the text field and its edits in order. */
const EDIT_INPUT = schema.looseObject({
    field: schema.string(),
    edits: schema.array(TextEdit),
});

/** The columns of an included chunk that assemble its owner's text. */
const INCLUDED_CHUNK = schema.looseObject({
    field: schema.string(),
    position: schema.string(),
    runs: schema.array(TextRun),
    marks: schema.array(TextMark).default([]),
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
    inverse: schema.array(TextEdit),
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
        elements: readonly TextElement[],
    ): Promise<boolean> {
        // find each element among its run's chunks
        const parent: Parent = {
            scope: owner.scope,
            parentPackageId: owner.packageId,
            parentType: owner.type,
            parentId: owner.id,
            field,
        };
        const entries = await seekEntries(view, parent, elements, new Map());
        const ids = [...new Set(entries.map((entry) => entry.row))];
        const rows =
            ids.length === 0
                ? []
                : await view.read(chunk).findMany({ where: { ...parent, id: { in: ids } } });
        const pieces = rows.flatMap((row) => row.runs);

        return elements.every((element) =>
            pieces.some((piece) => TextSequence.contains(piece, element)),
        );
    },

    /** Read the texts of some owners, by owner identifier. */
    async texts(
        transaction: Transaction,
        owner: ObjectType,
        scope: string,
        ids: readonly string[],
    ): Promise<Map<string, Record<string, TextSpan[]>>> {
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
    included(value: unknown): readonly Pick<Chunk, "field" | "position" | "runs" | "marks">[] {
        return INCLUDED_CHUNKS.parse(value ?? []);
    },

    /** Assemble an owner's texts from its chunks as spans, by field. */
    text(
        owner: ObjectType,
        rows: readonly Pick<Chunk, "field" | "position" | "runs" | "marks">[],
    ): Record<string, TextSpan[]> {
        return Object.fromEntries(
            owner.text.map((field) => [
                field,
                joinChunks(rows.filter((row) => row.field === field)).spans(
                    textSchema(owner, field),
                ),
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
        const rows = await readTouched(context, parent, edits);

        // order the marks at the home, a prediction leaving them last until it does
        const marked = edits.filter((edit) => "mark" in edit).length;
        const first =
            !execution.isPredicted && marked > 0
                ? await issueOrders(context, execution, parent, marked)
                : undefined;
        let issued = 0;
        const ordered = edits.map((edit): TextEdit => {
            if (!("mark" in edit)) {
                return edit;
            }
            const order = first === undefined ? null : first + issued;
            issued += 1;

            return { mark: { ...edit.mark, order } };
        });

        // apply each edit
        const allowed = textSchema(owner, field);
        const { sequence, inverse } = applyEdits(rows, ordered, field, allowed);

        // cut the text back into chunks and write them
        const chunks = await recut(context, parent, rows, sequence);
        await Execution.write(
            execution,
            context,
            ...chunks.map(({ before, after }) => ({ table: chunk, before, after })),
        );

        // tell each attached type observing a node type that the edit changed those nodes, as the editor, outside predictions
        const written = chunks.flatMap(({ after }) => (after === undefined ? [] : [after]));
        const observers = owner.attachments.flatMap(({ object }) => {
            const node = object.observes?.node;
            const isChanged =
                node !== undefined && !isSameSet(nodesOf(rows, node), nodesOf(written, node));

            return isChanged
                ? [object.record("observe", { parent: hostOf(parent), field }, context.scope)]
                : [];
        });
        if (!execution.isPredicted && observers.length > 0) {
            await context.mutate(context.caller, observers);
        }

        return { inverse };
    },

    /** Compose edits into the previous ones when they type or delete the next characters. */
    compose(previous: Step, next: Step): Step | undefined {
        // read both edits and the edits undoing them
        const earlier = EDIT_INPUT.parse(previous.input);
        const later = EDIT_INPUT.parse(next.input);
        if (previous.result === undefined || next.result === undefined) {
            return undefined;
        }
        const undoEarlier = Undo.parse(previous.result).inverse;
        const undoLater = Undo.parse(next.result).inverse;

        // compose edits continuing the last one in the same field of the same object
        const last = earlier.edits.at(-1);
        const isContinuing =
            last !== undefined &&
            earlier.field === later.field &&
            Step.same(Step.target(previous), Step.target(next)) &&
            continues(last, later.edits, undoLater);
        if (!isContinuing) {
            return undefined;
        }

        // apply both edits, and undo the later ones first
        const edits = schema.array(schema.json()).parse([...earlier.edits, ...later.edits]);
        const inverse = TextSequence.compact([...undoLater, ...undoEarlier]);

        return { ...next, input: { ...next.input, edits }, result: { inverse } };
    },
};

/** Decide whether edits type or delete one character each, right next to the last edit. */
function continues(last: TextEdit, edits: readonly TextEdit[], undo: readonly TextEdit[]): boolean {
    // type characters one after another, right after the last typed one
    if ("insert" in last && edits.length > 0 && edits.every((edit) => "insert" in edit)) {
        return [last, ...edits].every((edit, position, chain) => {
            const before = chain[position - 1];

            return (
                "insert" in edit &&
                typeof edit.insert === "string" &&
                isCharacter(edit.insert) &&
                (before === undefined ||
                    ("insert" in before &&
                        typeof before.insert === "string" &&
                        edit.run === before.run &&
                        edit.start === before.start + before.insert.length))
            );
        });
    }
    // delete pieces one after another, each touching the last deleted, one character each
    else if ("delete" in last && edits.length > 0 && edits.every((edit) => "delete" in edit)) {
        const pieces = edits.map((edit) => ("delete" in edit ? edit.delete : []));
        const isTouching = pieces.every((deleted, position) => {
            // touch the piece deleted just before
            const before = position === 0 ? last.delete : pieces[position - 1];
            const previous = before?.at(-1);
            const [piece, ...others] = deleted;

            return (
                previous !== undefined &&
                piece !== undefined &&
                others.length === 0 &&
                touches(previous, piece)
            );
        });

        return isTouching && restoredCharacters(undo) === edits.length;
    }
    // compose no other edits
    else {
        return false;
    }
}

/** Count the characters edits undoing deletions restore, absent when one restores no text. */
function restoredCharacters(undo: readonly TextEdit[]): number | undefined {
    let count = 0;
    for (const edit of undo) {
        // count each restored piece's characters
        if (!("restore" in edit)) {
            return undefined;
        }
        for (const piece of edit.restore) {
            if (!("text" in piece) || typeof piece.text !== "string") {
                return undefined;
            }
            count += [...GRAPHEMES.segment(piece.text)].length;
        }
    }

    return count;
}

/** Decide whether two pieces of one run touch, either before the other. */
function touches(left: TextRun, right: TextRun): boolean {
    return (
        left.run === right.run &&
        (left.start + TextSequence.length(left) === right.start ||
            right.start + TextSequence.length(right) === left.start)
    );
}

/** Decide whether text is one character as a person reads it. */
function isCharacter(text: string): boolean {
    const [first, second] = GRAPHEMES.segment(text);

    return first !== undefined && second === undefined;
}

/** A piece's run index entry: its chunk, run and first offset. */
const RUN_ENTRY = schema.looseObject({
    /** The chunk holding the piece. */
    row: schema.identifier("chunk"),
    /** The piece's run. */
    run: schema.string(),
    /** The piece's first offset in its run. */
    start: schema.number().int(),
});
/** A piece's entry in the run index. */
type RunEntry = schema.Infer<typeof RUN_ENTRY>;

/** The elements of one run an edit inserts: from its start to before its end. */
interface Segment {
    /** The offset of the first inserted element in the run. */
    readonly start: number;
    /** The offset after the last inserted element in the run. */
    readonly end: number;
}

/** A chunk an edit writes: as read, null for a new chunk, and as written. */
interface ChunkChange {
    /** The chunk as read, null for a new chunk. */
    readonly before: Chunk | null;
    /** The chunk as written. */
    readonly after: Chunk;
}

/** Read a text field's chunks in position order, within positions when given. */
async function readChunks(
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

    return orderChunks(rows);
}

/** Read the contiguous chunks containing the elements some edits refer to. */
async function readTouched(
    context: MethodContext,
    parent: Parent,
    edits: readonly TextEdit[],
): Promise<Chunk[]> {
    // find the chunks holding the edited elements
    const { named, inserted, isAtStart } = editedElements(edits);
    const entries = await seekEntries(context, parent, named, inserted);
    const ids = holders(entries, named, inserted, parent.field);

    // read the chunks with those runs, and the field's first chunk for an insertion at the start
    const rows =
        ids.length === 0
            ? []
            : await context.read(chunk).findMany({ where: { ...parent, id: { in: ids } } });
    const first = isAtStart ? await readFirst(context, parent) : [];
    const found = orderChunks([...rows, ...first.filter((row) => !ids.includes(row.id))]);

    // take one chunk as it is, else read every chunk from the first touched to the last
    const [from] = found;
    const to = found.at(-1);

    return found.length <= 1 || from === undefined || to === undefined
        ? found
        : readChunks(context, parent, { from: from.position, to: to.position });
}

/** Read the latest run entry at or before each element and each inserted segment's end. */
async function seekEntries(
    view: Pick<Snapshot, "read">,
    parent: Parent,
    named: readonly TextElement[],
    inserted: ReadonlyMap<string, Segment>,
): Promise<RunEntry[]> {
    // seek each segment's end, covering the element right before it
    const seeks = new Map<string, TextElement>();
    for (const [run, segment] of inserted) {
        seeks.set(run, { run, offset: segment.end - 1 });
    }
    for (const element of named) {
        const segment = inserted.get(element.run);
        if (segment === undefined || element.offset !== segment.start - 1) {
            seeks.set(`${element.run} ${element.offset}`, element);
        }
    }

    // read each seek's latest entry through the run index
    const found = await Promise.all(
        [...seeks.values()].map((element) =>
            view.read(runEntry).findMany({
                where: {
                    parentId: parent.parentId,
                    field: parent.field,
                    run: element.run,
                    start: { lte: element.offset },
                },
                orderBy: { start: "desc" },
                limit: 1,
            }),
        ),
    );

    return found.flat().map((row) => RUN_ENTRY.parse(row));
}

/** Choose the chunks holding some elements. */
function holders(
    entries: readonly RunEntry[],
    named: readonly TextElement[],
    inserted: ReadonlyMap<string, Segment>,
    field: string,
): Identifier<"chunk">[] {
    // take the piece holding each element
    const latest = (run: string, offset: number) =>
        entries
            .filter((entry) => entry.run === run && entry.start <= offset)
            .reduce<RunEntry | undefined>(
                (best, entry) => (best === undefined || entry.start > best.start ? entry : best),
                undefined,
            );
    const held = named.map((element) => {
        const holder = latest(element.run, element.offset);
        if (holder === undefined) {
            throw ObjectError.of(
                "ELEMENT_INVALID",
                `element '${element.run}' at offset ${element.offset} is invalid: no chunk has it`,
            );
        }

        return holder.row;
    });

    // refuse repeated elements and take each continued run's end
    const ends = [...inserted].flatMap(([run, segment]) => {
        // refuse an entry of the run inside the inserted segment
        const repeated = entries.find(
            (entry) =>
                entry.run === run && entry.start >= segment.start && entry.start < segment.end,
        );
        if (repeated !== undefined) {
            throw ObjectError.of(
                "RUN_INVALID",
                `run '${run}' is invalid: '${field}' already has its elements from ${segment.start}`,
            );
        }
        const end = segment.start === 0 ? undefined : latest(run, segment.start - 1);

        return end === undefined ? [] : [end.row];
    });

    return [...new Set([...held, ...ends])];
}

/** Collect the elements some edits refer to, without inserted runs, and any start insertion. */
function editedElements(edits: readonly TextEdit[]): {
    readonly named: TextElement[];
    readonly inserted: Map<string, Segment>;
    readonly isAtStart: boolean;
} {
    // walk the edits in order
    const inserted = new Map<string, Segment>();
    const named: TextElement[] = [];
    let isAtStart = false;
    for (const edit of edits) {
        // collect the elements of runs inserted before
        const elements = TextSequence.elements(edit);
        named.push(
            ...elements.filter((element) => {
                const segment = inserted.get(element.run);

                return segment === undefined || element.offset < segment.start;
            }),
        );

        // remember the inserted run, and an edit at the text's start
        isAtStart ||= TextSequence.isAtStart(edit);
        if ("insert" in edit) {
            const start = edit.start;
            const length = typeof edit.insert === "string" ? edit.insert.length : 1;
            const before = inserted.get(edit.run);
            inserted.set(edit.run, {
                start: Math.min(before?.start ?? start, start),
                end: Math.max(before?.end ?? 0, start + length),
            });
        }
    }

    return { named, inserted, isAtStart };
}

/** Read a text field's first chunk, none for an empty field. */
async function readFirst(context: MethodContext, parent: Parent): Promise<Chunk[]> {
    return context
        .read(chunk)
        .findMany({ where: { ...parent }, orderBy: { position: "asc" }, limit: 1 });
}

/** Apply edits to a text, collecting the edits inverting them in reverse order. */
function applyEdits(
    rows: readonly Chunk[],
    edits: readonly TextEdit[],
    field: string,
    allowed: TextSchema,
): { readonly sequence: TextSequence; readonly inverse: TextEdit[] } {
    // start from the text before
    let sequence = joinChunks(rows);
    const inverses: TextEdit[][] = [];
    for (const edit of edits) {
        // refuse repeated elements and gaps in a run
        if ("insert" in edit && sequence.isRepeating(edit)) {
            throw ObjectError.of(
                "RUN_INVALID",
                `run '${edit.run}' is invalid: '${field}' already has its elements from ${edit.start}`,
            );
        } else if ("insert" in edit && !sequence.isContinuing(edit)) {
            throw ObjectError.of(
                "RUN_INVALID",
                `run '${edit.run}' is invalid: it continues at ${edit.start}, where none of its elements ends`,
            );
        }

        // check, invert and apply the edit
        try {
            allowed.require(edit);
            inverses.unshift(TextSequence.inverse(edit, sequence));
            sequence = sequence.apply(edit);
        } catch (error) {
            if (error instanceof RangeError) {
                throw ObjectError.of("TEXT_EDIT_INVALID", error.message);
            }
            throw error;
        }
    }

    return { sequence, inverse: inverses.flat() };
}

/** Issue a count of mark orders for one text field from its counter, returning the first. */
async function issueOrders(
    context: MethodContext,
    execution: Execution,
    parent: Parent,
    count: number,
): Promise<number> {
    // count within the field
    const key = {
        scope: parent.scope,
        packageId: parent.parentPackageId,
        type: parent.parentType,
        within: `${parent.parentId}/${parent.field}`,
    };
    const [before] = await context.read(counter).findMany({ where: key });
    const first = (before?.last ?? 0) + 1;
    await Execution.write(execution, context, {
        table: counter,
        before: before ?? null,
        after: { ...key, last: first + count - 1 },
    });

    return first;
}

/** List the values of a type's nodes some chunks hold, each once in canonical form. */
function nodesOf(rows: readonly Pick<Chunk, "runs">[], type: string): ReadonlySet<string> {
    return new Set(
        rows.flatMap((row) =>
            row.runs.flatMap((run) =>
                "node" in run && run.node.type === type ? [canonicalize(run.node.value)] : [],
            ),
        ),
    );
}

/** Report whether two sets hold the same values. */
function isSameSet(one: ReadonlySet<string>, other: ReadonlySet<string>): boolean {
    return one.size === other.size && [...one].every((value) => other.has(value));
}

/** Read the reference of a text's owner, as a nested object names its parent. */
function hostOf(parent: Parent): {
    readonly packageId: string;
    readonly type: string;
    readonly id: string;
} {
    return { packageId: parent.parentPackageId, type: parent.parentType, id: parent.parentId };
}

/** Assemble a text's sequence from its chunks in position order, with their marks. */
function joinChunks(rows: readonly Pick<Chunk, "position" | "runs" | "marks">[]): TextSequence {
    const ordered = orderChunks(rows);

    return new TextSequence(
        ordered.flatMap((row) => row.runs),
        ordered.flatMap((row) => row.marks),
    );
}

/** Read the schema of an owner's text field. */
function textSchema(owner: ObjectType, field: string): TextSchema {
    const declared = owner.fields[field];
    if (!(declared instanceof TextField)) {
        throw new TypeError(`object '${owner.name}' has no text field '${field}'`);
    }

    return declared.schema;
}

/** Cut an edited text back into chunk rows. */
async function recut(
    context: MethodContext,
    parent: Parent,
    rows: readonly Chunk[],
    edited: TextSequence,
): Promise<ChunkChange[]> {
    // lay the edited text out in the rows' sections
    const sections = TextRope.layout(
        edited,
        rows.slice(1).map((row) => chunkStart(row)),
    );
    const laid: ChunkChange[] = [];
    for (const [index, section] of sections.entries()) {
        // take an existing chunk's first group
        const current = rows[index];
        const [first, ...rest] = section;
        if (current !== undefined) {
            laid.push({
                before: current,
                after: { ...current, ...chunkColumns(first) },
            });
        }

        // place new chunks between this one and the next
        const groups = current === undefined ? [first, ...rest] : rest;
        let before = current?.position;
        const isLast = index + 1 >= rows.length && !context.isPredicted;
        let after: string | undefined;
        if (groups.length > 0) {
            after = isLast
                ? await nextPosition(context, parent, current?.position)
                : rows[index + 1]?.position;
        }
        for (const group of groups) {
            before = FractionalIndex.between(before, after);
            const created = Table.complete(chunk, {
                ...parent,
                id: Identifier.create("chunk", context.environment),
                position: before,
                ...chunkColumns(group),
            });
            laid.push({ before: null, after: created });
        }
    }

    // return the new and changed chunks
    return laid.filter(
        ({ before, after }) =>
            before === null ||
            canonicalize(before.runs) !== canonicalize(after.runs) ||
            canonicalize(before.marks) !== canonicalize(after.marks),
    );
}

/** Read the position of the chunk after one, absent after the last. */
async function nextPosition(
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
}

/** Order chunks by position, comparing bytes. */
function orderChunks<Row extends Pick<Chunk, "position">>(rows: readonly Row[]): Row[] {
    return rows.toSorted((left, right) => (left.position < right.position ? -1 : 1));
}

/** Read the stored columns of a chunk's sequence. */
function chunkColumns(sequence: TextSequence): Pick<Chunk, "runs" | "marks" | "text"> {
    return { runs: [...sequence.runs], marks: [...sequence.marks], text: sequence.text() };
}

/** Read the element a chunk starts at. */
function chunkStart(row: Chunk): TextElement {
    const first = present(row.runs[0], `the first run of chunk ${row.id}`);

    return { run: first.run, offset: first.start };
}
