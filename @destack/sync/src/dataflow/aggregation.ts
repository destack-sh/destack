import type { Row } from "@destack/db";
import { Condition, type Scalar } from "@destack/db/query";
import { canonicalize } from "@destack/schema/json";
import type { Node } from "../query/node.ts";
import { NOWHERE, Pipeline, keyed, without, type Context } from "./pipeline.ts";
import type { Run, Work } from "./run.ts";
import { PAGE_ROWS } from "./selection.ts";
import { Tally, type Result } from "./tally.ts";
import type { Group } from "./upstream.ts";

/** The partitions one grouped read counts at most, an OR chain well within the parameter budget and a condition's terms. */
const COUNTED_PARTITIONS = 500;

/**
 * The aggregate groups of a node's rows per partition, each group's measures in a tally.
 *
 * A linear aggregate counts each change's images out and in, keeping nothing per row.
 * A tracked one, reached through a junction or tree path or reading relations, decides its rows one by one and keeps what each adds.
 */
export class Aggregation extends Pipeline {
    /** The tallies, by group name. */
    readonly tallies = new Map<string, Tally>();
    /** What each tracked member adds, by key. */
    readonly contributions = new Map<string, readonly Contribution[]>();
    /** Whether the aggregate decides its rows one by one. */
    readonly isTracked: boolean;
    /** The groups the run changed, by name, with their results before the run first changed them. */
    readonly #changed = new Map<
        string,
        { readonly group: Record<string, Scalar>; readonly before: string }
    >();
    /** The groups that lost an extreme in the run, which only a count restores, by name. */
    readonly #lost = new Set<string>();

    /** Measure a node's groups within a dataflow. */
    constructor(node: Node, context: Context, hasTreeIndex: boolean) {
        super(node, context, hasTreeIndex);
        this.isTracked =
            (node.path !== undefined && node.path.kind !== "key") ||
            node.relations.length > 0 ||
            node.isRelating;
    }

    /** The groups the tallies hold. */
    get size(): number {
        return this.tallies.size;
    }

    /** Forget every group. */
    forget(): void {
        // forget the partitions and members, then the groups
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

    /** Empty the closed partitions, count the opened ones, and decide a tracked aggregate's dirty rows again. */
    async step(work: Work, run: Run): Promise<void> {
        // empty and forget the closed partitions
        for (const name of work.closed) {
            const partition = this.partitions.get(name);
            if (partition === undefined || partition.holders > 0) {
                continue;
            }
            for (const key of this.isTracked ? partition.members : []) {
                this.#assign(key, undefined, without(this.members.get(key)!.partitions, name), run);
            }
            if (!this.isTracked) {
                this.#release(partition.value, run);
            }
            this.partitions.delete(name);
        }

        // count the opened partitions in the database, or decide a tracked aggregate's rows in them
        const opened = [...work.opened].filter(([name]) => this.partitions.has(name));
        if (!this.isTracked) {
            await this.#countAll(opened, run);
        } else if (this.node.parent !== undefined) {
            const read = await this.matched(
                opened.map(([, value]) => value),
                run,
            );
            for (const [index, [name]] of opened.entries()) {
                for (const row of read[index]!) {
                    const key = this.node.keyOf(row);
                    const partitions = this.members.get(key)?.partitions ?? NOWHERE;
                    this.#assign(key, row, new Set([...partitions, name]), run);
                }
            }
        } else if (opened.length > 0) {
            await this.pages(run, (rows) => this.decide(keyed(this.node, rows), run));
        }

        // decide the dirty rows again, and send the groups they moved
        await this.decide(work.dirty, run);
        await this.regroup(run);
    }

    /** Count the changes of a linear aggregate in the partitions counted before them, and send the groups they moved. */
    async tally(run: Run): Promise<void> {
        await this.countImages(
            run,
            (value) => this.partitions.get(this.node.partition(value))?.sequence,
        );
        await this.regroup(run);
    }

    /**
     * Count each change of the node's table out as it was and in as it is, from the sequence the row's partition was counted at.
     *
     * Groups of rows the audience decides again count again, since access decides them as of now.
     */
    protected async countImages(
        run: Run,
        countedAt: (value: unknown) => number | undefined,
    ): Promise<void> {
        // decide every image of the node's table at once
        const node = this.node;
        const changes = run.changes.filter((change) => change.table === node.table);
        const affected = [...(run.affected.get(node.table)?.values() ?? [])];
        await this.filter.prepare(
            [...changes.flatMap((change) => [change.before, change.after]), ...affected].filter(
                (image): image is Row => image !== undefined,
            ),
            run,
        );

        // count each candidate image out or in, into a partition counted before the change
        for (const change of changes) {
            for (const [image, sign] of [
                [change.before, -1],
                [change.after, 1],
            ] as const) {
                const value = image === undefined ? undefined : joinedOf(node, image);
                const sequence = image === undefined ? undefined : countedAt(value);
                if (
                    image === undefined ||
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

    /**
     * Move a tracked member's contributions: out of the groups it counted in before, into its new ones.
     *
     * A row taking an extreme out of its group leaves the group to count again.
     */
    protected count(key: string, next: readonly Contribution[], run: Run): void {
        // count the row out of its groups
        for (const entry of this.contributions.get(key) ?? []) {
            this.#note(entry.name, entry.group);
            if (!this.tallies.get(entry.name)!.count(entry.row, -1)) {
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

    /**
     * Send the groups whose results the run changed, counting again those that lost an extreme and letting go of empty ones.
     *
     * Returns the groups sent, whose measures readers of the aggregate see change.
     */
    protected async regroup(run: Run): Promise<Record<string, Scalar>[]> {
        // take the run's changed groups
        const changed = [...this.#changed.values()];
        const lost = new Set(this.#lost);
        this.#changed.clear();
        this.#lost.clear();

        // count again the groups that lost an extreme, and send those whose results moved
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

    /** Forget the groups a hydration changed, since it sends every group it shows by itself. */
    protected settleHydration(): void {
        this.#changed.clear();
        this.#lost.clear();
    }

    /** Send a group's values while it holds rows and the subscriber is shown it, or its leaving. */
    protected result(
        group: Readonly<Record<string, Scalar>>,
        tally: Tally | undefined,
        run: Run,
    ): void {
        this.context.sink.result(run, this.node, group, tally, this.isShown(group));
    }

    /** Read every row of the node's selection a page at a time, in its order, handing each page on. */
    protected async pages(run: Run, each: (rows: Row[]) => Promise<void>): Promise<void> {
        let after: Row | undefined;
        for (let isDone = false; !isDone;) {
            // read the next page of the whole selection, whatever the path
            const [rows] = await run.view.ordered(
                this.node,
                [after === undefined ? {} : { after }],
                PAGE_ROWS,
                this.context.audience,
                run,
                this.node.relations.length === 0 ? undefined : this.relationView(run),
            );
            isDone = rows!.length < PAGE_ROWS;
            after = rows!.at(-1);
            await each(rows!);
        }
    }

    /** Admit a tracked member to the partitions holding it, counting it into their groups. */
    protected admit(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        this.#assign(key, row, names, run);
    }

    /** Set the partitions holding a tracked member, counting it into the group of each, or keeping what it added in those it stays in. */
    #assign(key: string, row: Row | undefined, names: ReadonlySet<string>, run: Run): void {
        // count the row into its groups
        const node = this.node;
        const member = this.move(key, names);
        this.count(
            key,
            row === undefined
                ? (this.contributions.get(key) ?? []).filter((entry) => names.has(entry.partition))
                : [...names].map((partition) =>
                      contributionOf(node, row, partition, this.partitions.get(partition)!.value),
                  ),
            run,
        );

        // hold the member in its partitions
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

    /** Read a group's tally, starting an empty one counted at a sequence. */
    #tally(name: string, group: Record<string, Scalar>, sequence: number): Tally {
        let tally = this.tallies.get(name);
        if (tally === undefined) {
            tally = Tally.empty(this.node, group, sequence);
            this.tallies.set(name, tally);
        }

        return tally;
    }

    /** Count a linear aggregate's opened partitions: those keys select in one grouped read per batch, a root's alone. */
    async #countAll(opened: readonly (readonly [string, unknown])[], run: Run): Promise<void> {
        // name the conditions selecting the opened partitions: a batch of keyed ones per read, or a root's every row
        const node = this.node;
        const path = node.path;
        const sequence = run.view.position.sequence;
        const batches: Condition[] = [];
        if (path?.kind === "key") {
            for (let start = 0; start < opened.length; start += COUNTED_PARTITIONS) {
                const batch = opened.slice(start, start + COUNTED_PARTITIONS);
                batches.push(
                    Condition.oneOf(
                        path.column,
                        batch.map(
                            ([, value]) => node.json(path.column, value) as Exclude<Scalar, null>,
                        ),
                    ),
                );
            }
        } else if (opened.length > 0) {
            batches.push(Condition.all());
        }

        // count each batch, noting the sequence later changes count from
        for (const [name] of opened) {
            this.partitions.get(name)!.sequence = sequence;
        }
        for (const within of batches) {
            for (const [name, tally] of await this.tallyAt(within, run)) {
                this.tallies.set(name, tally);
                this.result(tally.group, tally, run);
            }
        }
    }

    /** Count one group again as of the run's position, keeping it empty when no row holds it any more. */
    async #recount(group: Readonly<Record<string, Scalar>>, run: Run): Promise<void> {
        // measure the group in its partition: in the database for a root or key, else through its path
        const node = this.node;
        const name = JSON.stringify(group);
        const counted =
            node.path === undefined || node.path.kind === "key"
                ? await this.tallyAt(
                      Condition.all(
                          ...Object.entries(group)
                              .filter(([column]) => Object.hasOwn(node.columns, column))
                              .map(([column, entry]) =>
                                  entry === null
                                      ? Condition.missing(column)
                                      : Condition.eq(column, entry),
                              ),
                      ),
                      run,
                      group,
                  )
                : await this.tallyRows(run, group, {
                      value: node.parent!.fromJson(node.parentColumn!, group[node.partitionName!]!),
                  });
        this.tallies.set(
            name,
            counted.get(name) ?? Tally.empty(node, { ...group }, run.view.position.sequence),
        );
    }

    /**
     * Measure the groups among the rows a condition over columns selects, as of the run's position.
     *
     * One grouped read counts the rows as of its own later position; the changes it counted beyond the run's are undone, and groups whose extremes they moved are counted from their rows.
     */
    async tallyAt(
        within: Condition,
        run: Run,
        only?: Readonly<Record<string, Scalar>>,
    ): Promise<Map<string, Tally>> {
        // read the groups in one grouped read, as of the position it reads at
        const node = this.node;
        const read = await run.view.measure(node, within, this.context.audience);
        const tallies = new Map(
            read.tallies
                .filter((tally) => only === undefined || sameGroup(tally.group, only))
                .map((tally) => [JSON.stringify(tally.group), tally]),
        );

        // undo the changes the read counted beyond the run's position, newest first, counting each row out as it is and in as it was
        const position = run.view.position.sequence;
        const beyond = await this.context.changesThrough(
            [{ table: node.table, scopes: node.scopes }],
            position,
            read.sequence,
        );
        if (beyond === undefined) {
            return this.tallyRows(run, only, { within });
        }
        const match = Condition.compile(within, node.table);
        const images = beyond.flatMap((change) =>
            [change.after, change.before].filter(
                (image): image is Row => image !== undefined && Condition.matches(match, image),
            ),
        );
        await this.filter.prepare(images, run);
        const lost = new Set<string>();
        for (const change of beyond.toReversed()) {
            for (const [image, sign] of [
                [change.after, -1],
                [change.before, 1],
            ] as const) {
                // count back in or out each row the read counted
                if (
                    image === undefined ||
                    !Condition.matches(match, image) ||
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

        // count the groups that lost an extreme from their rows, and hold every group as of the run's position
        for (const name of lost) {
            const group = tallies.get(name)!.group;
            const exact = await this.tallyRows(run, group, { within });
            tallies.set(name, exact.get(name) ?? Tally.empty(node, group, position));
        }
        for (const [name, tally] of tallies) {
            tally.sequence = position;
            if (tally.isEmpty) {
                tallies.delete(name);
            }
        }

        return tallies;
    }

    /** Tally rows by group as of the run's position: a partition's through its path, or those a condition over columns selects. */
    async tallyRows(
        run: Run,
        only: Readonly<Record<string, Scalar>> | undefined,
        selected: { readonly value: unknown } | { readonly within: Condition },
    ): Promise<Map<string, Tally>> {
        // read the rows as of the position, keeping the candidates
        const node = this.node;
        let rows: Row[];
        if ("value" in selected) {
            [rows] = (await this.matched([selected.value], run)) as [Row[]];
        } else {
            const read = await run.view.matching(
                node.table,
                Condition.all(node.condition(), selected.within),
            );
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

    /** Let go of the groups of a partition no held row names any more. */
    #release(value: unknown, run: Run): void {
        // let go of each group of the partition
        const node = this.node;
        const partition = JSON.stringify(node.parent!.json(node.parentColumn!, value));
        for (const name of this.tallies.keys()) {
            const group = JSON.parse(name) as Record<string, Scalar>;
            if (JSON.stringify(group[node.partitionName!]) === partition) {
                this.tallies.delete(name);
                this.result(group, undefined, run);
            }
        }
    }
}

/**
 * The measures of a relation the node's condition and computed values read: every group of its rows by the held value naming them.
 *
 * It counts every group of its scopes, since any row may be a candidate, and shows the subscriber the groups its held rows name.
 * Groups whose measures a run changes decide the rows of the parent node naming them again.
 */
export class Relation extends Aggregation {
    /** The log sequence a linear relation's count of every group read at, which later changes continue. */
    #sequence = -1;

    /** Forget every group. */
    forget(): void {
        super.forget();
        this.#sequence = -1;
    }

    /** Decide whether the subscriber is shown a group: one a held row of the parent names. */
    isShown(group: Readonly<Record<string, Scalar>>): boolean {
        const node = this.node;

        return (
            node.parent!.isHolding &&
            (this.partitions.get(JSON.stringify(group[node.partitionName!]))?.holders ?? 0) > 0
        );
    }

    /** Measure every group as of a hydration's position: in one grouped read, or row by row when tracked. */
    async hydrate(run: Run): Promise<void> {
        // count every group of a linear relation, noting the position later changes count from
        if (!this.isTracked) {
            for (const [name, tally] of await this.tallyAt(Condition.all(), run)) {
                this.tallies.set(name, tally);
            }
            this.#sequence = run.view.position.sequence;

            return;
        }

        // decide every related row a page at a time, then start the first step with no group changed
        await this.pages(run, (rows) => this.#measure(keyed(this.node, rows), run));
        this.settleHydration();
    }

    /**
     * Apply a run's changes to every group, deciding again the parent's rows naming the groups whose measures changed.
     *
     * A linear relation counts each change's row out and in; a tracked one decides the rows the run touched again.
     */
    async step(work: Work, run: Run): Promise<void> {
        // count the run's changes, or decide the touched rows again
        if (this.isTracked) {
            const dirty = new Map(work.dirty);
            work.dirty.clear();
            await this.#measure(dirty, run);
        } else {
            await this.countImages(run, () => this.#sequence);
        }

        // decide again the parent's rows naming each group whose measures moved, read at once
        const node = this.node;
        const values = (await this.regroup(run))
            .map((group) => group[node.partitionName!])
            .filter((value) => value !== null && value !== undefined)
            .map((value) => node.parent!.fromJson(node.parentColumn!, value!));
        const parent = this.parent!;
        const holders = await run.view.lookup(
            parent.node.table,
            values.map((value) => ({ [node.parentColumn!]: value })),
        );
        const dirty = run.workOf(parent).dirty;
        for (const row of holders.flat()) {
            dirty.set(parent.node.keyOf(row), row);
        }
    }

    /** Send the groups held rows newly name, and let go of those they name no more. */
    show(work: Work, run: Run): void {
        // let go of the groups no held row names
        const node = this.node;
        for (const name of work.closed) {
            const partition = this.partitions.get(name);
            if (partition !== undefined && partition.holders === 0) {
                this.partitions.delete(name);
                this.result(node.groupFor(partition.value), undefined, run);
            }
        }

        // send the groups held rows newly name
        for (const [name, value] of work.opened) {
            if (this.partitions.has(name)) {
                const group = node.groupFor(value);
                this.result(group, this.tallies.get(JSON.stringify(group)), run);
            }
        }
    }

    /** Decide related rows again as they are at the position, counting each into the group of every value naming its holders. */
    async #measure(rows: ReadonlyMap<string, Row | undefined>, run: Run): Promise<void> {
        // decide the rows and locate the candidates' holders at once
        const node = this.node;
        const present = [...rows.values()].filter((row): row is Row => row !== undefined);
        await this.filter.prepare(present, run);
        const candidates = present.filter((row) => this.filter.isCandidate(row, run));
        const located = await this.input.locate(candidates, run);
        const values = new Map(candidates.map((row, index) => [row, located[index]!]));

        // count each row out of its groups and into those of the values naming it now
        for (const [key, row] of rows) {
            const named = row === undefined ? undefined : values.get(row);
            const resolved = named === undefined ? undefined : this.filter.resolve(row!, run);
            this.count(
                key,
                (named ?? []).map((value) =>
                    contributionOf(node, resolved!, node.partition(value), value),
                ),
                run,
            );
        }
    }
}

/**
 * The aggregate groups of a node as a copy's source measured them, with the copy's predictions, which a copy reads instead of counting its own rows.
 *
 * It reads the groups again whenever a run changes the node's table or the source's groups of the node.
 */
export class Mirror extends Pipeline {
    /** The groups the subscriber is shown, by name, with their measures and those measures' canonical text. */
    readonly #shown = new Map<string, Group>();

    /** The groups shown. */
    get size(): number {
        return this.#shown.size;
    }

    /** Forget every group. */
    forget(): void {
        super.forget();
        this.#shown.clear();
    }

    /** Show the groups of opened partitions, and let go of those of closed ones. */
    async step(work: Work, run: Run): Promise<void> {
        for (const name of work.closed) {
            const partition = this.partitions.get(name);
            if (partition !== undefined && partition.holders === 0) {
                this.partitions.delete(name);
            }
        }
        await this.refresh(run);
    }

    /** Read the groups again, sending those whose measures changed and letting go of those gone or no longer held. */
    async refresh(run: Run): Promise<void> {
        // read the groups of the open partitions
        const node = this.node;
        const groups = new Map<string, Group>();
        for (const group of await this.context.upstream!.groups(node)) {
            const partition =
                node.path === undefined ? "" : JSON.stringify(group.group[node.partitionName!]);
            if (this.partitions.has(partition)) {
                groups.set(canonicalize(group.group), group);
            }
        }

        // let go of the groups no longer held, then send the changed ones
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

    /** List the groups shown. */
    groups(): Group[] {
        return [...this.#shown.values()];
    }

    /** Admit no rows, since the source measures them. */
    protected admit(): void {}
}

/** What one row adds to an aggregate in one partition: its group there, and its measured values. */
interface Contribution {
    /** The partition the row counts in. */
    readonly partition: string;
    /** The group the row counts in. */
    readonly group: Record<string, Scalar>;
    /** The group's name, which keys its tally. */
    readonly name: string;
    /** The row with its computed values, whose values the group measures. */
    readonly row: Row;
}

/** Read a source's group as the result a page sends. */
function resultOf(group: Group): Result {
    return {
        rows: group.rows,
        isEmpty: group.rows === 0,
        values: () => group.values,
        parts: () => group.parts,
    };
}

/** Whether two groups hold the same values. */
function sameGroup(
    left: Readonly<Record<string, Scalar>>,
    right: Readonly<Record<string, Scalar>>,
): boolean {
    return canonicalize(left) === canonicalize(right);
}

/** Write what a group's result carries as text, which tells whether a run changed it: nothing once empty. */
function signatureOf(tally: Tally | undefined): string {
    return tally === undefined || tally.isEmpty
        ? ""
        : JSON.stringify([tally.rows, tally.values(), tally.parts()]);
}

/** Name what a row adds to its group in one partition, the held value naming the partition. */
function contributionOf(node: Node, row: Row, partition: string, value: unknown): Contribution {
    const group = node.groupOf(row, value);

    return { partition, group, name: JSON.stringify(group), row };
}

/** Read the held value a row of a root or a key-joined node joins: its own join column, none for a root. */
function joinedOf(node: Node, row: Row): unknown {
    return node.path?.kind === "key" ? row[node.path.column] : undefined;
}
