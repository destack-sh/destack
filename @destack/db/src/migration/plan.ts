import type { Dialect } from "../dialect/dialect.ts";
import type { TableDescription } from "../table/description.ts";
import { createLog, logTriggers } from "../log/trigger.ts";
import { multikeyIndexTriggers, rebuildMultikeyIndex } from "../multikey/trigger.ts";
import { FullTextIndex } from "../search/fulltext.ts";
import { aggregateTriggers, recomputeAggregate } from "../aggregate/trigger.ts";
import { dependentTriggers } from "../dependent/trigger.ts";
import { createStamp, createState, deleteState, coversState, type TableState } from "./state.ts";
import * as statement from "./statement.ts";
import { literal, quote } from "../dialect/quote.ts";
import { relation } from "../table/namespace.ts";
import { bridgeTriggers } from "./bridge.ts";
import type { TriggerSet } from "./trigger.ts";
import type { Merge } from "./merge.ts";
import { canonicalize, Edition, nth, present, Version } from "@destack/schema";
import { Expression } from "../expression/expression.ts";
import { DatabaseError } from "../error/error.ts";
import { inline } from "../sql/render.ts";
import { Table } from "../table/table.ts";
import { Address, Plan, type Risk, type Step, type StepKind } from "@destack/resource";
import { type PlanProblem, ResourceError, unreachablePlan } from "@destack/resource/error";

/** The generated triggers a plan removes and reinstalls on the tables it changes. */
const TRIGGERS: readonly TriggerSet[] = [
    logTriggers,
    multikeyIndexTriggers,
    aggregateTriggers,
    dependentTriggers,
    bridgeTriggers,
];

/** The phases plan steps run in, in order: expand the tables, convert their rows, contract them. */
const PHASES = ["tables", "rows", "contract"] as const;

/** One change of a database plan to one table, addressed as `table/<name>` and its entries. */
export interface TableStep extends Step {
    /** When the step runs: expanding the tables, converting their rows, or contracting them. */
    readonly phase: TablePhase;
    /** The step's SQL, empty for log updates. */
    readonly statements: readonly string[];
    /** The table and columns a conversion step writes, whose values the apply checks. */
    readonly converted?: { readonly table: string; readonly columns: readonly string[] };
    /** The table a SQLite replace step rebuilds, written with the plan's other rebuilds. */
    readonly rebuild?: statement.TableRebuild;
    /** The table whose new full-text index SQLite fills after the steps. */
    readonly indexed?: TableState;
}

/** When a table step runs: expanding tables, converting rows after the triggers, or contracting. */
export type TablePhase = (typeof PHASES)[number];

/** The steps from a database's applied state to its declared tables. */
export interface TablePlan extends Plan<TableStep> {
    /** The database's dialect. */
    readonly dialect: Dialect;
    /** Remove generated triggers before the steps. */
    readonly before: readonly string[];
    /** Create the log and state tables and install triggers after the steps. */
    readonly after: readonly string[];
    /** Install again the triggers of the tables the contract rebuilds. */
    readonly restored: readonly string[];
    /** The state recorded once the plan applied. */
    readonly state: readonly TableState[];
    /** The applied tables the plan drops. */
    readonly dropped: readonly string[];
}

/** A problem the declarations must fix. */
type Problem = PlanProblem;

/** The inputs of a plan. */
export interface PlanInput {
    /** The applied state, empty before the first plan. */
    readonly applied: readonly TableState[];
    /** The tables in the database's catalog. */
    readonly existing: readonly string[];
    /** The declared tables. */
    readonly declared: readonly TableState[];
    /** The conflicting tables with their reasons. */
    readonly conflicts?: Merge["conflicts"];
    /** The database's dialect. */
    readonly dialect: Dialect;
    /** The database's namespace in a shared SQLite store, a prefix of the states' names. */
    readonly namespace?: string;
    /** The applied tables whose declared state differs, by applied name. */
    readonly changed: ReadonlySet<string>;
    /** The applied tables with no rows, whose changes convert and fill nothing. */
    readonly empty: ReadonlySet<string>;
    /** The applied tables of other managers, which rebuild with a declared table they reference. */
    readonly others?: readonly TableState[];
}

/** List the applied tables whose declared state differs, by applied name. */
export function changedTables(
    applied: readonly TableState[],
    declared: readonly TableState[],
): Set<string> {
    // compare each declared table with the applied one it retains or renames
    const changed = new Set<string>();
    for (const entry of declared) {
        const previous = applied.find(
            (each) =>
                each.table.name === entry.table.name || each.table.name === entry.moved?.table,
        );
        if (previous !== undefined && canonicalize(previous) !== canonicalize(entry)) {
            changed.add(previous.table.name);
        }
    }

    return changed;
}

/** Plan a database's migration, or report every problem to fix. */
export function planTables(input: PlanInput): TablePlan {
    // index applied tables and collect problems
    const { applied, declared, dialect } = input;
    const remaining = new Map(applied.map((state) => [state.table.name, state]));
    const problems: Problem[] = (input.conflicts ?? []).map((conflict) => ({
        target: Address.join("table", conflict.table),
        detail: conflict.reason,
    }));

    // rebuild each table referencing a rebuilt one with it, as SQLite enforces their foreign keys
    const others = input.others ?? [];
    const drafted = planDeclared(input, new Map(remaining), [], new Set());
    const following = referencingTables(
        drafted.steps.flatMap((each) => (each.rebuild === undefined ? [] : [each.rebuild.table])),
        [...declared, ...others].map((state) => state.table),
    );

    // plan each agreeing table, and rebuild the other managers' tables referencing a rebuilt one
    const planned = planDeclared(input, remaining, problems, following);
    const { created } = planned;
    const followed = others.filter((state) => following.has(state.table.name));
    const rebuilt = [
        ...planned.steps.filter((each) => each.rebuild !== undefined),
        ...followed.map(({ table }): TableStep => {
            // copy every stored column of a table rebuilt as it is
            const copied = table.columns.filter((column) => column.generated === undefined);
            const target = Address.join("table", table.name);
            const detail = "rebuild table: it references a rebuilt table";

            return {
                ...step("replace", target, "safe", detail, []),
                rebuild: { table, copied: copied.map((column) => column.name) },
            };
        }),
    ];

    // rebuild every table together, in the contract once a conversion reads a table's old columns
    const rebuilds = rebuilt.flatMap((each) => (each.rebuild === undefined ? [] : [each.rebuild]));
    const phase: TablePhase = rebuilt.some((each) => each.phase === "contract")
        ? "contract"
        : "tables";
    const [first, ...rest] = rebuilt.map((each) => ({ ...each, phase }));
    const steps =
        first === undefined
            ? planned.steps
            : rebuildSteps(planned.steps, created, [
                  { ...first, statements: statement.rebuildTables(rebuilds) },
                  ...rest,
              ]);

    // plan the tables' aggregates, dependents and foreign keys
    steps.push(...aggregateSteps(declared, applied, dialect));
    steps.push(...multikeyIndexSteps(declared, applied, dialect));
    steps.push(...dependentSteps(declared, applied));
    steps.push(...(dialect === "postgresql" ? foreignKeySteps(created) : []));
    steps.push(...declared.flatMap((state) => predefinedSteps(state, applied)));
    steps.push(...declared.flatMap((state) => globalIndexSteps(state, applied, input.namespace)));

    // drop applied tables no longer declared, except a newer release's tables
    const dropped = droppedTables(declared, remaining);
    steps.push(...(dialect === "postgresql" ? cyclicKeySteps(dropped.cyclic) : []));
    for (const name of dropped.ordered) {
        const previous = present(remaining.get(name), `the applied state of ${name}`);
        steps.push(
            step("delete", Address.join("table", name), "destructive", "drop table", [
                ...(previous.fullText === undefined ? [] : FullTextIndex.drop(previous, dialect)),
                statement.dropTable(name),
                deleteState(name, input.namespace),
            ]),
        );
    }

    // fail with every problem at once
    if (problems.length > 0) {
        throw unreachablePlan(problems);
    }

    // reinstall the generated triggers of the changed tables around the steps
    const triggers = reinstallTriggers(
        [...applied, ...followed],
        [...declared, ...followed],
        dialect,
        steps,
        input.namespace,
    );

    // install the triggers of the tables the contract rebuilds again, as a rebuild drops them
    const contracted = new Set(
        steps.flatMap((each) =>
            each.phase === "contract" && each.rebuild !== undefined
                ? [each.rebuild.table.name]
                : [],
        ),
    );
    const restored = [...declared, ...followed]
        .filter((state) => contracted.has(state.table.name))
        .flatMap((state) =>
            TRIGGERS.flatMap((set) => set.install(state, dialect, input.namespace)),
        );

    // list the steps in the order the phases run them
    const ordered = steps.toSorted(
        (left, right) => PHASES.indexOf(left.phase) - PHASES.indexOf(right.phase),
    );

    return {
        dialect,
        steps: ordered,
        ...triggers,
        restored,
        state: declared,
        dropped: dropped.ordered,
    };
}

/** Plan each declared table without a conflict. */
function planDeclared(
    input: PlanInput,
    remaining: Map<string, TableState>,
    problems: Problem[],
    following: ReadonlySet<string>,
): { readonly steps: TableStep[]; readonly created: TableDescription[] } {
    // collect the steps and the created tables
    const steps: TableStep[] = [];
    const created: TableDescription[] = [];
    const conflicting = new Set((input.conflicts ?? []).map((conflict) => conflict.table));
    for (const state of input.declared) {
        // skip a conflicting table
        const name = state.table.name;
        if (conflicting.has(name)) {
            remaining.delete(name);
            continue;
        }
        const previous =
            remaining.get(name) ??
            (state.moved?.table === undefined ? undefined : remaining.get(state.moved.table));
        remaining.delete(previous?.table.name ?? name);

        // forget the state an earlier rename left under the table's old name
        const left =
            state.moved?.table === undefined ? undefined : remaining.get(state.moved.table);
        if (left !== undefined && previous?.table.name === name) {
            remaining.delete(left.table.name);
            steps.push(
                step(
                    "delete",
                    Address.join("table", left.table.name),
                    "safe",
                    "forget renamed table",
                    [deleteState(left.table.name, input.namespace)],
                ),
            );
        }

        // create each missing table unless a newer release renamed it or an unmanaged one has it
        if (!previous) {
            const creating = planCreate(state, input, problems);
            if (creating !== undefined) {
                steps.push(creating);
                created.push(state.table);
            }
        }
        // retain a newer applied table an older release is covered by, refusing one it is not
        else if (Version.compare(state.package.version, previous.package.version) < 0) {
            if (!coversState(previous, state)) {
                const detail = `rollback to ${state.package.version} cannot read the table as ${previous.package.version} applied it`;
                problems.push({ target: Address.join("table", name), detail });
            }
        }
        // change an applied table whose declared state differs, or which follows a rebuilt table
        else if (input.changed.has(previous.table.name) || following.has(name)) {
            const isEmpty = input.empty.has(name);
            steps.push(
                ...planChange(
                    state,
                    previous,
                    isEmpty,
                    problems,
                    following.has(name),
                    input.namespace,
                ),
            );
        }
    }

    return { steps, created };
}

/** Plan the creation of a missing table. */
function planCreate(
    state: TableState,
    { applied, existing }: PlanInput,
    problems: Problem[],
): TableStep | undefined {
    const name = state.table.name;
    const renamed = applied.find(
        (entry) =>
            entry.moved?.table === name &&
            Version.compare(entry.package.version, state.package.version) > 0,
    );
    // refuse a table a newer release renamed, which the rollback would create again empty
    if (renamed !== undefined) {
        const detail = `rollback to ${state.package.version} cannot read the table ${renamed.package.version} renamed to ${renamed.table.name}`;
        problems.push({ target: Address.join("table", name), detail });

        return undefined;
    }
    // refuse a name an unmanaged table has
    else if (existing.includes(name)) {
        const target = Address.join("table", name);
        problems.push({ target, detail: "table exists without applied state" });

        return undefined;
    }
    // create the table and its full-text index
    else {
        const statements = [
            ...createStatements(state.table),
            ...(state.fullText === undefined
                ? []
                : FullTextIndex.create(state, state.table.dialect)),
        ];

        return step("create", Address.join("table", name), "safe", "create table", statements);
    }
}

/** Order a plan's steps around its rebuilds: the renames and new tables, the rebuilds, the rest. */
function rebuildSteps(
    steps: readonly TableStep[],
    created: readonly TableDescription[],
    rebuilt: readonly TableStep[],
): TableStep[] {
    // rename before the rebuilds copy by the new names, and create the tables they may reference
    const creating = new Set(created.map((table) => Address.join("table", table.name)));
    const isEarlier = (each: TableStep) => each.action === "rename" || creating.has(each.target);

    return [
        ...steps.filter(isEarlier),
        ...rebuilt,
        ...steps.filter((each) => !isEarlier(each) && each.rebuild === undefined),
    ];
}

/** List the tables referencing some tables by a foreign key, directly or through another. */
function referencingTables(
    referenced: readonly TableDescription[],
    tables: readonly TableDescription[],
): Set<string> {
    // follow the references from the tables found so far until none adds one
    const names = new Set(referenced.map((table) => table.name));
    const following = new Set<string>();
    let found = names.size;
    while (found > 0) {
        found = 0;
        for (const table of tables) {
            const isReferencing = table.constraints.some(
                (constraint) => constraint.kind === "foreignKey" && names.has(constraint.table),
            );
            if (isReferencing && !names.has(table.name)) {
                names.add(table.name);
                following.add(table.name);
                found += 1;
            }
        }
    }

    return following;
}

/** Plan the changes from a table's applied state to its declared one. */
function planChange(
    state: TableState,
    previous: TableState,
    isEmpty: boolean,
    problems: Problem[],
    isFollowing: boolean,
    namespace: string | undefined,
): TableStep[] {
    // rename a moved table before changing it, with its state under the new name alone
    const name = state.table.name;
    const steps: TableStep[] = [];
    if (previous.table.name !== name) {
        steps.push(
            step(
                "rename",
                Address.join("table", name),
                "backward-incompatible",
                `rename table from ${previous.table.name}`,
                [
                    statement.renameTable(previous.table.name, name),
                    deleteState(previous.table.name, namespace),
                ],
            ),
        );
    }
    const rendered = renderConversions(state, previous, problems);
    const assigned = assignedColumns(rendered);
    const changed = changeTable(
        { ...previous.table, name },
        state.table,
        state.moved?.columns ?? {},
        assigned,
        isEmpty,
        problems,
        isFollowing,
    );

    // change the tables, convert rows, then run the changes that need the converted rows
    const changes = changed.filter((each) => each.phase === "tables");
    changes.push(...convertSteps(state, previous, rendered, isEmpty, problems));
    changes.push(...changed.filter((each) => each.phase === "rows"));
    changes.push(...changed.filter((each) => each.phase === "contract"));

    // backfill newly bridged columns
    changes.push(...bridgeSteps(state, previous));
    changes.push(...fullTextSteps(state, previous));

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

    return [...steps, ...changes];
}

/** Convert rows by each conversion the applied state lacks, in edition order. */
function convertSteps(
    state: TableState,
    previous: TableState,
    rendered: Readonly<Record<string, Readonly<Record<string, string>>>>,
    isEmpty: boolean,
    problems: Problem[],
): TableStep[] {
    // refuse a conversion the applied state has that the declared one changes
    const changed = Edition.changed(previous.conversions ?? {}, state.conversions ?? {});
    if (changed.length > 0) {
        const detail = `changes the conversion to edition ${changed.join(", ")} that release ${previous.package.version} applied: add a new edition`;
        problems.push({ target: Address.join("table", state.table.name), detail });
    }

    // run each conversion the applied state lacks
    const conversions = Object.entries(state.conversions ?? {})
        .filter(([edition]) => previous.conversions?.[edition] === undefined)
        .toSorted(([left], [right]) => Number(left) - Number(right));
    const editions = conversions.map(([edition]) => edition);

    // update rows for each edition assigning columns, an edition that only drops some updating none
    const assigning = conversions.filter(([, assignments]) => Object.keys(assignments).length > 0);
    const updates = assigning.map(([edition]) =>
        convertRows(state, edition, rendered[edition] ?? {}),
    );

    return [...updates, ...changeValues(previous, state, editions, isEmpty, problems)];
}

/** Backfill the columns the declared state newly bridges from their previous names. */
function bridgeSteps(state: TableState, previous: TableState): TableStep[] {
    const name = state.table.name;
    const bridged = new Set((previous.bridges ?? []).map((bridge) => bridge.to));

    return (state.bridges ?? [])
        .filter((bridge) => !bridged.has(bridge.to))
        .map((bridge) =>
            step(
                "convert",
                Address.join("table", name, "column", bridge.to),
                "fallible",
                `copy ${bridge.from} into ${bridge.to}`,
                [`UPDATE ${quote(name)} SET ${quote(bridge.to)} = ${quote(bridge.from)}`],
                "rows",
            ),
        );
}

/** Fill the multikey indexes an existing table adds. */
function multikeyIndexSteps(
    declared: readonly TableState[],
    applied: readonly TableState[],
    dialect: Dialect,
): TableStep[] {
    return declared.flatMap((state) => {
        // leave a new table to its triggers
        const previous = applied.find((entry) => entry.table.name === state.table.name);
        if (previous === undefined) {
            return [];
        }
        const filled = new Set((previous.multikeyIndexes ?? []).map((each) => canonicalize(each)));

        return (state.multikeyIndexes ?? [])
            .filter((each) => !filled.has(canonicalize(each)))
            .map((each) =>
                step(
                    "update",
                    Address.join("table", state.table.name, "entries", each.name),
                    "safe",
                    `fill the ${each.name} entries`,
                    rebuildMultikeyIndex(each, dialect),
                ),
            );
    });
}

/** Rebuild a full-text index that changed, and drop or create one as declared. */
function fullTextSteps(state: TableState, previous: TableState): TableStep[] {
    // leave an unchanged index of an unrenamed table
    const isSame =
        canonicalize(state.fullText ?? null) === canonicalize(previous.fullText ?? null) &&
        state.fullTextVersion === previous.fullTextVersion &&
        state.table.name === previous.table.name;
    if (isSame) {
        return [];
    }

    // drop the applied index, then create the declared one
    const dialect = state.table.dialect;
    const rebuilt = step(
        "update",
        Address.join("table", state.table.name, "fullText"),
        "safe",
        "rebuild the full-text index",
        [
            ...(previous.fullText === undefined ? [] : FullTextIndex.drop(previous, dialect)),
            ...(state.fullText === undefined ? [] : FullTextIndex.create(state, dialect)),
        ],
    );

    // fill SQLite's, which PostgreSQL's own expression does
    const isFilled = state.fullText !== undefined && dialect === "sqlite";

    return [isFilled ? { ...rebuilt, indexed: state } : rebuilt];
}

/** Compute new or changed aggregates and stop maintaining removed ones. */
function aggregateSteps(
    declared: readonly TableState[],
    applied: readonly TableState[],
    dialect: Dialect,
): TableStep[] {
    // compute new or changed aggregates
    const created = declared.flatMap((state) => {
        const previous = applied.find((entry) => entry.table.name === state.table.name);

        return missing(state.aggregates, previous?.aggregates).map((aggregate) =>
            step(
                "create",
                Address.join("table", aggregate.table, "aggregate", aggregate.column),
                "safe",
                `compute ${aggregate.column} from ${aggregate.source}`,
                [recomputeAggregate(aggregate, dialect)],
            ),
        );
    });

    // stop maintaining removed aggregates
    const deleted = applied.flatMap((previous) => {
        const state = declared.find((entry) => entry.table.name === previous.table.name);

        return missing(previous.aggregates, state?.aggregates).map((aggregate) =>
            step(
                "delete",
                Address.join("table", aggregate.table, "aggregate", aggregate.column),
                "safe",
                `stop keeping ${aggregate.column} from ${aggregate.source}`,
                [],
            ),
        );
    });

    return [...created, ...deleted];
}

/** Start maintaining new dependents and stop maintaining removed ones. */
function dependentSteps(
    declared: readonly TableState[],
    applied: readonly TableState[],
): TableStep[] {
    // start maintaining new dependents
    const created = declared.flatMap((state) => {
        const previous = applied.find((entry) => entry.table.name === state.table.name);

        return missing(state.dependents, previous?.dependents).map((dependent) =>
            step(
                "create",
                Address.join("table", dependent.table, "dependent", dependent.source),
                "safe",
                `${dependent.onDelete} deletes into ${dependent.source}`,
                [],
            ),
        );
    });

    // stop maintaining removed dependents
    const deleted = applied.flatMap((previous) => {
        const state = declared.find((entry) => entry.table.name === previous.table.name);

        return missing(previous.dependents, state?.dependents).map((dependent) =>
            step(
                "delete",
                Address.join("table", dependent.table, "dependent", dependent.source),
                "safe",
                `stop ${dependent.onDelete} deletes into ${dependent.source}`,
                [],
            ),
        );
    });

    return [...created, ...deleted];
}

/** Add the created tables' PostgreSQL foreign keys after every table exists. */
function foreignKeySteps(created: readonly TableDescription[]): TableStep[] {
    return created.flatMap((table) =>
        table.constraints
            .filter((constraint) => constraint.kind === "foreignKey")
            .map((key) =>
                step(
                    "create",
                    Address.join("table", table.name, "constraint", key.name),
                    "safe",
                    `add foreign key ${key.name}`,
                    [statement.addConstraint(table.name, key)],
                ),
            ),
    );
}

/** List the applied tables to drop, each before the tables it references, a cycle's tables last. */
function droppedTables(
    declared: readonly TableState[],
    remaining: ReadonlyMap<string, TableState>,
): { readonly ordered: string[]; readonly cyclic: readonly TableDescription[] } {
    // find each declared package's newest release
    const releases = new Map<string, Version>();
    for (const state of declared) {
        const release = releases.get(state.package.id);
        if (release === undefined || Version.compare(state.package.version, release) > 0) {
            releases.set(state.package.id, state.package.version);
        }
    }

    // take the tables of no newer release
    let pending = [...remaining.values()]
        .filter((previous) => {
            const release = releases.get(previous.package.id);

            return release === undefined || Version.compare(previous.package.version, release) <= 0;
        })
        .map((previous) => previous.table);

    // drop the tables in layers, as a foreign key blocks dropping the table it references
    const ordered: string[] = [];
    let layer = unreferencedTables(pending);
    while (layer.length > 0) {
        const names = layer.map((table) => table.name);
        ordered.push(...names);
        pending = pending.filter((table) => !names.includes(table.name));
        layer = unreferencedTables(pending);
    }

    // drop the tables left last, whose foreign keys form a cycle
    ordered.push(...pending.map((table) => table.name));

    return { ordered, cyclic: pending };
}

/** Drop the foreign keys between tables dropped as a cycle, as PostgreSQL checks each drop. */
function cyclicKeySteps(cyclic: readonly TableDescription[]): TableStep[] {
    const names = new Set(cyclic.map((table) => table.name));

    return cyclic.flatMap((table) =>
        table.constraints
            .filter((constraint) => constraint.kind === "foreignKey")
            .filter((key) => key.table !== table.name && names.has(key.table))
            .map((key) =>
                step(
                    "delete",
                    Address.join("table", table.name, "constraint", key.name),
                    "destructive",
                    `drop foreign key ${key.name}`,
                    [statement.dropConstraint(table.name, key.name)],
                ),
            ),
    );
}

/** Select the tables that no other table in the list references. */
function unreferencedTables(tables: readonly TableDescription[]): TableDescription[] {
    // collect the names the foreign keys point at, leaving out a table's own name
    const referenced = new Set(
        tables.flatMap((table) =>
            table.constraints
                .filter((constraint) => constraint.kind === "foreignKey")
                .map((key) => key.table)
                .filter((name) => name !== table.name),
        ),
    );

    return tables.filter((table) => !referenced.has(table.name));
}

/** Remove the changed tables' triggers before the steps and install them again after. */
function reinstallTriggers(
    applied: readonly TableState[],
    declared: readonly TableState[],
    dialect: Dialect,
    steps: readonly TableStep[],
    namespace: string | undefined,
): { readonly before: string[]; readonly after: string[] } {
    // leave the triggers of an unchanged database
    if (steps.length === 0) {
        return { before: [], after: [] };
    }

    // list the tables the steps change, the second segment of each step's address
    const changed = new Set(steps.map((each) => nth(Address.split(each.target), 1)));
    const install = (state: TableState) =>
        TRIGGERS.flatMap((triggers) => triggers.install(state, dialect, namespace));
    const isChanged = (state: TableState) =>
        [state.table.name, ...TRIGGERS.flatMap((triggers) => triggers.tables(state))].some(
            (table) => changed.has(table),
        );

    // keep the triggers of a table no step changes whose declared triggers equal its applied ones
    const appliedBy = new Map(applied.map((state) => [state.table.name, state]));
    const kept = new Set(
        declared.flatMap((state) => {
            const previous = appliedBy.get(state.table.name);
            const isKept =
                previous !== undefined &&
                !isChanged(previous) &&
                !isChanged(state) &&
                install(previous).join("\n") === install(state).join("\n");

            return isKept ? [state.table.name] : [];
        }),
    );

    return {
        before: applied
            .filter((state) => !kept.has(state.table.name))
            .flatMap((state) => TRIGGERS.flatMap((triggers) => triggers.remove(state, dialect))),
        after: [
            ...createLog(dialect, namespace),
            createState(namespace),
            createStamp(namespace),
            ...declared.filter((state) => !kept.has(state.table.name)).flatMap(install),
        ],
    };
}

/** A retained column with its applied and its declared description. */
interface ColumnChange {
    /** The column as applied. */
    readonly applied: TableDescription["columns"][number];
    /** The column as declared. */
    readonly declared: TableDescription["columns"][number];
}

/** Plan the changes of one table retained under its name. */
function changeTable(
    previous: TableDescription,
    next: TableDescription,
    moved: Readonly<Record<string, string>>,
    assigned: Readonly<Record<string, string>>,
    isEmpty: boolean,
    problems: Problem[],
    isFollowing: boolean,
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

    // require a value for each new required column of a table with rows that no conversion assigns
    const unfilled = added.filter(
        (column) => !isEmpty && isUndefaulted(column) && assigned[column.name] === undefined,
    );
    for (const column of unfilled) {
        problems.push({
            target: Address.join("table", next.name),
            detail: `declare a default for the required column ${column.name}, or assign it in a new edition's conversion`,
        });
    }

    // lower the changes per dialect: add, convert, then require the columns a conversion fills
    const filled = filledColumns(added, assigned);

    return [
        ...steps,
        ...(next.dialect === "sqlite"
            ? changeSQLiteTable(current, next, added, removed, changed, assigned, isFollowing)
            : changePostgresTable(current, next, added, removed, changed, filled)),
    ];
}

/** Lower table changes to SQLite, rebuilding for anything beyond additions and indexes. */
function changeSQLiteTable(
    previous: TableDescription,
    next: TableDescription,
    added: readonly TableDescription["columns"][number][],
    removed: readonly TableDescription["columns"][number][],
    changed: readonly ColumnChange[],
    assigned: Readonly<Record<string, string>>,
    isFollowing: boolean,
): TableStep[] {
    // rebuild to follow a rebuilt table, for changes beyond additions and indexes, or a new required column, which SQLite cannot require in place
    const isRebuilt =
        isFollowing ||
        removed.length > 0 ||
        changed.length > 0 ||
        added.some((column) => column.generated?.mode === "stored") ||
        added.some((column) => isUndefaulted(column)) ||
        canonicalize(previous.constraints) !== canonicalize(next.constraints);
    if (isRebuilt) {
        // add the new columns before converting, as nullable, and rebuild in the contract once a conversion reads the old ones
        const isContracted = Object.keys(assigned).length > 0;
        const expanded = isContracted
            ? added.filter((column) => column.generated === undefined)
            : [];
        const expansions = expanded.map((column) =>
            step(
                "create",
                Address.join("table", next.name, "column", column.name),
                "safe",
                `add column ${column.name}`,
                [statement.addColumn(next.name, { ...column, nullable: true }, "sqlite")],
            ),
        );
        const copied = next.columns
            .filter(
                (column) =>
                    column.generated === undefined &&
                    (expanded.includes(column) ||
                        previous.columns.some(
                            (applied) =>
                                applied.name === column.name && applied.generated === undefined,
                        )),
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

        const isConstrained = canonicalize(previous.constraints) !== canonicalize(next.constraints);
        const reason = isConstrained ? "constraints" : "it references a rebuilt table";

        return [
            {
                ...step(
                    "replace",
                    Address.join("table", next.name),
                    isLossy ? "destructive" : isChecked ? "fallible" : "safe",
                    `rebuild table: ${detail || reason}`,
                    [],
                    isContracted ? "contract" : "tables",
                ),
                rebuild: { table: next, copied },
            },
            ...expansions,
        ];
    }

    // add columns and replace changed indexes
    const indexes = changeIndexes(previous, next);

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
        ...indexes.dropped,
        ...indexes.created,
    ];
}

/** Lower table changes to PostgreSQL alterations. */
function changePostgresTable(
    previous: TableDescription,
    next: TableDescription,
    added: readonly TableDescription["columns"][number][],
    removed: readonly TableDescription["columns"][number][],
    changed: readonly ColumnChange[],
    filled: Readonly<Record<string, string>>,
): TableStep[] {
    // drop the changed constraints and indexes first, as dropping a column drops those using it
    const constraints = changeConstraints(previous, next);
    const indexes = changeIndexes(previous, next);

    // add and drop columns
    const steps: TableStep[] = [
        ...constraints.dropped,
        ...indexes.dropped,
        ...added.map((column) =>
            step(
                "create",
                Address.join("table", next.name, "column", column.name),
                "safe",
                `add column ${column.name}`,
                [
                    statement.addColumn(
                        next.name,
                        Object.hasOwn(filled, column.name) ? { ...column, nullable: true } : column,
                        "postgresql",
                    ),
                ],
            ),
        ),
        ...removed.map((column) =>
            step(
                "delete",
                Address.join("table", next.name, "column", column.name),
                "destructive",
                `drop column ${column.name}`,
                [statement.dropColumn(next.name, column.name)],
                "contract",
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

    // create the changed constraints and indexes over the altered columns
    const created = [...constraints.created, ...indexes.created];
    if (Object.keys(filled).length === 0) {
        return [...steps, ...created];
    }

    // require the columns a conversion fills once it converted the rows, then create the constraints over them
    const required = Object.keys(filled).map((column) =>
        step(
            "update",
            Address.join("table", next.name, "column", column),
            "fallible",
            `require column ${column}`,
            [`ALTER TABLE ${quote(next.name)} ALTER COLUMN ${quote(column)} SET NOT NULL`],
            "rows",
        ),
    );

    return [...steps, ...required, ...created.map((each) => ({ ...each, phase: "rows" as const }))];
}

/** Plan the drops and creations replacing differing PostgreSQL constraints. */
function changeConstraints(previous: TableDescription, next: TableDescription): TableChanges {
    const { dropped, created } = differing(previous.constraints, next.constraints);

    return {
        dropped: dropped.map((constraint) =>
            step(
                "delete",
                Address.join("table", next.name, "constraint", constraint.name),
                "safe",
                `drop constraint ${constraint.name}`,
                [statement.dropConstraint(next.name, constraint.name)],
            ),
        ),
        created: created.map((constraint) =>
            step(
                "create",
                Address.join("table", next.name, "constraint", constraint.name),
                "fallible",
                `add constraint ${constraint.name}`,
                [statement.addConstraint(next.name, constraint)],
            ),
        ),
    };
}

/** Plan the drops and creations replacing removed, new or changed indexes. */
function changeIndexes(previous: TableDescription, next: TableDescription): TableChanges {
    const { dropped, created } = differing(previous.indexes, next.indexes);

    return {
        dropped: dropped.map((index) =>
            step(
                "delete",
                Address.join("table", next.name, "index", index.name),
                "safe",
                `drop index ${index.name}`,
                [statement.dropIndex(index.name)],
            ),
        ),
        created: created.map((index) =>
            step(
                "create",
                Address.join("table", next.name, "index", index.name),
                index.unique ? "fallible" : "safe",
                `create index ${index.name}`,
                [statement.createIndex(next.name, index)],
            ),
        ),
    };
}

/** The steps replacing a table's differing constraints or indexes: drops, then creations. */
interface TableChanges {
    /** The steps dropping the applied ones that differ. */
    readonly dropped: TableStep[];
    /** The steps creating the declared ones that differ. */
    readonly created: TableStep[];
}

/** Split named items into applied ones to drop and declared ones to create, by definition. */
function differing<Named extends { readonly name: string }>(
    applied: readonly Named[],
    declared: readonly Named[],
): { readonly dropped: Named[]; readonly created: Named[] } {
    const before = new Map(applied.map((named) => [named.name, canonicalize(named)]));
    const after = new Map(declared.map((named) => [named.name, canonicalize(named)]));

    return {
        dropped: applied.filter((named) => after.get(named.name) !== before.get(named.name)),
        created: declared.filter((named) => before.get(named.name) !== after.get(named.name)),
    };
}

/** List the items absent from others, compared by definition. */
function missing<Item>(items: readonly Item[] = [], others: readonly Item[] = []): Item[] {
    const known = new Set(others.map((other) => canonicalize(other)));

    return items.filter((item) => !known.has(canonicalize(item)));
}

/** Upsert a table's new or changed predefined rows and delete the ones it no longer declares. */
function predefinedSteps(state: TableState, applied: readonly TableState[]): TableStep[] {
    // compare the declared rows with the applied ones of the table or its previous name
    const name = state.table.name;
    const previous = applied.find(
        (entry) => entry.table.name === name || entry.table.name === state.moved?.table,
    );
    if (state.predefined === undefined && previous?.predefined === undefined) {
        return [];
    }
    const key = primaryKey(state);
    const declared = new Set((state.predefined ?? []).map((row) => predefinedKey(row, key)));
    const upserted = missing(state.predefined, previous?.predefined);
    const deleted = (previous?.predefined ?? []).filter(
        (row) => !declared.has(predefinedKey(row, key)),
    );
    if (upserted.length === 0 && deleted.length === 0) {
        return [];
    }

    // write each row as declared, and delete the ones no longer declared
    const deletes = deleted.map(
        (row) =>
            `DELETE FROM ${quote(name)} WHERE ${key.map((column) => `${quote(column)} = ${present(row[column], `${name}.${column}`)}`).join(" AND ")}`,
    );

    return [
        step(
            "update",
            Address.join("table", name, "predefined"),
            "safe",
            `keep ${state.predefined?.length ?? 0} predefined rows`,
            [...upsertPredefined(state, upserted), ...deletes],
        ),
    ];
}

/** Queue a table's rows for the global indexes it adds or removes, to build or drop them. */
function globalIndexSteps(
    state: TableState,
    applied: readonly TableState[],
    namespace: string | undefined,
): TableStep[] {
    // compare the declared indexes with those of the applied table, none for a new table
    const name = state.table.name;
    const previous = applied.find(
        (entry) => entry.table.name === name || entry.table.name === state.moved?.table,
    );
    if (previous === undefined) {
        return [];
    }
    const declared = state.globalIndexes ?? [];
    const earlier = previous.globalIndexes ?? [];
    const changed = [
        ...declared.filter(
            (index) =>
                !earlier.some(
                    (each) => each.name === index.name && each.declaration === index.declaration,
                ),
        ),
        ...earlier.filter((index) => !declared.some((each) => each.name === index.name)),
    ];
    if (changed.length === 0) {
        return [];
    }

    // queue every row under each changed index, keyed by the index so the queues stay apart
    const columns = ["request_id", "object_id", "owner", "index", "expires_at"].map(quote);
    const queues = changed.map(
        (index) =>
            `INSERT INTO ${quote(relation(index.outbox, namespace))} (${columns.join(", ")}) SELECT ${literal(`index:${index.name}`)}, ${quote(index.id)}, ${quote(index.scope)}, ${literal(index.name)}, 0 FROM ${quote(name)} WHERE true ON CONFLICT DO NOTHING`,
    );

    return [
        step(
            "update",
            Address.join("table", name, "globalIndexes"),
            "safe",
            `queue the rows for ${changed.map((index) => index.name).join(", ")}`,
            queues,
        ),
    ];
}

/** Write a table's predefined rows as declared, replacing the stored ones. */
export function upsertPredefined(
    state: TableState,
    rows: readonly Readonly<Record<string, string>>[] = state.predefined ?? [],
): string[] {
    const name = state.table.name;
    const key = primaryKey(state);

    return rows.map((row) => {
        // list the values, and update every column but the key on a conflict
        const columns = Object.keys(row);
        const values = columns.map((column) => present(row[column], `${name}.${column}`));
        const updates = columns
            .filter((column) => !key.includes(column))
            .map((column) => `${quote(column)} = excluded.${quote(column)}`);
        const conflict =
            updates.length === 0 ? "DO NOTHING" : `DO UPDATE SET ${updates.join(", ")}`;

        return `INSERT INTO ${quote(name)} (${columns.map(quote).join(", ")}) VALUES (${values.join(", ")}) ON CONFLICT (${key.map(quote).join(", ")}) ${conflict}`;
    });
}

/** Read the primary key columns of a table with predefined rows. */
function primaryKey(state: TableState): readonly string[] {
    const [key] = state.table.constraints.flatMap((constraint) =>
        constraint.kind === "primaryKey" ? [constraint.columns] : [],
    );
    if (key === undefined) {
        throw new TypeError(`'${state.table.name}' has predefined rows but no primary key`);
    }

    return key;
}

/** Write a predefined row's key as its SQL literals, alike for equal keys. */
function predefinedKey(row: Readonly<Record<string, string>>, key: readonly string[]): string {
    return JSON.stringify(key.map((column) => row[column]));
}

/** Create a table and its indexes. */
function createStatements(table: TableDescription): string[] {
    return [
        statement.createTable(table),
        ...table.indexes.map((index) => statement.createIndex(table.name, index)),
    ];
}

/** Convert every row of a table from earlier editions by the conversion leading to one. */
function convertRows(
    state: TableState,
    edition: string,
    assignments: Readonly<Record<string, string>>,
): TableStep {
    // assign every converted column in one statement
    const name = state.table.name;
    const set = Object.entries(assignments)
        .map(([column, expression]) => `${quote(column)} = ${expression}`)
        .join(", ");

    return {
        ...step(
            "convert",
            Address.join("table", name),
            "fallible",
            `convert rows to edition ${edition}`,
            [`UPDATE ${quote(name)} SET ${set}`],
            "rows",
        ),
        converted: { table: name, columns: Object.keys(assignments) },
    };
}

/** Check each retained column's values: widened ones are safe, narrowed ones need a conversion. */
function changeValues(
    previous: TableState,
    state: TableState,
    editions: readonly string[],
    isEmpty: boolean,
    problems: Problem[],
): TableStep[] {
    // compare each retained column's value schema, under its previous name when moved
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

        // plan the change, leaving convert steps to row conversions, which an empty table skips
        const isConverted =
            isEmpty ||
            editions.some((edition) => state.conversions?.[edition]?.[column.name] !== undefined);
        try {
            const planned = Plan.of({
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
            if (!ResourceError.is(error, "PLAN_BLOCKED")) {
                throw error;
            }
            problems.push(...error.data.problems);
        }
    }

    return steps;
}

/** Read the new editions' rendered assignments by column, a later edition's winning. */
function assignedColumns(
    rendered: Readonly<Record<string, Readonly<Record<string, string>>>>,
): Record<string, string> {
    const editions = Object.entries(rendered).toSorted(
        ([left], [right]) => Number(left) - Number(right),
    );

    return Object.fromEntries(editions.flatMap(([, assignments]) => Object.entries(assignments)));
}

/** Render the new editions' conversions as SQL, each over the row the edition before it stores. */
function renderConversions(
    state: TableState,
    previous: TableState,
    problems: Problem[],
): Record<string, Record<string, string>> {
    // start from the row as stored, under the table's new name and its moved columns' new names
    const moved = new Map(
        Object.entries(state.moved?.columns ?? {}).map(([to, from]) => [from, to]),
    );
    const columns = previous.table.columns.map((column) => ({
        ...column,
        name: moved.get(column.name) ?? column.name,
    }));

    // render the new editions in order, each over the row the earlier editions converted
    const editions = Object.entries(state.conversions ?? {})
        .filter(([edition]) => previous.conversions?.[edition] === undefined)
        .toSorted(([left], [right]) => Number(left) - Number(right));
    const rendered: Record<string, Record<string, string>> = {};
    for (const [edition, assignments] of editions) {
        const row = { ...previous.table, name: state.table.name, columns: [...columns] };
        const stored = Table.from(row, previous.package);
        rendered[edition] = Object.fromEntries(
            Object.entries(assignments).flatMap(([column, expression]) => {
                const assigned = renderAssignment(state, column, expression, stored, problems);

                return assigned === undefined ? [] : [[column, assigned]];
            }),
        );

        // add the columns this edition fills to the row the next edition reads
        for (const column of state.table.columns) {
            if (
                Object.hasOwn(assignments, column.name) &&
                !columns.some((each) => each.name === column.name)
            ) {
                columns.push(column);
            }
        }
    }

    return rendered;
}

/** Render an assignment over the stored row, refusing a missing column or another kind. */
function renderAssignment(
    state: TableState,
    column: string,
    expression: Expression,
    stored: Table,
    problems: Problem[],
): string | undefined {
    // read the declared column and the assignment's kind over the stored row
    const target = Address.join("table", state.table.name, "column", column);
    const declared = present(
        state.table.columns.find((each) => each.name === column),
        `the converted column '${column}'`,
    );
    let kind: ReturnType<typeof Expression.kind>;
    try {
        kind = Expression.kind(expression, stored);
    } catch (error) {
        if (!DatabaseError.is(error, "QUERY_INVALID")) {
            throw error;
        }
        problems.push({
            target,
            detail: `convert ${column} from the stored row: ${error.message}`,
        });

        return undefined;
    }

    // refuse another kind, reading integers as reals
    const isSameKind = kind === declared.kind || (kind === "real" && declared.kind === "integer");
    if (!isSameKind) {
        problems.push({
            target,
            detail: `convert ${column} to a ${declared.kind}: the conversion produces ${kind}`,
        });

        return undefined;
    }

    return inline(Expression.render(expression, stored), state.table.dialect);
}

/** Decide whether a column needs a value from each row: required, without default or generator. */
function isUndefaulted(column: TableDescription["columns"][number]): boolean {
    return !column.nullable && column.default === undefined && column.generated === undefined;
}

/** Map each added required column a conversion assigns to its assignment. */
function filledColumns(
    added: readonly TableDescription["columns"][number][],
    assigned: Readonly<Record<string, string>>,
): Record<string, string> {
    return Object.fromEntries(
        added.flatMap((column) => {
            const assignment = assigned[column.name];

            return isUndefaulted(column) && assignment !== undefined
                ? [[column.name, assignment]]
                : [];
        }),
    );
}

/** Build a step. */
function step(
    action: StepKind,
    target: Address,
    risk: Risk,
    detail: string,
    statements: readonly string[],
    phase: TablePhase = "tables",
): TableStep {
    return { action, target, risk, detail, phase, statements };
}
