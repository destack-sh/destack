import { canonicalize } from "@destack/schema/json";
import type { TableDescription } from "../inspect/table.ts";
import { logOf, type TableState } from "./state.ts";

/** The merged table states of several declarations, and their conflicts. */
export interface MergedState {
    /** The union of each table's declared states. */
    readonly declared: readonly TableState[];
    /** The conflicting tables with their reasons. */
    readonly conflicts: readonly { readonly table: string; readonly reason: string }[];
}

/** Merge the table states of several declarations. */
export function mergeStates(declarations: readonly (readonly TableState[])[]): MergedState {
    // group each table's states
    const groups = new Map<string, TableState[]>();
    for (const state of declarations.flat()) {
        groups.set(state.table.name, [...(groups.get(state.table.name) ?? []), state]);
    }

    // merge each table's states
    const declared: TableState[] = [];
    const conflicts: { table: string; reason: string }[] = [];
    for (const [name, states] of groups) {
        const distinct = [...new Map(states.map((state) => [canonicalize(state), state])).values()];
        const merged = distinct.length === 1 ? { state: distinct[0]! } : mergeTable(distinct);
        declared.push(merged.state);
        if (merged.reason !== undefined) {
            conflicts.push({ table: name, reason: merged.reason });
        }
    }

    // block renaming a table still used under its old name
    for (const state of declared) {
        const previous = state.moved?.table;
        if (previous !== undefined && groups.has(previous)) {
            conflicts.push({
                table: state.table.name,
                reason: `a running release still uses ${previous}`,
            });
        }
    }

    return { declared, conflicts };
}

/** Merge one table's states into the newest, keeping older columns. */
function mergeTable(states: readonly TableState[]): {
    readonly state: TableState;
    readonly reason?: string;
} {
    // start from the newest release
    const newest = [...states].sort(
        (left, right) =>
            right.version - left.version ||
            Number(renames(right, states)) - Number(renames(left, states)) ||
            canonicalize(left).localeCompare(canonicalize(right)),
    )[0]!;
    const others = states.filter((state) => state !== newest);
    const declaresColumn = (name: string) =>
        others.some((state) => state.table.columns.some((column) => column.name === name));

    // bridge renamed columns still in use
    const moves = { ...newest.moved?.columns };
    const bridges = [...(newest.bridges ?? [])];
    for (const [column, previous] of Object.entries(moves)) {
        if (declaresColumn(previous)) {
            delete moves[column];
            bridges.push({ from: previous, to: column });
        }
    }

    // relax bridged columns the older release leaves empty
    const bridged = new Set(bridges.map((bridge) => bridge.to));
    const columns = newest.table.columns.map((column) =>
        bridged.has(column.name) ? relax(column) : column,
    );
    const byName = new Map(columns.map((column) => [column.name, column]));

    // keep older columns
    for (const state of others) {
        for (const column of state.table.columns) {
            const existing = byName.get(column.name);
            if (!existing) {
                if (
                    state.table.constraints.some(
                        (constraint) =>
                            constraint.kind === "primaryKey" &&
                            constraint.columns.includes(column.name),
                    )
                ) {
                    return { state: newest, reason: `releases disagree on the key ${column.name}` };
                }
                const retained = relax(column);
                columns.push(retained);
                byName.set(column.name, retained);
            } else if (
                !bridged.has(column.name) &&
                canonicalize(existing) !== canonicalize(column)
            ) {
                return { state: newest, reason: `releases disagree on column ${column.name}` };
            }
        }
    }

    // unite named constraints and indexes
    const ordered = [newest, ...others];
    const constraints = unite(ordered.map((state) => state.table.constraints));
    const indexes = unite(ordered.map((state) => state.table.indexes));
    const disagreement = constraints.conflict ?? indexes.conflict;
    if (disagreement !== undefined) {
        return { state: newest, reason: `releases disagree on ${disagreement}` };
    }
    const table: TableDescription = {
        ...newest.table,
        columns,
        constraints: constraints.entries,
        indexes: indexes.entries,
    };

    // require agreement on the log and tree
    const logs = new Set(states.map((state) => logOf(state)));
    const trees = new Set(states.map((state) => canonicalize(state.tree ?? null)));
    if (logs.size > 1 || trees.size > 1) {
        return { state: newest, reason: "releases disagree on the table's log or tree" };
    }

    // log every retained column
    const log = newest.log && {
        ...newest.log,
        columns: union(states.map((state) => state.log?.columns ?? [])),
        exact: union(states.map((state) => state.log?.exact ?? [])),
        compared: union(states.map((state) => state.log?.compared ?? [])),
    };

    return {
        state: {
            ...newest,
            table,
            ...(log ? { log } : {}),
            ...(newest.moved ? { moved: { ...newest.moved, columns: moves } } : {}),
            ...(bridges.length === 0 ? {} : { bridges }),
        },
    };
}

/** Make a required column without a default nullable. */
function relax(column: TableDescription["columns"][number]): TableDescription["columns"][number] {
    return column.nullable || column.default !== undefined || column.generated
        ? column
        : { ...column, nullable: true };
}

/** Unite named entries, or name the first differing entry. */
function unite<Entry extends { readonly name: string }>(
    lists: readonly (readonly Entry[])[],
): { entries: Entry[]; conflict?: string } {
    const united = new Map<string, Entry>();
    for (const entry of lists.flat()) {
        const existing = united.get(entry.name);
        if (existing !== undefined && canonicalize(existing) !== canonicalize(entry)) {
            return { entries: [], conflict: entry.name };
        }
        united.set(entry.name, existing ?? entry);
    }

    return { entries: [...united.values()] };
}

/** Unite name lists in order. */
function union(lists: readonly (readonly string[])[]): string[] {
    return [...new Set(lists.flat())];
}

/** Report whether a release renames a column another release still declares. */
function renames(state: TableState, states: readonly TableState[]): boolean {
    return Object.values(state.moved?.columns ?? {}).some((previous) =>
        states.some(
            (other) =>
                other !== state && other.table.columns.some((column) => column.name === previous),
        ),
    );
}
