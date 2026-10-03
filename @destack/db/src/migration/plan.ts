import type { Dialect } from "../dialect/dialect.ts";
import type { TableDescription } from "../inspect/table.ts";
import type { TreeDescription } from "../inspect/tree.ts";
import { createLog, logTriggers } from "../log/trigger.ts";
import { treeTriggers } from "../tree/trigger.ts";
import { aggregateTriggers, recomputeAggregate } from "../aggregate/trigger.ts";
import { dependentTriggers } from "../dependent/trigger.ts";
import { createState, deleteState, coversState, type TableState } from "./state.ts";
import * as statement from "./statement.ts";
import { quote } from "../dialect/quote.ts";
import { bridgeTriggers } from "./bridge.ts";
import type { Triggers } from "./trigger.ts";
import type { MergedState } from "./merge.ts";
import { canonicalize, Version } from "@destack/schema";
import { v7 } from "uuid";
import { Address, Plan, type Action, type Risk, type Step } from "@destack/resource";
import { PlanError } from "@destack/resource/error";

/** The generated triggers every plan removes and reinstalls. */
const TRIGGERS: readonly Triggers[] = [
    logTriggers,
    treeTriggers,
    aggregateTriggers,
    dependentTriggers,
    bridgeTriggers,
];

/** One change of a database plan to one table, addressed as `table/<name>` and its parts. */
export interface TableStep extends Step {
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
        target: Address.join("table", conflict.table),
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

        // create each missing table unless a newer release renamed it or an unmanaged one has its name
        if (!previous) {
            const renamed = applied.find(
                (entry) =>
                    entry.moved?.table === name &&
                    Version.compare(entry.package.version, state.package.version) > 0,
            );
            // refuse a table a newer release renamed, which the rollback would create again empty
            if (renamed !== undefined) {
                const detail = `rollback to ${state.package.version} cannot read the table ${renamed.package.version} renamed to ${renamed.table.name}`;
                problems.push({ target: Address.join("table", name), detail });
            }
            // refuse a name an unmanaged table has
            else if (existing.includes(name)) {
                const target = Address.join("table", name);
                problems.push({ target, detail: "table exists without applied state" });
            }
            // create the table
            else {
                steps.push(
                    step(
                        "create",
                        Address.join("table", name),
                        "safe",
                        "create table",
                        createStatements(state.table),
                    ),
                );
                created.push(state.table);
            }
            continue;
        }

        // keep a newer applied table for an older release that it covers, refusing one it cannot cover
        if (Version.compare(state.package.version, previous.package.version) < 0) {
            if (!coversState(previous, state)) {
                const detail = `rollback to ${state.package.version} cannot read the table as ${previous.package.version} applied it`;
                problems.push({ target: Address.join("table", name), detail });
            }
            continue;
        }

        // rename a moved table before changing it
        if (previous.table.name !== name) {
            steps.push(
                step(
                    "rename",
                    Address.join("table", name),
                    "backward-incompatible",
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

        // convert rows through each release after the applied one, and check the column values
        const conversions = Version.between(
            state.conversions ?? {},
            previous.package.version,
            state.package.version,
        );
        for (const [release, assignments] of conversions) {
            changes.push(convertRows(state, release, assignments));
        }
        const releases = conversions.map(([release]) => release);
        changes.push(...changeValues(previous, state, releases, problems));

        // backfill newly bridged columns
        const bridged = new Set((previous.bridges ?? []).map((bridge) => bridge.to));
        for (const bridge of state.bridges ?? []) {
            if (!bridged.has(bridge.to)) {
                changes.push(
                    step(
                        "convert",
                        Address.join("table", name, "column", bridge.to),
                        "fallible",
                        `copy ${bridge.from} into ${bridge.to}`,
                        [`UPDATE ${quote(name)} SET ${quote(bridge.to)} = ${quote(bridge.from)}`],
                    ),
                );
            }
        }

        // rebuild a changed tree's index
        if (state.tree && canonicalize(state.tree) !== canonicalize(previous.tree ?? null)) {
            changes.push({
                ...step(
                    "update",
                    Address.join("table", name, "tree"),
                    "safe",
                    "rebuild the ancestor index",
                    [],
                ),
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
                    "update",
                    Address.join("table", name, "log"),
                    "safe",
                    `log changes with retention ${state.log?.retention ?? "none"}`,
                    [],
                ),
            );
        }
        steps.push(...changes);
    }

    // compute new or changed aggregates
    for (const state of declared) {
        const previous = applied.find((entry) => entry.table.name === state.table.name);
        for (const aggregate of missing(state.aggregates, previous?.aggregates)) {
            steps.push(
                step(
                    "create",
                    Address.join("table", aggregate.table, "aggregate", aggregate.column),
                    "safe",
                    `compute ${aggregate.column} from ${aggregate.source}`,
                    [recomputeAggregate(aggregate, dialect)],
                ),
            );
        }
    }

    // stop keeping removed aggregates
    for (const previous of applied) {
        const state = declared.find((entry) => entry.table.name === previous.table.name);
        for (const aggregate of missing(previous.aggregates, state?.aggregates)) {
            steps.push(
                step(
                    "delete",
                    Address.join("table", aggregate.table, "aggregate", aggregate.column),
                    "safe",
                    `stop keeping ${aggregate.column} from ${aggregate.source}`,
                    [],
                ),
            );
        }
    }

    // start and stop keeping dependents
    for (const state of declared) {
        const previous = applied.find((entry) => entry.table.name === state.table.name);
        for (const dependent of missing(state.dependents, previous?.dependents)) {
            steps.push(
                step(
                    "create",
                    Address.join("table", dependent.table, "dependent", dependent.source),
                    "safe",
                    `${dependent.onDelete} deletes into ${dependent.source}`,
                    [],
                ),
            );
        }
    }
    for (const previous of applied) {
        const state = declared.find((entry) => entry.table.name === previous.table.name);
        for (const dependent of missing(previous.dependents, state?.dependents)) {
            steps.push(
                step(
                    "delete",
                    Address.join("table", dependent.table, "dependent", dependent.source),
                    "safe",
                    `stop ${dependent.onDelete} deletes into ${dependent.source}`,
                    [],
                ),
            );
        }
    }

    // add PostgreSQL foreign keys after every table exists
    if (dialect === "postgresql") {
        for (const table of created) {
            const keys = table.constraints.filter((constraint) => constraint.kind === "foreignKey");
            for (const key of keys) {
                steps.push(
                    step(
                        "create",
                        Address.join("table", table.name, "constraint", key.name),
                        "safe",
                        `add foreign key ${key.name}`,
                        [statement.addConstraint(table.name, key)],
                    ),
                );
            }
        }
    }

    // drop applied tables no longer declared, keeping those a newer release of a declared package added
    const releases = new Map<string, Version>();
    for (const state of declared) {
        const release = releases.get(state.package.id);
        if (release === undefined || Version.compare(state.package.version, release) > 0) {
            releases.set(state.package.id, state.package.version);
        }
    }
    const dropped = [...remaining.values()]
        .filter((previous) => {
            const release = releases.get(previous.package.id);

            return release === undefined || Version.compare(previous.package.version, release) <= 0;
        })
        .map((previous) => previous.table.name);
    for (const name of dropped) {
        steps.push(
            step("delete", Address.join("table", name), "destructive", "drop table", [
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
                  ...createLog(dialect, v7()),
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

/** A kept column with its applied and its declared description. */
interface ColumnChange {
    /** The column as applied. */
    readonly applied: TableDescription["columns"][number];
    /** The column as declared. */
    readonly declared: TableDescription["columns"][number];
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
        if (from !== undefined && renamed !== undefined && !columns.has(column.name)) {
            steps.push(
                step(
                    "rename",
                    Address.join("table", next.name, "column", column.name),
                    "backward-incompatible",
                    `rename column ${from} to ${column.name}`,
                    [statement.renameColumn(next.name, from, column.name)],
                ),
            );
            columns.delete(from);
            columns.set(column.name, { ...renamed, name: column.name });
        }
    }
    const current: TableDescription = { ...previous, columns: [...columns.values()] };

    // derive added, removed and changed columns
    const added = next.columns.filter((column) => !columns.has(column.name));
    const removed = current.columns.filter(
        (column) => !next.columns.some((entry) => entry.name === column.name),
    );
    const changed = next.columns.flatMap((column): ColumnChange[] => {
        const old = columns.get(column.name);
        const isChanged =
            old !== undefined &&
            canonicalize({ ...old, value: undefined }) !==
                canonicalize({ ...column, value: undefined });

        return old !== undefined && isChanged ? [{ applied: old, declared: column }] : [];
    });

    // require a value for each new required column
    const unfilled = added.filter(
        (column) => !column.nullable && column.default === undefined && !column.generated,
    );
    for (const column of unfilled) {
        problems.push({
            target: Address.join("table", next.name),
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
    changed: readonly ColumnChange[],
): TableStep[] {
    // rebuild for changes beyond additions and indexes
    const isRebuilt =
        removed.length > 0 ||
        changed.length > 0 ||
        added.some((column) => column.generated?.mode === "stored") ||
        canonicalize(previous.constraints) !== canonicalize(next.constraints);
    if (isRebuilt) {
        const copied = next.columns
            .filter(
                (column) =>
                    column.generated === undefined &&
                    previous.columns.some(
                        (applied) =>
                            applied.name === column.name && applied.generated === undefined,
                    ),
            )
            .map((column) => column.name);
        const isLossy =
            removed.length > 0 ||
            changed.some(({ applied, declared }) => applied.type !== declared.type);
        const isChecked =
            changed.some(({ applied, declared }) => applied.nullable && !declared.nullable) ||
            next.constraints.some(
                (constraint) =>
                    !previous.constraints.some(
                        (applied) => canonicalize(applied) === canonicalize(constraint),
                    ),
            );
        const detail = [
            ...added.map((column) => `add ${column.name}`),
            ...removed.map((column) => `drop ${column.name}`),
            ...changed.map(({ declared }) => `change ${declared.name}`),
        ].join(", ");

        return [
            step(
                "replace",
                Address.join("table", next.name),
                isLossy ? "destructive" : isChecked ? "fallible" : "safe",
                `rebuild table: ${detail || "constraints"}`,
                statement.rebuildTable(next, copied),
            ),
        ];
    }

    // add columns and replace changed indexes
    return [
        ...added.map((column) =>
            step(
                "create",
                Address.join("table", next.name, "column", column.name),
                "safe",
                `add column ${column.name}`,
                [statement.addColumn(next.name, column, "sqlite")],
            ),
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
    changed: readonly ColumnChange[],
): TableStep[] {
    // add and drop columns
    const steps: TableStep[] = [
        ...added.map((column) =>
            step(
                "create",
                Address.join("table", next.name, "column", column.name),
                "safe",
                `add column ${column.name}`,
                [statement.addColumn(next.name, column, "postgresql")],
            ),
        ),
        ...removed.map((column) =>
            step(
                "delete",
                Address.join("table", next.name, "column", column.name),
                "destructive",
                `drop column ${column.name}`,
                [statement.dropColumn(next.name, column.name)],
            ),
        ),
    ];

    // alter changed columns in place
    for (const { applied: old, declared: column } of changed) {
        if (old.generated !== undefined || column.generated !== undefined) {
            steps.push(
                step(
                    "replace",
                    Address.join("table", next.name, "column", column.name),
                    "safe",
                    `recreate column ${column.name}`,
                    [
                        statement.dropColumn(next.name, column.name),
                        statement.addColumn(next.name, column, "postgresql"),
                    ],
                ),
            );
        } else {
            steps.push(
                step(
                    "update",
                    Address.join("table", next.name, "column", column.name),
                    old.type !== column.type
                        ? "destructive"
                        : old.nullable && !column.nullable
                          ? "fallible"
                          : "safe",
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
    const { dropped, created } = differing(previous.constraints, next.constraints);

    return [
        ...dropped.map((constraint) =>
            step(
                "delete",
                Address.join("table", next.name, "constraint", constraint.name),
                "safe",
                `drop constraint ${constraint.name}`,
                [statement.dropConstraint(next.name, constraint.name)],
            ),
        ),
        ...created.map((constraint) =>
            step(
                "create",
                Address.join("table", next.name, "constraint", constraint.name),
                "fallible",
                `add constraint ${constraint.name}`,
                [statement.addConstraint(next.name, constraint)],
            ),
        ),
    ];
}

/** Replace removed, new or changed indexes. */
function changeIndexes(previous: TableDescription, next: TableDescription): TableStep[] {
    const { dropped, created } = differing(previous.indexes, next.indexes);

    return [
        ...dropped.map((index) =>
            step(
                "delete",
                Address.join("table", next.name, "index", index.name),
                "safe",
                `drop index ${index.name}`,
                [statement.dropIndex(index.name)],
            ),
        ),
        ...created.map((index) =>
            step(
                "create",
                Address.join("table", next.name, "index", index.name),
                index.unique ? "fallible" : "safe",
                `create index ${index.name}`,
                [statement.createIndex(next.name, index)],
            ),
        ),
    ];
}

/** Split named parts into the applied ones to drop and the declared ones to create, compared by name and definition. */
function differing<Part extends { readonly name: string }>(
    applied: readonly Part[],
    declared: readonly Part[],
): { readonly dropped: Part[]; readonly created: Part[] } {
    const before = new Map(applied.map((part) => [part.name, canonicalize(part)]));
    const after = new Map(declared.map((part) => [part.name, canonicalize(part)]));

    return {
        dropped: applied.filter((part) => after.get(part.name) !== before.get(part.name)),
        created: declared.filter((part) => before.get(part.name) !== after.get(part.name)),
    };
}

/** List the items absent from others, compared by definition. */
function missing<Item>(items: readonly Item[] = [], others: readonly Item[] = []): Item[] {
    const known = new Set(others.map((other) => canonicalize(other)));

    return items.filter((item) => !known.has(canonicalize(item)));
}

/** Create a table and its indexes. */
function createStatements(table: TableDescription): string[] {
    return [
        statement.createTable(table),
        ...table.indexes.map((index) => statement.createIndex(table.name, index)),
    ];
}

/** Convert every row of a table from earlier releases by the conversion one release introduces. */
function convertRows(
    state: TableState,
    release: Version,
    assignments: Readonly<Record<string, string>>,
): TableStep {
    // assign every converted column in one statement
    const name = state.table.name;
    const set = Object.entries(assignments)
        .map(([column, expression]) => `${quote(column)} = ${expression}`)
        .join(", ");

    return step("convert", Address.join("table", name), "fallible", `convert rows to ${release}`, [
        `UPDATE ${quote(name)} SET ${set}`,
    ]);
}

/** Check each kept column's values: widened values are safe, narrowed ones need a conversion. */
function changeValues(
    previous: TableState,
    state: TableState,
    releases: readonly Version[],
    problems: Problem[],
): TableStep[] {
    // compare each kept column's value schema, under its previous name when moved
    const steps: TableStep[] = [];
    const name = state.table.name;
    for (const column of state.table.columns) {
        const from = state.moved?.columns[column.name] ?? column.name;
        const old = previous.table.columns.find(
            (entry) => entry.name === from || entry.name === column.name,
        );
        if (old === undefined) {
            continue;
        }

        // plan the change, leaving convert steps to the row conversions
        const isConverted = releases.some(
            (release) => state.conversions?.[release]?.[column.name] !== undefined,
        );
        try {
            const planned = Plan.values({
                target: Address.join("table", name, "column", column.name),
                before: old.value,
                after: column.value,
                release: state.package.version,
                compatibility: "backward",
                isConverted,
            });
            for (const change of planned.steps.filter((entry) => entry.action === "update")) {
                steps.push(step("update", change.target, change.risk, change.detail, []));
            }
        } catch (error) {
            if (!(error instanceof PlanError)) {
                throw error;
            }
            problems.push(...error.problems);
        }
    }

    return steps;
}

/** Build a step. */
function step(
    action: Action,
    target: Address,
    risk: Risk,
    detail: string,
    statements: readonly string[],
): TableStep {
    return { action, target, risk, detail, statements };
}
