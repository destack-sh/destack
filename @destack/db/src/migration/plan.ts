import type { Dialect } from "../dialect/dialect.ts";
import type { TableDescription } from "../inspect/table.ts";
import type { TreeDescription } from "../inspect/tree.ts";
import { createLog, logTriggers } from "../log/trigger.ts";
import { treeTriggers } from "../tree/trigger.ts";
import { aggregateTriggers, recomputeAggregate } from "../aggregate/trigger.ts";
import { dependentTriggers } from "../dependent/trigger.ts";
import { createState, deleteState, type TableState } from "./state.ts";
import * as statement from "./statement.ts";
import { quote } from "../dialect/quote.ts";
import { bridgeTriggers } from "./bridge.ts";
import type { Triggers } from "./trigger.ts";
import type { MergedState } from "./merge.ts";
import { canonicalize } from "@destack/schema/json";
import type { Plan, Risk, Step } from "@destack/resource";
import { PlanError } from "@destack/resource/error";

/** The generated triggers every plan removes and reinstalls. */
const TRIGGERS: readonly Triggers[] = [
    logTriggers,
    treeTriggers,
    aggregateTriggers,
    dependentTriggers,
    bridgeTriggers,
];

/** What a table step changes. */
export type TableStepKind =
    | "createTable"
    | "dropTable"
    | "renameTable"
    | "rebuildTable"
    | "addColumn"
    | "dropColumn"
    | "renameColumn"
    | "alterColumn"
    | "alterConstraint"
    | "createIndex"
    | "dropIndex"
    | "convertRows"
    | "bridgeColumn"
    | "rebuildTree"
    | "updateLog"
    | "recomputeAggregate"
    | "dropAggregate"
    | "addDependent"
    | "dropDependent";

/** One change of a database plan to one table. */
export interface TableStep extends Step {
    /** What the step changes. */
    readonly kind: TableStepKind;
    /** The step's SQL, empty for tree rebuilds and log updates. */
    readonly statements: readonly string[];
    /** The tree a rebuild step reconstructs. */
    readonly tree?: TreeDescription;
}

/** The steps from a database's applied state to its declared tables. */
export interface TablePlan extends Plan<TableStep> {
    /** The database's dialect. */
    readonly dialect: Dialect;
    /** Remove generated triggers before the steps. */
    readonly before: readonly string[];
    /** Create the log and state tables and install triggers after the steps. */
    readonly after: readonly string[];
    /** The state recorded once the plan applied. */
    readonly state: readonly TableState[];
    /** The applied tables the plan drops. */
    readonly dropped: readonly string[];
}

/** A problem the declarations must fix. */
type Problem = PlanError["problems"][number];

/** The inputs of a plan. */
export interface PlanInput {
    /** The applied state, empty before the first plan. */
    readonly applied: readonly TableState[];
    /** The tables in the database's catalog. */
    readonly existing: readonly string[];
    /** The declared tables. */
    readonly declared: readonly TableState[];
    /** The conflicting tables with their reasons. */
    readonly conflicts?: MergedState["conflicts"];
    /** The database's dialect. */
    readonly dialect: Dialect;
}

/** Plan a database's migration, or name every problem to fix. */
export function planTables(input: PlanInput): TablePlan {
    // index applied tables and collect problems
    const { applied, existing, declared, dialect } = input;
    const remaining = new Map(applied.map((state) => [state.table.name, state]));
    const problems: Problem[] = (input.conflicts ?? []).map((conflict) => ({
        target: conflict.table,
        detail: conflict.reason,
    }));
    const steps: TableStep[] = [];
    const created: TableDescription[] = [];

    // plan each agreeing table
    const conflicting = new Set((input.conflicts ?? []).map((conflict) => conflict.table));
    for (const state of declared) {
        const name = state.table.name;
        if (conflicting.has(name)) {
            remaining.delete(name);
            continue;
        }
        const previous =
            remaining.get(name) ??
            (state.moved?.table === undefined ? undefined : remaining.get(state.moved.table));
        remaining.delete(previous?.table.name ?? name);

        // create each missing table unless an unmanaged one holds its name
        if (!previous) {
            if (existing.includes(name)) {
                problems.push({ target: name, detail: "table exists without applied state" });
            } else {
                steps.push(
                    step(
                        "createTable",
                        "safe",
                        name,
                        "create table",
                        createStatements(state.table),
                    ),
                );
                created.push(state.table);
            }
            continue;
        }

        // rename a moved table before changing it
        if (previous.table.name !== name) {
            steps.push(
                step(
                    "renameTable",
                    "backward-incompatible",
                    name,
                    `rename table from ${previous.table.name}`,
                    [statement.renameTable(previous.table.name, name)],
                ),
            );
        }
        const changes = changeTable(
            { ...previous.table, name },
            state.table,
            state.moved?.columns ?? {},
            problems,
        );

        // convert rows through each version above the applied one
        for (let version = previous.version + 1; version <= state.version; version++) {
            changes.push(...convertRows(state, version, problems));
        }

        // backfill newly bridged columns
        const bridged = new Set((previous.bridges ?? []).map((bridge) => bridge.to));
        for (const bridge of state.bridges ?? []) {
            if (!bridged.has(bridge.to)) {
                changes.push(
                    step(
                        "bridgeColumn",
                        "data-dependent",
                        name,
                        `copy ${bridge.from} into ${bridge.to}`,
                        [`UPDATE ${quote(name)} SET ${quote(bridge.to)} = ${quote(bridge.from)}`],
                    ),
                );
            }
        }

        // rebuild a changed tree's index
        if (state.tree && canonicalize(state.tree) !== canonicalize(previous.tree ?? null)) {
            changes.push({
                ...step("rebuildTree", "safe", name, "rebuild the ancestor index", []),
                tree: state.tree,
            });
        }

        // reinstall log triggers when nothing else changed
        if (
            changes.length === 0 &&
            canonicalize(state.log ?? null) !== canonicalize(previous.log ?? null)
        ) {
            changes.push(
                step(
                    "updateLog",
                    "safe",
                    name,
                    `log changes at tier ${state.log?.tier ?? "none"}`,
                    [],
                ),
            );
        }
        steps.push(...changes);
    }

    // compute new or changed aggregates
    for (const state of declared) {
        const previous = applied.find((entry) => entry.table.name === state.table.name);
        for (const aggregate of state.aggregates ?? []) {
            const isKnown = (previous?.aggregates ?? []).some(
                (entry) => canonicalize(entry) === canonicalize(aggregate),
            );
            if (!isKnown) {
                steps.push(
                    step(
                        "recomputeAggregate",
                        "safe",
                        aggregate.table,
                        `compute ${aggregate.column} from ${aggregate.source}`,
                        [recomputeAggregate(aggregate, dialect)],
                    ),
                );
            }
        }
    }

    // stop keeping removed aggregates
    for (const previous of applied) {
        const state = declared.find((entry) => entry.table.name === previous.table.name);
        for (const aggregate of previous.aggregates ?? []) {
            const isKept = (state?.aggregates ?? []).some(
                (entry) => canonicalize(entry) === canonicalize(aggregate),
            );
            if (!isKept) {
                steps.push(
                    step(
                        "dropAggregate",
                        "safe",
                        aggregate.table,
                        `stop keeping ${aggregate.column} from ${aggregate.source}`,
                        [],
                    ),
                );
            }
        }
    }

    // start and stop keeping dependents
    for (const state of declared) {
        const previous = applied.find((entry) => entry.table.name === state.table.name);
        for (const dependent of state.dependents ?? []) {
            const isKnown = (previous?.dependents ?? []).some(
                (entry) => canonicalize(entry) === canonicalize(dependent),
            );
            if (!isKnown) {
                steps.push(
                    step(
                        "addDependent",
                        "safe",
                        dependent.table,
                        `${dependent.onDelete} deletes into ${dependent.source}`,
                        [],
                    ),
                );
            }
        }
    }
    for (const previous of applied) {
        const state = declared.find((entry) => entry.table.name === previous.table.name);
        for (const dependent of previous.dependents ?? []) {
            const isKept = (state?.dependents ?? []).some(
                (entry) => canonicalize(entry) === canonicalize(dependent),
            );
            if (!isKept) {
                steps.push(
                    step(
                        "dropDependent",
                        "safe",
                        dependent.table,
                        `stop ${dependent.onDelete} deletes into ${dependent.source}`,
                        [],
                    ),
                );
            }
        }
    }

    // add PostgreSQL foreign keys after every table exists
    if (dialect === "postgresql") {
        for (const table of created) {
            const keys = table.constraints.filter((constraint) => constraint.kind === "foreignKey");
            if (keys.length > 0) {
                steps.push(
                    step(
                        "alterConstraint",
                        "safe",
                        table.name,
                        "add foreign keys",
                        keys.map((key) => statement.addConstraint(table.name, key)),
                    ),
                );
            }
        }
    }

    // drop applied tables no longer declared
    const dropped = [...remaining.keys()];
    for (const name of dropped) {
        steps.push(
            step("dropTable", "destructive", name, "drop table", [
                statement.dropTable(name),
                deleteState(name),
            ]),
        );
    }

    // fail with every problem at once
    if (problems.length > 0) {
        throw new PlanError(problems);
    }

    // reinstall every generated trigger around the steps
    const isChanged = steps.length > 0;

    return {
        dialect,
        steps,
        before: isChanged
            ? applied.flatMap((state) =>
                  TRIGGERS.flatMap((triggers) => triggers.remove(state, dialect)),
              )
            : [],
        after: isChanged
            ? [
                  ...createLog(dialect),
                  createState(),
                  ...declared.flatMap((state) =>
                      TRIGGERS.flatMap((triggers) => triggers.install(state, dialect)),
                  ),
              ]
            : [],
        state: declared,
        dropped,
    };
}

/** Plan the changes of one table kept under its name. */
function changeTable(
    previous: TableDescription,
    next: TableDescription,
    moved: Readonly<Record<string, string>>,
    problems: Problem[],
): TableStep[] {
    // rename moved columns first
    const steps: TableStep[] = [];
    const columns = new Map(previous.columns.map((column) => [column.name, column]));
    for (const column of next.columns) {
        const from = moved[column.name];
        const renamed = from === undefined ? undefined : columns.get(from);
        if (renamed && !columns.has(column.name)) {
            steps.push(
                step(
                    "renameColumn",
                    "backward-incompatible",
                    next.name,
                    `rename column ${from} to ${column.name}`,
                    [statement.renameColumn(next.name, from!, column.name)],
                ),
            );
            columns.delete(from!);
            columns.set(column.name, { ...renamed, name: column.name });
        }
    }
    const current: TableDescription = { ...previous, columns: [...columns.values()] };

    // derive added, removed and changed columns
    const added = next.columns.filter((column) => !columns.has(column.name));
    const removed = current.columns.filter(
        (column) => !next.columns.some((entry) => entry.name === column.name),
    );
    const changed = next.columns.filter((column) => {
        const old = columns.get(column.name);

        return old !== undefined && canonicalize(old) !== canonicalize(column);
    });

    // require a value for each new required column
    const unfilled = added.filter(
        (column) => !column.nullable && column.default === undefined && !column.generated,
    );
    for (const column of unfilled) {
        problems.push({
            target: next.name,
            detail: `declare a default for the required column ${column.name}`,
        });
    }

    // lower the changes per dialect
    return [
        ...steps,
        ...(next.dialect === "sqlite"
            ? changeSQLiteTable(current, next, added, removed, changed)
            : changePostgresTable(current, next, added, removed, changed)),
    ];
}

/** Lower table changes to SQLite, rebuilding for anything beyond additions and indexes. */
function changeSQLiteTable(
    previous: TableDescription,
    next: TableDescription,
    added: readonly TableDescription["columns"][number][],
    removed: readonly TableDescription["columns"][number][],
    changed: readonly TableDescription["columns"][number][],
): TableStep[] {
    // rebuild for changes beyond additions and indexes
    const isRebuilt =
        removed.length > 0 ||
        changed.length > 0 ||
        added.some((column) => column.generated?.mode === "stored") ||
        canonicalize(previous.constraints) !== canonicalize(next.constraints);
    if (isRebuilt) {
        const copied = new Map(
            next.columns
                .filter(
                    (column) =>
                        !column.generated &&
                        previous.columns.some((old) => old.name === column.name && !old.generated),
                )
                .map((column) => [column.name, column.name]),
        );
        const isLossy =
            removed.length > 0 ||
            changed.some(
                (column) =>
                    previous.columns.find((old) => old.name === column.name)!.type !== column.type,
            );
        const isChecked =
            changed.some(
                (column) =>
                    previous.columns.find((old) => old.name === column.name)!.nullable &&
                    !column.nullable,
            ) ||
            next.constraints.some(
                (constraint) =>
                    !previous.constraints.some(
                        (old) => canonicalize(old) === canonicalize(constraint),
                    ),
            );
        const detail = [
            ...added.map((column) => `add ${column.name}`),
            ...removed.map((column) => `drop ${column.name}`),
            ...changed.map((column) => `change ${column.name}`),
        ].join(", ");

        return [
            step(
                "rebuildTable",
                isLossy ? "destructive" : isChecked ? "data-dependent" : "safe",
                next.name,
                `rebuild table: ${detail || "constraints"}`,
                statement.rebuildTable(next, copied),
            ),
        ];
    }

    // add columns and replace changed indexes
    return [
        ...added.map((column) =>
            step("addColumn", "safe", next.name, `add column ${column.name}`, [
                statement.addColumn(next.name, column, "sqlite"),
            ]),
        ),
        ...changeIndexes(previous, next),
    ];
}

/** Lower table changes to PostgreSQL alterations. */
function changePostgresTable(
    previous: TableDescription,
    next: TableDescription,
    added: readonly TableDescription["columns"][number][],
    removed: readonly TableDescription["columns"][number][],
    changed: readonly TableDescription["columns"][number][],
): TableStep[] {
    // add and drop columns
    const steps: TableStep[] = [
        ...added.map((column) =>
            step("addColumn", "safe", next.name, `add column ${column.name}`, [
                statement.addColumn(next.name, column, "postgresql"),
            ]),
        ),
        ...removed.map((column) =>
            step("dropColumn", "destructive", next.name, `drop column ${column.name}`, [
                statement.dropColumn(next.name, column.name),
            ]),
        ),
    ];

    // alter changed columns in place
    for (const column of changed) {
        const old = previous.columns.find((entry) => entry.name === column.name)!;
        if (old.generated || column.generated) {
            steps.push(
                step("alterColumn", "safe", next.name, `recreate column ${column.name}`, [
                    statement.dropColumn(next.name, column.name),
                    statement.addColumn(next.name, column, "postgresql"),
                ]),
            );
        } else {
            steps.push(
                step(
                    "alterColumn",
                    old.type !== column.type
                        ? "destructive"
                        : old.nullable && !column.nullable
                          ? "data-dependent"
                          : "safe",
                    next.name,
                    `change column ${column.name}`,
                    statement.alterColumn(next.name, old, column),
                ),
            );
        }
    }

    // replace changed constraints and indexes
    return [...steps, ...changeConstraints(previous, next), ...changeIndexes(previous, next)];
}

/** Replace differing PostgreSQL constraints. */
function changeConstraints(previous: TableDescription, next: TableDescription): TableStep[] {
    // compare constraints by name and definition
    const before = new Map(previous.constraints.map((entry) => [entry.name, canonicalize(entry)]));
    const after = new Map(next.constraints.map((entry) => [entry.name, canonicalize(entry)]));

    return [
        ...previous.constraints
            .filter((constraint) => after.get(constraint.name) !== before.get(constraint.name))
            .map((constraint) =>
                step("alterConstraint", "safe", next.name, `drop constraint ${constraint.name}`, [
                    statement.dropConstraint(next.name, constraint.name),
                ]),
            ),
        ...next.constraints
            .filter((constraint) => before.get(constraint.name) !== after.get(constraint.name))
            .map((constraint) =>
                step(
                    "alterConstraint",
                    "data-dependent",
                    next.name,
                    `add constraint ${constraint.name}`,
                    [statement.addConstraint(next.name, constraint)],
                ),
            ),
    ];
}

/** Replace removed, new or changed indexes. */
function changeIndexes(previous: TableDescription, next: TableDescription): TableStep[] {
    // compare indexes by name and definition
    const before = new Map(previous.indexes.map((index) => [index.name, canonicalize(index)]));
    const after = new Map(next.indexes.map((index) => [index.name, canonicalize(index)]));

    return [
        ...previous.indexes
            .filter((index) => after.get(index.name) !== before.get(index.name))
            .map((index) =>
                step("dropIndex", "safe", next.name, `drop index ${index.name}`, [
                    statement.dropIndex(index.name),
                ]),
            ),
        ...next.indexes
            .filter((index) => before.get(index.name) !== after.get(index.name))
            .map((index) =>
                step(
                    "createIndex",
                    index.unique ? "data-dependent" : "safe",
                    next.name,
                    `create index ${index.name}`,
                    [statement.createIndex(next.name, index)],
                ),
            ),
    ];
}

/** Create a table and its indexes. */
function createStatements(table: TableDescription): string[] {
    return [
        statement.createTable(table),
        ...table.indexes.map((index) => statement.createIndex(table.name, index)),
    ];
}

/** Convert every row of a table to one version. */
function convertRows(state: TableState, version: number, problems: Problem[]): TableStep[] {
    // require the conversion as SQL
    const name = state.table.name;
    const assignments = state.conversions?.[String(version)];
    if (!assignments || Object.keys(assignments).length === 0) {
        problems.push({ target: name, detail: `declare a conversion to version ${version}` });

        return [];
    }

    // assign every converted column in one statement
    const set = Object.entries(assignments)
        .map(([column, expression]) => `${quote(column)} = ${expression}`)
        .join(", ");

    return [
        step("convertRows", "data-dependent", name, `convert rows to version ${version}`, [
            `UPDATE ${quote(name)} SET ${set}`,
        ]),
    ];
}

/** Build a step. */
function step(
    kind: TableStepKind,
    risk: Risk,
    target: string,
    detail: string,
    statements: readonly string[],
): TableStep {
    return { kind, risk, target, detail, statements };
}
