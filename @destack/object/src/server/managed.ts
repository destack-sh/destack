import { v7 } from "uuid";
import { Authorizer, type Policy, Scope } from "@destack/access";
import { Snapshot } from "@destack/db/log";
import {
    and,
    type Column,
    type DatabaseConnection,
    eq,
    type Insert,
    isNull,
    type Select,
    TABLE,
    type Table,
} from "@destack/db";
import type { schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { ObjectError } from "../error/error.ts";
import type { ObjectType } from "../object/object.ts";
import { Manager, type ManagedColumnMap } from "../trait/declarable.ts";
import type { KeyIndex, Reservation } from "../key/key.ts";
import { ScopeHolder } from "../error/holder.ts";
import type { ObjectServer } from "./server.ts";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { Identifier } from "@destack/schema";

/** A table of managed records: their identity, revision and manager. */
type ManagedTable = Table &
    ManagedColumnMap & {
        /** The record identifier. */
        readonly id: Column;
        /** The scope the record lives in. */
        readonly scope: Column;
        /** The last modification time. */
        readonly updatedAt: Column;
        /** The revision used by conditional updates. */
        readonly revision: Column;
    };

/** One change reconciling a document makes to a record. */
export interface RecordChange {
    /** Create, update or delete. */
    readonly action: "create" | "update" | "delete";
    /** The declaration name of the changed record. */
    readonly name: string;
    /** Each changed column's previous and declared value, for updates. */
    readonly fields?: Readonly<
        Record<string, { readonly before: unknown; readonly after: unknown }>
    >;
}

/** What reconciling one object type's declarations can read and ask for. */
export interface ReconciliationContext {
    /** The scope holding the declared records, such as a space identifier. */
    readonly scope: string;
    /** The installation and package applying the document. */
    readonly manager: Omit<Manager, "name">;
    /** The transaction on the database holding this object type's records. */
    readonly database: DatabaseConnection;
    /** The complete declaration document. */
    readonly document: Readonly<Record<string, unknown>>;
    /** Resolve an object type the host reconciles, by name. */
    object(name: string): ObjectType;
    /** Find the identifier of the record the manager declared under a name. */
    find(object: ObjectType | string, name: string): Promise<string | undefined>;
    /** Find the identifier of the record declared under a name, or reject the declaration. */
    require(object: ObjectType | string, name: string): Promise<string>;
    /** Stop reconciling until a dependency is ready, reporting why. */
    wait(message: string): never;
    /** The time the document is applied, in UTC milliseconds. */
    readonly now: number;
    /** The policies validating declared access. */
    readonly authorizer: Authorizer;
    /** Run a system method in the reconciliation's transaction and scope. */
    invoke(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): Promise<unknown>;
    /** Open the build of a package's release in the scope, the installation's when named. */
    release(packageId: PackageId, installation?: Identifier<"installation">): Promise<BuildReader>;
}

/** How a stack's declarations of one object type become its managed records. */
export interface Reconciler<
    Object extends ObjectType = ObjectType,
    Collected = unknown,
    Resolved = unknown,
> {
    /** The object type reconciled. */
    readonly object: Object;
    /** The database holding the records, the document's database when absent. */
    readonly database?: DatabaseConnection;
    /** The object types reconciled before and retired after this one, or "every". */
    readonly after?: readonly ObjectType[] | "every";
    /** The document keys read, the object's plural by default. */
    readonly keys?: readonly string[];
    /** Select the declarations from a document, the object's plural collection by default. */
    collect?(document: Readonly<Record<string, unknown>>): Readonly<Record<string, Collected>>;
    /** Resolve references within a declaration, returning it unchanged by default. */
    resolve?(name: string, declared: Collected, context: ReconciliationContext): Promise<Resolved>;
    /** Map a resolved declaration to the columns it determines. */
    values(
        name: string,
        resolved: Resolved,
        context: ReconciliationContext,
    ): Partial<Insert<Object["table"]>>;
    /** Write a changed record's revision and update time. */
    touch?(row: Select<Object["table"]>, now: number): Partial<Insert<Object["table"]>>;
    /** Report changes held outside the record's own columns. */
    changed?(
        database: DatabaseConnection,
        row: Select<Object["table"]>,
        resolved: Resolved,
    ): Promise<boolean>;
    /** Write records a created or updated record owns. */
    written?(
        context: ReconciliationContext,
        row: Select<Object["table"]>,
        resolved: Resolved,
    ): Promise<void>;
    /** Retire records through their own deletion. */
    readonly retire?: {
        /** Report whether a record is already retiring. */
        isRetiring(database: DatabaseConnection, row: Select<Object["table"]>): Promise<boolean>;
        /** Begin retiring a record the manager no longer declares. */
        start(context: ReconciliationContext, row: Select<Object["table"]>): Promise<void>;
    };
}

/** The declaration type an object type accepts from stacks. */
type DeclaredOf<Object extends ObjectType> = [NonNullable<Object["declaration"]>] extends [never]
    ? never
    : schema.Infer<NonNullable<Object["declaration"]>>;

/** Define how a stack's declarations of an object type become its managed records. */
export function defineReconciler<
    Object extends ObjectType,
    Collected = DeclaredOf<Object>,
    Resolved = Collected,
>(
    object: Object,
    reconciler: Omit<Reconciler<Object, Collected, Resolved>, "object">,
): Reconciler<Object, Collected, Resolved> {
    return { object, ...reconciler };
}

/** The records a reconciliation changed and what it waits for. */
export interface ReconciliationResult {
    /** Changes by object plural name. */
    readonly changes: Readonly<Record<string, readonly RecordChange[]>>;
    /** Why reconciliation stopped before writing every declaration. */
    readonly waiting?: string;
}

/** The inputs of applying a declaration document. */
export interface ReconciliationOptions {
    /** The database holding the document's scope and records. */
    readonly database: DatabaseConnection;
    /** The reconcilers of the object types the host manages. */
    readonly reconcilers: readonly Reconciler[];
    /** The installation and package applying the document. */
    readonly manager: Omit<Manager, "name">;
    /** The scope containing the declared records. */
    readonly scope: string;
    /** The declaration document, keyed by collection. */
    readonly document: Readonly<Record<string, unknown>>;
    /** Report the changes to records held in other databases without writing them. */
    readonly isDry?: boolean;
    /** Further policies of declared objects. */
    readonly policies?: readonly Policy[];
    /** The key index, in another database. */
    readonly index?: KeyIndex;
    /** The object server running the reconcilers' system calls. */
    readonly server?: Pick<ObjectServer, "invoke">;
    /** Open the build of a package's release in the scope, the installation's when named. */
    readonly release?: ReconciliationContext["release"];
}

/** One application of a declaration document at one time. */
export class Reconciliation {
    /** What the application reads. */
    readonly options: ReconciliationOptions;
    /** The reconcilers in dependency order. */
    readonly ordered: readonly Reconciler[];
    /** The policies validating declared access. */
    readonly authorizer: Authorizer;
    /** The time of the application, in UTC milliseconds. */
    readonly now: number;

    /** Order the reconcilers and validate the document. */
    private constructor(options: ReconciliationOptions) {
        // require a key index for reconciled objects declaring indexes
        const indexed = options.reconcilers.find(
            (entry) => Object.keys(entry.object.indexes).length > 0,
        );
        if (indexed !== undefined && options.index === undefined) {
            throw new TypeError(
                `object ${indexed.object.name} declares indexes but no key index keeps them`,
            );
        }

        // reject unread collections
        const ordered = order(options.reconcilers);
        const keys = new Set(ordered.flatMap((entry) => entry.keys ?? [entry.object.plural]));
        for (const key of Object.keys(options.document)) {
            if (!keys.has(key)) {
                throw new ObjectError("UNSUPPORTED_DECLARATION", `this host cannot apply ${key}`);
            }
        }

        // keep the order, policies and time
        this.options = options;
        this.ordered = ordered;
        this.authorizer = new Authorizer([
            ...options.reconcilers.map((entry) => entry.object.policy),
            ...(options.policies ?? []),
        ]);
        this.now = Date.now();
    }

    /** Apply a declaration document through the reconcilers of its object types. */
    static async apply(options: ReconciliationOptions): Promise<ReconciliationResult> {
        return new Reconciliation(options).#run();
    }

    /** Write each object type's declarations, then retire undeclared records. */
    async #run(): Promise<ReconciliationResult> {
        // write each type in order
        const changes: Record<string, RecordChange[]> = {};
        const resolved = new Map<Reconciler, ReadonlyMap<string, unknown>>();
        for (const reconciler of this.ordered) {
            try {
                await this.#transact(reconciler, async (context) => {
                    // resolve, then write the difference
                    const desired = new Map<string, unknown>();
                    for (const [name, declared] of Object.entries(
                        collect(reconciler, this.options.document),
                    )) {
                        desired.set(
                            name,
                            reconciler.resolve
                                ? await reconciler.resolve(name, declared, context)
                                : declared,
                        );
                    }
                    resolved.set(reconciler, desired);
                    changes[reconciler.object.plural] = await this.#write(
                        reconciler,
                        context,
                        desired,
                    );
                });
            } catch (error) {
                // stop at a wait
                if (error instanceof Waiting) {
                    return { changes, waiting: error.message };
                }
                throw error;
            }
        }

        // retire in reverse order
        for (const reconciler of [...this.ordered].reverse()) {
            await this.#transact(reconciler, async (context) => {
                const retired = await this.#retire(reconciler, context, resolved.get(reconciler)!);
                const plural = reconciler.object.plural;
                changes[plural] = [...(changes[plural] ?? []), ...retired];
            });
        }

        return { changes };
    }

    /** Run a step on a reconciler's database, reserving and confirming its index keys. */
    async #transact(
        reconciler: Reconciler,
        step: (context: ReconciliationContext) => Promise<void>,
    ): Promise<void> {
        // pick the database
        const { options, now } = this;
        const root = reconciler.database ?? options.database;
        const requestId = crypto.randomUUID();
        let reservation: Reservation | undefined;
        try {
            reservation = await root.transaction(async (database) => {
                // guard the scope chain and refuse a moved scope
                const chain = await Scope.chain(Snapshot.live(database), options.scope);
                await Scope.guard(
                    database,
                    chain.map((link) => link.object.id),
                );
                const moved = chain.find((link) => link.movedTo !== undefined);
                if (moved !== undefined) {
                    throw ScopeHolder.error({ scope: moved.object.id, holder: moved.movedTo! });
                }

                // run the step
                const find = (object: ObjectType | string, name: string) => {
                    const target = this.#select(object);
                    const targetRoot = target.database ?? options.database;

                    return this.#find(target, targetRoot === root ? database : targetRoot, name);
                };
                await step({
                    scope: options.scope,
                    manager: options.manager,
                    authorizer: this.authorizer,
                    database,
                    document: options.document,
                    object: (name) => this.#select(name).object,
                    find,
                    require: async (object, name) => {
                        const id = await find(object, name);
                        if (id === undefined) {
                            const type = typeof object === "string" ? object : object.name;
                            throw new ObjectError(
                                "INVALID_DECLARATION",
                                `unknown ${type}: ${name}`,
                            );
                        }

                        return id;
                    },
                    wait: (message) => {
                        throw new Waiting(message);
                    },
                    now,
                    invoke: (object, name, input) => {
                        // require the server running system calls
                        if (options.server === undefined) {
                            throw new TypeError(
                                `reconciling ${reconciler.object.plural} invokes ${object.name}.${name}, but no object server runs it`,
                            );
                        }

                        return options.server.invoke(
                            database,
                            options.scope,
                            object,
                            name,
                            input,
                            now,
                        );
                    },
                    release: (packageId, installation) => {
                        // require the host opening releases
                        if (options.release === undefined) {
                            throw new TypeError(
                                `reconciling ${reconciler.object.plural} opens the release of ${packageId}, but no host opens releases`,
                            );
                        }

                        return options.release(packageId, installation);
                    },
                });

                // reserve written keys
                return await options.index?.reserve(
                    database,
                    options.reconcilers.map((entry) => entry.object),
                    requestId,
                    now,
                );
            });
        } catch (error) {
            // release keys
            await options.index?.release(requestId);
            throw error;
        }

        // confirm keys
        if (reservation !== undefined) {
            await options.index!.confirm(reservation);
        }
    }

    /** Select a type's reconciler by type or name. */
    #select(object: ObjectType | string): Reconciler {
        const target = this.options.reconcilers.find((entry) =>
            typeof object === "string" ? entry.object.name === object : entry.object.same(object),
        );
        if (!target) {
            const plural = typeof object === "string" ? object : object.plural;
            throw new ObjectError("UNSUPPORTED_DECLARATION", `this host cannot apply ${plural}`);
        }

        return target;
    }

    /** Find the identifier of the record declared under a name in the scope. */
    async #find(
        reconciler: Reconciler,
        database: DatabaseConnection,
        name: string,
    ): Promise<string | undefined> {
        // match by manager and name
        const table = reconciler.object.table as ManagedTable;
        const { manager, scope } = this.options;
        const [row] = (await database
            .select({ id: table.id })
            .from(table)
            .where(
                and(
                    eq(table.scope, scope),
                    eq(table.managerInstallationId, manager.installationId),
                    eq(table.managerPackageId, manager.packageId),
                    eq(table.managerName, name),
                ),
            )) as { id: string }[];

        return row?.id;
    }

    /** Read the manager's records of one object type in the scope, by declaration name. */
    async #records(
        reconciler: Reconciler,
        context: ReconciliationContext,
    ): Promise<Map<string, Select<ManagedTable>>> {
        const table = reconciler.object.table as ManagedTable;
        const rows = (await context.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.scope, context.scope),
                    eq(table.managerInstallationId, context.manager.installationId),
                    eq(table.managerPackageId, context.manager.packageId),
                    "purgedAt" in table[TABLE].columns
                        ? isNull((table as ManagedTable & Record<string, Column>).purgedAt!)
                        : undefined,
                ),
            )) as Select<ManagedTable>[];

        return new Map(rows.map((row) => [row.managerName as string, row]));
    }

    /** Whether to only report a reconciler's changes. */
    #isDry(reconciler: Reconciler): boolean {
        return this.options.isDry === true && reconciler.database !== undefined;
    }

    /** Create missing records and update changed attached ones. */
    async #write(
        reconciler: Reconciler,
        context: ReconciliationContext,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<RecordChange[]> {
        // read existing records
        const isDry = this.#isDry(reconciler);
        const { database, manager } = context;
        const table = reconciler.object.table as ManagedTable;
        const existing = await this.#records(reconciler, context);
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        const changes: RecordChange[] = [];
        const { now } = context;
        for (const [name, resolved] of desired) {
            // create a missing record
            const row = existing.get(name) as Record<string, unknown> | undefined;
            const values = reconciler.values(name, resolved, context) as Record<string, unknown>;
            requireColumns(reconciler.object, values);
            const changed = row === undefined ? {} : difference(row, values);
            if (!row) {
                changes.push({ action: "create", name });
                if (!isDry) {
                    const created = {
                        id: `${reconciler.object.identity}-${v7()}`,
                        createdAt: now,
                        updatedAt: now,
                        ...values,
                        scope: context.scope,
                        ...Manager.values({ ...manager, name }),
                    } as Insert<Table>;
                    await database.insert(table).values(created);
                    const id = (created as Record<string, unknown>).id;
                    await reconciler.written?.(
                        context,
                        (await read(database, table, id))!,
                        resolved,
                    );
                }
            }
            // update a changed attached record
            else if (
                Manager.isManaging(row) &&
                (Object.keys(changed).length > 0 ||
                    (isDeletable && row.deletionRequestedAt !== null) ||
                    (await reconciler.changed?.(database, row as never, resolved)))
            ) {
                changes.push({
                    action: "update",
                    name,
                    fields: {
                        ...changed,
                        ...(isDeletable && row.deletionRequestedAt !== null
                            ? {
                                  deletionRequestedAt: {
                                      before: row.deletionRequestedAt,
                                      after: null,
                                  },
                              }
                            : {}),
                    },
                });
                if (!isDry) {
                    await database
                        .update(table)
                        .set({
                            ...values,
                            ...("generation" in table[TABLE].columns
                                ? { generation: (row.generation as number) + 1 }
                                : {}),
                            ...(isDeletable ? { deletionRequestedAt: null } : {}),
                            ...(reconciler.touch?.(row as never, now) ?? {
                                revision: (row.revision as number) + 1,
                                updatedAt: now,
                            }),
                        } as Partial<Insert<Table>>)
                        .where(eq(table.id, row.id));
                    await reconciler.written?.(
                        context,
                        (await read(database, table, row.id))!,
                        resolved,
                    );
                }
            }
        }

        return changes;
    }

    /** Retire attached records the manager no longer declares. */
    async #retire(
        reconciler: Reconciler,
        context: ReconciliationContext,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<RecordChange[]> {
        // read existing records
        const isDry = this.#isDry(reconciler);
        const { database } = context;
        const table = reconciler.object.table as ManagedTable;
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        const changes: RecordChange[] = [];
        for (const [name, row] of await this.#records(reconciler, context)) {
            // skip declared, detached and already retiring records
            const record = row as Record<string, unknown>;
            if (
                desired.has(name) ||
                !Manager.isManaging(record) ||
                (isDeletable && record.deletionRequestedAt !== null) ||
                (await reconciler.retire?.isRetiring(database, row as never))
            ) {
                continue;
            }
            changes.push({ action: "delete", name });
            if (isDry) {
                continue;
            }

            // retire, request deletion, or delete
            if (reconciler.retire) {
                await reconciler.retire.start(context, row as never);
            } else if (isDeletable) {
                await database
                    .update(table)
                    .set({ deletionRequestedAt: context.now } as Partial<Insert<Table>>)
                    .where(eq(table.id, row.id));
            } else {
                await database.delete(table).where(eq(table.id, row.id));
            }
        }

        return changes;
    }
}

/** Stop reconciling until a dependency is ready. */
class Waiting extends Error {}

/** Select a reconciler's declarations from a document. */
function collect(
    reconciler: Reconciler,
    document: Readonly<Record<string, unknown>>,
): Readonly<Record<string, unknown>> {
    if (reconciler.collect) {
        return reconciler.collect(document);
    }
    const declared = (document[reconciler.object.plural] ?? {}) as Record<string, unknown>;
    const schema = reconciler.object.declaration;

    return schema
        ? Object.fromEntries(
              Object.entries(declared).map(([name, value]) => [name, schema.parse(value)]),
          )
        : declared;
}

/** Order reconcilers so each follows the object types it references. */
function order(reconcilers: readonly Reconciler[]): Reconciler[] {
    // read explicit dependencies
    const explicit = new Map(
        reconcilers.map((entry) => {
            const dependencies = Array.isArray(entry.after) ? entry.after : [];

            return [
                entry,
                dependencies.flatMap((object) =>
                    reconcilers.filter((other) => other.object.same(object)),
                ),
            ] as const;
        }),
    );

    // expand "every" to reconcilers not depending on this one
    const dependsOn = (from: Reconciler, to: Reconciler, seen = new Set<Reconciler>()): boolean => {
        if (seen.has(from)) {
            return false;
        }
        seen.add(from);

        return explicit
            .get(from)!
            .some((dependency) => dependency === to || dependsOn(dependency, to, seen));
    };
    const dependencies = new Map(
        reconcilers.map((entry) => [
            entry,
            entry.after === "every"
                ? reconcilers.filter(
                      (other) =>
                          other !== entry && other.after !== "every" && !dependsOn(other, entry),
                  )
                : explicit.get(entry)!,
        ]),
    );

    // visit dependencies depth first, rejecting cycles
    const ordered: Reconciler[] = [];
    const visiting = new Set<Reconciler>();
    const visit = (reconciler: Reconciler) => {
        if (ordered.includes(reconciler)) {
            return;
        }
        if (visiting.has(reconciler)) {
            throw new ObjectError(
                "CYCLIC_DECLARATION",
                `declarations of ${reconciler.object.plural} depend on themselves`,
            );
        }
        visiting.add(reconciler);
        for (const dependency of dependencies.get(reconciler)!) {
            visit(dependency);
        }
        visiting.delete(reconciler);
        ordered.push(reconciler);
    };
    for (const reconciler of reconcilers) {
        visit(reconciler);
    }

    return ordered;
}

/** Read one record by identifier. */
async function read(
    database: DatabaseConnection,
    table: ManagedTable,
    id: unknown,
): Promise<Select<ManagedTable> | undefined> {
    const [row] = (await database
        .select()
        .from(table)
        .where(eq(table.id, id))) as Select<ManagedTable>[];

    return row;
}

/** Collect the declared values that differ from a row. */
function difference(
    row: Record<string, unknown>,
    values: Record<string, unknown>,
): Record<string, { before: unknown; after: unknown }> {
    return Object.fromEntries(
        Object.entries(values)
            .filter(
                ([key, value]) => canonicalize(row[key] ?? null) !== canonicalize(value ?? null),
            )
            .map(([key, value]) => [key, { before: row[key] ?? null, after: value ?? null }]),
    );
}

/** Refuse values naming unknown columns. */
function requireColumns(object: ObjectType, values: Readonly<Record<string, unknown>>): void {
    const columns = object.table[TABLE].columns;
    const unknown = Object.keys(values).find((key) => !Object.hasOwn(columns, key));
    if (unknown !== undefined) {
        throw new TypeError(`reconciler of ${object.name} writes unknown column ${unknown}`);
    }
}
