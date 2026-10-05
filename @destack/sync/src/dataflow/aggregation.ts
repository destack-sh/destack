import { Change, type ColumnValue, type Row, Condition, Predicate, type Scalar } from "@destack/db";
import { canonicalize, found, zip } from "@destack/schema";
import type { Node } from "../query/node.ts";
import { NOWHERE, Pipeline, keyed, without, type Context } from "./pipeline.ts";
import type { Run, Work } from "./run.ts";
import { PAGE_ROWS } from "./selection.ts";
import { Tally, type Result } from "./tally.ts";
import type { Group } from "./upstream.ts";

/** The most partitions one grouped read counts, bounded by the parameter budget. */
const COUNTED_PARTITIONS = 500;

/** The aggregate groups of a node's rows per partition with their measures in tallies. */
export class Aggregation extends Pipeline {
    /** The tallies, by group name. */
    readonly tallies = new Map<string, Tally>();
    /** What each tracked member adds, by key. */
    readonly contributions = new Map<string, readonly Contribution[]>();
    /** Whether the aggregate decides its rows one by one. */
    readonly isTracked: boolean;
    /** The groups the run changed, by name, with their results before the change. */
    readonly #changed = new Map<
        string,
        { readonly group: Record<string, Scalar>; readonly before: string }
    >();
    /** The groups that lost an extreme in the run, by name. */
    readonly #lost = new Set<string>();

    /** Create the aggregation of a node. */
    constructor(node: Node, context: Context, hasTreeIndex: boolean) {
        super(node, context, hasTreeIndex);
        this.isTracked =
            (node.path !== undefined && node.path.kind !== "key") ||
            node.relations.length > 0 ||
            node.isRelating;
    }

    /** The groups of the tallies. */
    get size(): number {
        return this.tallies.size;
    }

    /** Forget every group. */
    override forget(): void {
        // forget the partitions, members and groups
        super.forget();
        this.tallies.clear();
        this.contributions.clear();
        this.#changed.clear();
        this.#lost.clear();
    }

    /** Decide whether the subscriber is shown a group: every group of a query. */
    isShown(_group: Readonly<Record<string, Scalar>>): boolean {
        return true;
    }

    /** Empty closed partitions, count opened ones, and decide dirty rows again. */
    async step(work: Work, run: Run): Promise<void> {
        // empty the closed partitions
        for (const name of work.closed) {
            const partition = this.partitions.get(name);
            if (partition === undefined || partition.holders > 0) {
                continue;
            }
            for (const key of this.isTracked ? partition.members : []) {
                this.#assign(key, null, without(found(this.members, key).partitions, name), run);
            }
            if (!this.isTracked) {
                this.#release(partition.value, run);
            }
            this.partitions.delete(name);
        }

        // count the opened partitions
        const opened = [...work.opened].filter(([name]) => this.partitions.has(name));
        if (!this.isTracked) {
            await this.#countAll(opened, run);
        } else if (this.node.parent !== undefined) {
            const read = await this.matched(
                opened.map(([, value]) => value),
                run,
            );
            for (const [[name], rows] of zip(opened, read)) {
                for (const row of rows) {
                    const key = this.node.keyOf(row);
                    const partitions = this.members.get(key)?.partitions ?? NOWHERE;
                    this.#assign(key, row, new Set([...partitions, name]), run);
                }
            }
        } else if (opened.length > 0) {
            await this.pages(run, (rows) => this.decide(keyed(this.node, rows), run));
        }

        // decide the dirty rows again
        await this.decide(work.dirty, run);
        await this.regroup(run);
    }

    /** Count a linear aggregate's changes and send the moved groups. */
    async tally(run: Run): Promise<void> {
        await this.countImages(
            run,
            (value) => this.partitions.get(this.node.partition(value))?.sequence,
        );
        await this.regroup(run);
    }

    /** Count each change of the node's table out before and in after. */
    protected async countImages(
        run: Run,
        countedAt: (value: ColumnValue | undefined) => number | undefined,
    ): Promise<void> {
        // decide every image at once
        const node = this.node;
        const changes = run.changes.filter((change) => change.table === node.table);
        const affected = [...(run.affected.get(node.table)?.values() ?? [])];
        await this.filter.prepare(
            [
                ...changes.flatMap((change) => [Change.before(change), Change.after(change)]),
                ...affected,
            ].filter((image): image is Row => image !== null),
            run,
        );

        // count each candidate image out or in
        for (const change of changes) {
            for (const [image, sign] of [
                [Change.before(change), -1],
                [Change.after(change), 1],
            ] as const) {
                const value = image === null ? undefined : joinedOf(node, image);
                const sequence = image === null ? undefined : countedAt(value);
                if (
                    image === null ||
                    sequence === undefined ||
                    change.sequence <= sequence ||
                    !this.filter.isCandidate(image, run)
                ) {
                    continue;
                }
                const resolved = this.filter.resolve(image, run);
                const group = node.groupOf(resolved, value);
                const name = JSON.stringify(group);
                if (!this.tallies.has(name) && sign < 0) {
                    throw new TypeError(`tally of ${node.name} lost group ${name}`);
                }
                this.#note(name, group);
                const tally = this.#tally(name, group, sequence);
                if (change.sequence > tally.sequence && !tally.count(resolved, sign)) {
                    this.#lost.add(name);
                }
            }
        }

        // count again the groups of rows the audience decides again
        for (const row of affected) {
            const value = joinedOf(node, row);
            if (countedAt(value) !== undefined) {
                const group = node.groupOf(this.filter.resolve(row, run), value);
                const name = JSON.stringify(group);
                this.#note(name, group);
                this.#lost.add(name);
            }
        }
    }

    /** Move a tracked member's contributions from its old groups to its new ones. */
    protected count(key: string, next: readonly Contribution[], run: Run): void {
        // count the row out of its groups
        for (const entry of this.contributions.get(key) ?? []) {
            this.#note(entry.name, entry.group);
            if (!found(this.tallies, entry.name).count(entry.row, -1)) {
                this.#lost.add(entry.name);
            }
        }

        // count it into its new ones
        for (const entry of next) {
            this.#note(entry.name, entry.group);
            this.#tally(entry.name, entry.group, run.view.position.sequence).count(entry.row, 1);
        }
        if (next.length === 0) {
            this.contributions.delete(key);
        } else {
            this.contributions.set(key, next);
        }
    }

    /** Send and return the groups with results the run changed. */
    protected async regroup(run: Run): Promise<Record<string, Scalar>[]> {
        // take the changed groups
        const changed = [...this.#changed.values()];
        const lost = new Set(this.#lost);
        this.#changed.clear();
        this.#lost.clear();

        // count again the groups that lost an extreme and send the moved ones
        const moved: Record<string, Scalar>[] = [];
        for (const { group, before } of changed) {
            const name = JSON.stringify(group);
            if (lost.has(name)) {
                await this.#recount(group, run);
            }
            const tally = this.tallies.get(name);
            if (tally?.isEmpty === true) {
                this.tallies.delete(name);
            }
            if (signatureOf(tally) !== before) {
                this.result(group, tally, run);
                moved.push(group);
            }
        }

        return moved;
    }

    /** Forget the groups a hydration changed. */
    protected settleHydration(): void {
        this.#changed.clear();
        this.#lost.clear();
    }

    /** Send a shown group's values while it has rows, or its leaving. */
    protected result(
        group: Readonly<Record<string, Scalar>>,
        tally: Tally | undefined,
        run: Run,
    ): void {
        this.context.sink.result(run, this.node, group, tally, this.isShown(group));
    }

    /** Read the node's selection a page at a time, in order. */
    protected async pages(run: Run, each: (rows: readonly Row[]) => Promise<void>): Promise<void> {
        let after: Row | undefined;
        for (let isDone = false; !isDone;) {
            // read the next page
            const [rows] = await run.view.ordered(
                this.node,
                [after === undefined ? { value: null } : { value: null, after }],
                PAGE_ROWS,
                this.context.audience,
                run,
                this.node.relations.length === 0 ? undefined : this.relationView(run),
            );
            if (rows === undefined) {
                throw new RangeError("an ordered read lacks its one window");
            }
            isDone = rows.length < PAGE_ROWS;
            after = rows.at(-1);
            await each(rows);
        }
    }

    /** Admit a tracked member to its partitions. */
    protected admit(key: string, row: Row | null, names: ReadonlySet<string>, run: Run): void {
        this.#assign(key, row, names, run);
    }

    /** Set a tracked member's partitions and count it into their groups. */
    #assign(key: string, row: Row | null, names: ReadonlySet<string>, run: Run): void {
        // count the row into its groups
        const node = this.node;
        const member = this.move(key, names);
        this.count(
            key,
            row === null
                ? (this.contributions.get(key) ?? []).filter((entry) => names.has(entry.partition))
                : [...names].map((partition) =>
                      contributionOf(node, row, partition, found(this.partitions, partition).value),
                  ),
            run,
        );

        // keep the member in its partitions
        if (names.size === 0) {
            this.members.delete(key);
        } else if (member === undefined) {
            this.members.set(key, { partitions: names, joins: [] });
        } else {
            member.partitions = names;
        }
    }

    /** Note a group's result before the run first changes it. */
    #note(name: string, group: Record<string, Scalar>): void {
        if (!this.#changed.has(name)) {
            this.#changed.set(name, { group, before: signatureOf(this.tallies.get(name)) });
        }
    }

    /** Read a group's tally, starting an empty one at a sequence. */
    #tally(name: string, group: Record<string, Scalar>, sequence: number): Tally {
        let tally = this.tallies.get(name);
        if (tally === undefined) {
            tally = Tally.empty(this.node, group, sequence);
            this.tallies.set(name, tally);
        }

        return tally;
    }

    /** Count a linear aggregate's opened partitions, one grouped read per batch. */
    async #countAll(opened: readonly (readonly [string, ColumnValue])[], run: Run): Promise<void> {
        // build the conditions selecting the opened partitions
        const node = this.node;
        const path = node.path;
        const sequence = run.view.position.sequence;
        const batches: Condition[] = [];
        if (path?.kind === "key") {
            for (let start = 0; start < opened.length; start += COUNTED_PARTITIONS) {
                const batch = opened.slice(start, start + COUNTED_PARTITIONS);
                const values = batch.flatMap(([, value]) => {
                    const json = node.scalar(path.column, value);

                    return json === null ? [] : [json];
                });
                batches.push({ [path.column]: { in: values } });
            }
        } else if (opened.length > 0) {
            batches.push({});
        }

        // count each batch at its sequence
        for (const [name] of opened) {
            found(this.partitions, name).sequence = sequence;
        }
        for (const within of batches) {
            for (const [name, tally] of await this.tallyAt(within, run)) {
                this.tallies.set(name, tally);
                this.result(tally.group, tally, run);
            }
        }
    }

    /** Count one group again as of the run's position. */
    async #recount(group: Readonly<Record<string, Scalar>>, run: Run): Promise<void> {
        // measure the group in its partition
        const node = this.node;
        const name = JSON.stringify(group);
        const counted =
            node.path === undefined || node.path.kind === "key"
                ? await this.tallyAt(
                      Condition.equal(
                          Object.fromEntries(
                              Object.entries(group).filter(([column]) =>
                                  Object.hasOwn(node.columns, column),
                              ),
                          ),
                      ),
                      run,
                      group,
                  )
                : await this.tallyRows(run, group, {
                      value: node.parentValueOf(node.partitionOf(group)),
                  });
        this.tallies.set(
            name,
            counted.get(name) ?? Tally.empty(node, { ...group }, run.view.position.sequence),
        );
    }

    /** Measure the groups of the rows a condition selects, as of the run's position. */
    async tallyAt(
        within: Condition,
        run: Run,
        only?: Readonly<Record<string, Scalar>>,
    ): Promise<Map<string, Tally>> {
        // read the groups in one grouped read
        const node = this.node;
        const read = await run.view.measure(node, within, this.context.audience);
        const tallies = new Map(
            read.tallies
                .filter((tally) => only === undefined || sameGroup(tally.group, only))
                .map((tally) => [JSON.stringify(tally.group), tally]),
        );

        // undo the changes the read counted beyond the run's position, newest first
        const position = run.view.position.sequence;
        const beyond = await this.context.changesThrough(
            [{ table: node.table, scopes: node.scopes }],
            position,
            read.sequence,
        );
        if (beyond === undefined) {
            return this.tallyRows(run, only, { within });
        }
        const lost = await this.#undo(tallies, beyond, within, only, run);

        // recount the groups that lost an extreme
        await this.#recountLost(tallies, lost, within, run);
        for (const [name, tally] of tallies) {
            tally.sequence = position;
            if (tally.isEmpty) {
                tallies.delete(name);
            }
        }

        return tallies;
    }

    /** Count changed rows back out of or into their tallies and return the groups that lost an extreme. */
    async #undo(
        tallies: Map<string, Tally>,
        beyond: readonly Change[],
        within: Condition,
        only: Readonly<Record<string, Scalar>> | undefined,
        run: Run,
    ): Promise<Set<string>> {
        // prepare the matching images
        const node = this.node;
        const position = run.view.position.sequence;
        const match = Predicate.compile(node.resolve(within), node.table);
        const images = beyond.flatMap((change) =>
            [Change.after(change), Change.before(change)].filter(
                (image): image is Row => image !== null && Predicate.matches(match, image),
            ),
        );
        await this.filter.prepare(images, run);

        // count each image back from the newest change
        const lost = new Set<string>();
        for (const change of beyond.toReversed()) {
            for (const [image, sign] of [
                [Change.after(change), -1],
                [Change.before(change), 1],
            ] as const) {
                // count each row back in or out
                if (
                    image === null ||
                    !Predicate.matches(match, image) ||
                    !this.filter.isCandidate(image, run)
                ) {
                    continue;
                }
                const resolved = this.filter.resolve(image, run);
                const group = node.groupOf(resolved, joinedOf(node, image));
                if (only !== undefined && !sameGroup(group, only)) {
                    continue;
                }
                const name = JSON.stringify(group);
                const tally = tallies.get(name) ?? Tally.empty(node, group, position);
                tallies.set(name, tally);
                if (!tally.count(resolved, sign)) {
                    lost.add(name);
                }
            }
        }

        return lost;
    }

    /** Count the groups that lost an extreme again from their rows. */
    async #recountLost(
        tallies: Map<string, Tally>,
        lost: ReadonlySet<string>,
        within: Condition,
        run: Run,
    ): Promise<void> {
        const position = run.view.position.sequence;
        for (const name of lost) {
            const group = found(tallies, name).group;
            const exact = await this.tallyRows(run, group, { within });
            tallies.set(name, exact.get(name) ?? Tally.empty(this.node, group, position));
        }
    }

    /** Tally rows by group as of the run's position. */
    async tallyRows(
        run: Run,
        only: Readonly<Record<string, Scalar>> | undefined,
        selected: { readonly value: ColumnValue } | { readonly within: Condition },
    ): Promise<Map<string, Tally>> {
        // read the candidate rows
        const node = this.node;
        let rows: Row[];
        if ("value" in selected) {
            const [matched] = await this.matched([selected.value], run);
            if (matched === undefined) {
                throw new RangeError("a partition read lacks its one partition");
            }
            rows = matched;
        } else {
            const read = await run.view.matching(node.table, {
                AND: [node.condition(), selected.within],
            });
            await this.filter.prepare(read, run);
            rows = read
                .filter((row) => this.filter.isCandidate(row, run))
                .map((row) => this.filter.resolve(row, run));
        }

        // tally them by group
        const tallies = new Map<string, Tally>();
        for (const row of rows) {
            const group = node.groupOf(
                row,
                "value" in selected ? selected.value : joinedOf(node, row),
            );
            if (only !== undefined && !sameGroup(group, only)) {
                continue;
            }
            const name = JSON.stringify(group);
            const tally = tallies.get(name) ?? Tally.empty(node, group, run.view.position.sequence);
            tally.count(row, 1);
            tallies.set(name, tally);
        }

        return tallies;
    }

    /** Let go of the groups of a partition no selected row names. */
    #release(value: ColumnValue, run: Run): void {
        // let go of each group of the partition
        const node = this.node;
        const partition = node.partition(value);
        for (const [name, tally] of this.tallies) {
            if (JSON.stringify(node.partitionOf(tally.group)) === partition) {
                this.tallies.delete(name);
                this.result(tally.group, undefined, run);
            }
        }
    }
}

/** The measures of a relation the node's condition and computed values read, by parent value. */
export class Relation extends Aggregation {
    /** The log sequence a linear relation's count read at. */
    #sequence = -1;

    /** Forget every group. */
    override forget(): void {
        super.forget();
        this.#sequence = -1;
    }

    /** Decide whether the subscriber is shown a group: one a selected row of the parent names. */
    override isShown(group: Readonly<Record<string, Scalar>>): boolean {
        const node = this.node;

        return (
            node.link().parent.hasRows &&
            (this.partitions.get(JSON.stringify(node.partitionOf(group)))?.holders ?? 0) > 0
        );
    }

    /** Measure every group as of a hydration's position. */
    async hydrate(run: Run): Promise<void> {
        // count every group of a linear relation
        if (!this.isTracked) {
            for (const [name, tally] of await this.tallyAt({}, run)) {
                this.tallies.set(name, tally);
            }
            this.#sequence = run.view.position.sequence;

            return;
        }

        // decide every related row a page at a time
        await this.pages(run, (rows) => this.#measure(keyed(this.node, rows), run));
        this.settleHydration();
    }

    /** Apply a run's changes to every group, deciding the parent's naming rows again. */
    override async step(work: Work, run: Run): Promise<void> {
        // count the changes, or decide the touched rows again
        if (this.isTracked) {
            const dirty = new Map(work.dirty);
            work.dirty.clear();
            await this.#measure(dirty, run);
        } else {
            await this.countImages(run, () => this.#sequence);
        }

        // decide the parent's rows naming each moved group again
        const node = this.node;
        const link = node.link();
        const values = (await this.regroup(run))
            .map((group) => node.partitionOf(group))
            .filter((value) => value !== null)
            .map((value) => node.parentValueOf(value));
        const parent = this.requireParent();
        const holders = await run.view.lookup(
            parent.node.table,
            values.map((value) => ({ [link.parentColumn]: value })),
        );
        const dirty = run.workOf(parent).dirty;
        for (const row of holders.flat()) {
            dirty.set(parent.node.keyOf(row), row);
        }
    }

    /** Send the groups selected rows newly name, and let go of the others. */
    show(work: Work, run: Run): void {
        // let go of the groups no selected row names
        const node = this.node;
        for (const name of work.closed) {
            const partition = this.partitions.get(name);
            if (partition !== undefined && partition.holders === 0) {
                this.partitions.delete(name);
                this.result(node.groupFor(partition.value), undefined, run);
            }
        }

        // send the newly named groups
        for (const [name, value] of work.opened) {
            if (this.partitions.has(name)) {
                const group = node.groupFor(value);
                this.result(group, this.tallies.get(JSON.stringify(group)), run);
            }
        }
    }

    /** Decide related rows again and count each into its groups. */
    async #measure(rows: ReadonlyMap<string, Row | null>, run: Run): Promise<void> {
        // decide the rows and locate their holders
        const node = this.node;
        const present = [...rows.values()].filter((row): row is Row => row !== null);
        await this.filter.prepare(present, run);
        const candidates = present.filter((row) => this.filter.isCandidate(row, run));
        const located = await this.input.locate(candidates, run);
        const values = new Map(zip(candidates, located));

        // count each row out of its old groups and into its new ones
        for (const [key, row] of rows) {
            const named = row === null ? undefined : values.get(row);
            if (row === null || named === undefined) {
                this.count(key, [], run);
            } else {
                const resolved = this.filter.resolve(row, run);
                this.count(
                    key,
                    named.map((value) =>
                        contributionOf(node, resolved, node.partition(value), value),
                    ),
                    run,
                );
            }
        }
    }
}

/** The aggregate groups of a node as a copy's source measured them, with the copy's predictions. */
export class Mirror extends Pipeline {
    /** The shown groups by name, with their measures and canonical text. */
    readonly #shown = new Map<string, Group>();

    /** The groups shown. */
    get size(): number {
        return this.#shown.size;
    }

    /** Forget every group. */
    override forget(): void {
        super.forget();
        this.#shown.clear();
    }

    /** Show the groups of opened partitions and let go of closed ones. */
    async step(work: Work, run: Run): Promise<void> {
        for (const name of work.closed) {
            const partition = this.partitions.get(name);
            if (partition !== undefined && partition.holders === 0) {
                this.partitions.delete(name);
            }
        }
        await this.refresh(run);
    }

    /** Read the groups again and send the changed ones. */
    async refresh(run: Run): Promise<void> {
        // read the groups of the open partitions
        const node = this.node;
        const groups = new Map<string, Group>();
        const upstream = this.context.upstream;
        if (upstream === undefined) {
            throw new TypeError(`mirror of ${node.name} reads no upstream`);
        }
        for (const group of await upstream.groups(node)) {
            const partition =
                node.path === undefined ? "" : JSON.stringify(node.partitionOf(group.group));
            if (this.partitions.has(partition)) {
                groups.set(canonicalize(group.group), group);
            }
        }

        // let go of the gone groups and send the changed ones
        for (const [name, { group }] of this.#shown) {
            if (!groups.has(name)) {
                this.#shown.delete(name);
                this.context.sink.result(run, node, group, undefined);
            }
        }
        for (const [name, group] of groups) {
            const shown = this.#shown.get(name);
            if (shown === undefined || canonicalize(shown.values) !== canonicalize(group.values)) {
                this.#shown.set(name, group);
                this.context.sink.result(run, node, group.group, resultOf(group));
            }
        }
    }

    /** List the shown groups. */
    groups(): Group[] {
        return [...this.#shown.values()];
    }

    /** Admit no rows. */
    protected admit(): void {}
}

/** What one row adds to an aggregate in one partition. */
interface Contribution {
    /** The partition the row counts in. */
    readonly partition: string;
    /** The group the row counts in. */
    readonly group: Record<string, Scalar>;
    /** The group's name. */
    readonly name: string;
    /** The row with its computed values. */
    readonly row: Row;
}

/** Read a source's group as a result. */
function resultOf(group: Group): Result {
    return {
        rows: group.rows,
        isEmpty: group.rows === 0,
        values: () => group.values,
        parts: () => group.parts,
    };
}

/** Whether two groups have the same values. */
function sameGroup(
    left: Readonly<Record<string, Scalar>>,
    right: Readonly<Record<string, Scalar>>,
): boolean {
    return canonicalize(left) === canonicalize(right);
}

/** Write a group's result as text, empty once the group is empty. */
function signatureOf(tally: Tally | undefined): string {
    return tally === undefined || tally.isEmpty
        ? ""
        : JSON.stringify([tally.rows, tally.values(), tally.parts()]);
}

/** Build what a row adds to its group in one partition. */
function contributionOf(
    node: Node,
    row: Row,
    partition: string,
    value: ColumnValue | undefined,
): Contribution {
    const group = node.groupOf(row, value);

    return { partition, group, name: JSON.stringify(group), row };
}

/** Read the parent value a row of a root or key-joined node joins. */
function joinedOf(node: Node, row: Row): ColumnValue | undefined {
    return node.path?.kind === "key" ? row[node.path.column] : undefined;
}
