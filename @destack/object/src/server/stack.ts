import { Address, type Plan, type Step } from "@destack/resource";
import { v7 } from "uuid";
import { Scope } from "@destack/sync";
import { Authorizer, type Policy } from "@destack/access";
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
import type { Directory } from "@destack/directory";
import { Reservation } from "../claim/index.ts";
import { Moved } from "@destack/directory";
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

/** How a stack's declarations of one object type become its managed records. */
export interface ObjectDeclaration<
    Object extends ObjectType = ObjectType,
    Collected = unknown,
    Resolved = unknown,
> {
    /** The database holding the records, the document's database when absent. */
    readonly database?: DatabaseConnection;
    /** The object types applied before and retired after this one, or "every". */
    readonly after?: readonly ObjectType[] | "every";
    /** The document keys read, the object's plural by default. */
    readonly keys?: readonly string[];
    /** Select the declarations from a document, the object's plural collection by default. */
    collect?(document: Readonly<Record<string, unknown>>): Readonly<Record<string, Collected>>;
    /** Resolve references within a declaration, returning it unchanged by default. */
    resolve?(name: string, declared: Collected, stack: Stack): Promise<Resolved>;
    /** Map a resolved declaration to the columns it determines. */
    values(name: string, resolved: Resolved, stack: Stack): Partial<Insert<Object["table"]>>;
    /** Write a changed record's revision and update time. */
    touch?(row: Select<Object["table"]>, now: number): Partial<Insert<Object["table"]>>;
    /** Report changes held outside the record's own columns. */
    changed?(
        database: DatabaseConnection,
        row: Select<Object["table"]>,
        resolved: Resolved,
    ): Promise<boolean>;
    /** Write records a created or updated record owns. */
    written?(stack: Stack, row: Select<Object["table"]>, resolved: Resolved): Promise<void>;
    /** Retire records through their own deletion. */
    readonly retire?: {
        /** Report whether a record is already retiring. */
        isRetiring(database: DatabaseConnection, row: Select<Object["table"]>): Promise<boolean>;
        /** Begin retiring a record the manager no longer declares. */
        start(stack: Stack, row: Select<Object["table"]>): Promise<void>;
    };
}

/** The declaration type an object type accepts from stacks. */
export type DeclarationOf<Object extends ObjectType> = [
    NonNullable<Object["declarationSchema"]>,
] extends [never]
    ? never
    : schema.Infer<NonNullable<Object["declarationSchema"]>>;

/** What a host applies a stack with. */
export interface ApplyOptions {
    /** The database with the stack's scope and records. */
    readonly database: DatabaseConnection;
    /** The object types the host applies declarations of. */
    readonly objects: readonly ObjectType[];
    /** The installation and package applying the stack. */
    readonly manager: Omit<Manager, "name">;
    /** The scope containing the declared records. */
    readonly scope: string;
    /** The stack's document, keyed by collection. */
    readonly document: Readonly<Record<string, unknown>>;
    /** Report the changes to records in other databases without writing them. */
    readonly isDry?: boolean;
    /** Further policies of declared objects. */
    readonly policies?: readonly Policy[];
    /** The directory with the claims of unique indexes, in the global database. */
    readonly directory?: Directory;
    /** The object server running the declarations' system calls. */
    readonly server?: Pick<ObjectServer, "invoke">;
    /** Open the build of a package's release in the scope, the installation's when given. */
    readonly release?: Stack["release"];
}

/** An object type beside how its declarations become records. */
type Declared = ObjectDeclaration & { readonly object: ObjectType };

/** A stack applied in one scope: its manager's document, and the lookups and system methods its declarations use. */
export class Stack {
    /** The scope containing the declared records, such as a space identifier. */
    readonly scope: string;
    /** The installation and package applying the stack. */
    readonly manager: Omit<Manager, "name">;
    /** The stack's document, keyed by collection. */
    readonly document: Readonly<Record<string, unknown>>;
    /** The time the stack is applied, in UTC milliseconds. */
    readonly now: number;
    /** The policies validating declared access. */
    readonly authorizer: Authorizer;
    /** The transaction writing one object type's records, the stack's database outside one. */
    readonly database: DatabaseConnection;
    /** What the host applies the stack with. */
    readonly #options: ApplyOptions;
    /** The declared object types in dependency order. */
    readonly #ordered: readonly Declared[];
    /** The object type whose records the transaction writes, absent outside one. */
    readonly #writing: Declared | undefined;

    /** Keep a stack's inputs, its ordered object types and the transaction of one of them. */
    private constructor(
        options: ApplyOptions,
        ordered: readonly Declared[],
        authorizer: Authorizer,
        now: number,
        database: DatabaseConnection,
        writing?: Declared,
    ) {
        // keep the stack
        this.scope = options.scope;
        this.manager = options.manager;
        this.document = options.document;
        this.now = now;
        this.authorizer = authorizer;

        // keep the transaction and what applies it
        this.database = database;
        this.#options = options;
        this.#ordered = ordered;
        this.#writing = writing;
    }

    /** Apply a stack: write each object type's declared records, then retire undeclared ones, as steps addressed `<type>/<name>`. */
    static async apply(options: ApplyOptions): Promise<Plan> {
        // pair each object type with its declaration
        const declared = options.objects.map((object) => {
            if (object.declaration === undefined) {
                throw new TypeError(`object ${object.name} has no declaration`);
            }

            return { ...object.declaration, object };
        });

        // require the directory for declared objects with indexes
        const indexed = declared.find((entry) => Object.keys(entry.object.indexes).length > 0);
        if (indexed !== undefined && options.directory === undefined) {
            throw new TypeError(
                `object ${indexed.object.name} declares indexes but no directory keeps their claims`,
            );
        }

        // reject collections and entries that no declaration collects, and entries several collect
        const ordered = order(declared);
        for (const [key, value] of Object.entries(options.document)) {
            const readers = ordered.filter((entry) =>
                (entry.keys ?? [entry.object.plural]).includes(key),
            );
            if (readers.length === 0) {
                throw new ObjectError("UNSUPPORTED_DECLARATION", `this host cannot apply ${key}`);
            }
            const collected = readers.map((entry) => Object.keys(collect(entry, options.document)));
            for (const name of Object.keys(value as Record<string, unknown>)) {
                const count = collected.filter((names) => names.includes(name)).length;
                if (count === 0) {
                    throw new ObjectError(
                        "UNSUPPORTED_DECLARATION",
                        `this host cannot apply ${key} ${name}`,
                    );
                } else if (count > 1) {
                    throw new TypeError(`several object types collect ${key} ${name}`);
                }
            }
        }

        // apply in order under the declared object types' policies
        const authorizer = new Authorizer([
            ...options.objects.map((object) => object.policy),
            ...(options.policies ?? []),
        ]);
        const stack = new Stack(options, ordered, authorizer, Date.now(), options.database);

        return stack.#apply();
    }

    /** Resolve an object type the host applies declarations of, by name. */
    object(name: string): ObjectType {
        return this.#select(name).object;
    }

    /** Find the identifier of the record the manager declared under a name. */
    async find(object: ObjectType | string, name: string): Promise<string | undefined> {
        // read through the transaction when the record lives in its database
        const target = this.#select(object);
        const root = this.#writing?.database ?? this.#options.database;
        const targetRoot = target.database ?? this.#options.database;
        const database = targetRoot === root ? this.database : targetRoot;

        // match by manager and name
        const table = target.object.table as ManagedTable;
        const [row] = (await database
            .select({ id: table.id })
            .from(table)
            .where(
                and(
                    eq(table.scope, this.scope),
                    eq(table.managerInstallationId, this.manager.installationId),
                    eq(table.managerPackageId, this.manager.packageId),
                    eq(table.managerName, name),
                ),
            )) as { id: string }[];

        return row?.id;
    }

    /** Find the identifier of the record declared under a name, or reject the declaration. */
    async require(object: ObjectType | string, name: string): Promise<string> {
        const id = await this.find(object, name);
        if (id === undefined) {
            const type = typeof object === "string" ? object : object.name;
            throw new ObjectError("INVALID_DECLARATION", `unknown ${type}: ${name}`);
        }

        return id;
    }

    /** Stop applying until a dependency is ready, reporting why. */
    wait(message: string): never {
        throw new Waiting(message);
    }

    /** Run a system method in the transaction and scope. */
    invoke(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): Promise<unknown> {
        // require the server running system calls
        const { server } = this.#options;
        if (server === undefined) {
            throw new TypeError(
                `the stack invokes ${object.name}.${name}, but no object server runs it`,
            );
        }

        return server.invoke(this.database, this.scope, object, name, input, this.now);
    }

    /** Open the build of a package's release in the scope, the installation's when given. */
    release(packageId: PackageId, installation?: Identifier<"installation">): Promise<BuildReader> {
        // require the host opening releases
        const { release } = this.#options;
        if (release === undefined) {
            throw new TypeError(
                `the stack opens the release of ${packageId}, but no host opens releases`,
            );
        }

        return release(packageId, installation);
    }

    /** Write each object type's declarations, then retire undeclared records. */
    async #apply(): Promise<Plan> {
        // write each type in order
        const steps: Step[] = [];
        const resolved = new Map<Declared, ReadonlyMap<string, unknown>>();
        for (const declaration of this.#ordered) {
            try {
                await this.#transact(declaration, async (stack) => {
                    // resolve, then write the difference
                    const desired = new Map<string, unknown>();
                    for (const [name, declared] of Object.entries(
                        collect(declaration, this.document),
                    )) {
                        desired.set(
                            name,
                            declaration.resolve
                                ? await declaration.resolve(name, declared, stack)
                                : declared,
                        );
                    }
                    resolved.set(declaration, desired);
                    steps.push(...(await stack.#write(declaration, desired)));
                });
            } catch (error) {
                // stop at a wait
                if (error instanceof Waiting) {
                    return { steps, deferred: error.message };
                }
                throw error;
            }
        }

        // retire in reverse order
        for (const declaration of [...this.#ordered].reverse()) {
            await this.#transact(declaration, async (stack) => {
                steps.push(...(await stack.#retire(declaration, resolved.get(declaration)!)));
            });
        }

        return { steps };
    }

    /** Run a step in a transaction on an object type's database, reserving and confirming its index keys. */
    async #transact(declaration: Declared, step: (stack: Stack) => Promise<void>): Promise<void> {
        // pick the database and the directory
        const options = this.#options;
        const root = declaration.database ?? options.database;
        const directory = options.isDry === true ? undefined : options.directory;
        const requestId = crypto.randomUUID();
        let reservation: Reservation | undefined;
        try {
            reservation = await root.transaction(async (database) => {
                // guard the scope chain and refuse a moved scope
                const chain = await Scope.chain(Snapshot.live(database), this.scope);
                await Scope.guard(
                    database,
                    chain.map((link) => link.object.id),
                );
                const moved = chain.find((link) => link.movedTo !== undefined);
                if (moved !== undefined) {
                    throw Moved.error({ scope: moved.object.id, cell: moved.movedTo! });
                }

                // run the step in the transaction
                await step(
                    new Stack(
                        options,
                        this.#ordered,
                        this.authorizer,
                        this.now,
                        database,
                        declaration,
                    ),
                );

                // reserve the names the written objects claim
                return (
                    directory &&
                    (await Reservation.open(directory, database, options.objects, requestId))
                );
            });
        } catch (error) {
            // release the reserved names
            await directory?.release(requestId);
            throw error;
        }

        // confirm the reserved names
        await reservation?.confirm();
    }

    /** Select an object type's declaration by type or name. */
    #select(object: ObjectType | string): Declared {
        const target = this.#ordered.find((entry) =>
            typeof object === "string" ? entry.object.name === object : entry.object.same(object),
        );
        if (!target) {
            const plural = typeof object === "string" ? object : object.plural;
            throw new ObjectError("UNSUPPORTED_DECLARATION", `this host cannot apply ${plural}`);
        }

        return target;
    }

    /** Read the manager's records of one object type in the scope, by declaration name. */
    async #records(declaration: Declared): Promise<Map<string, Select<ManagedTable>>> {
        const table = declaration.object.table as ManagedTable;
        const rows = (await this.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.scope, this.scope),
                    eq(table.managerInstallationId, this.manager.installationId),
                    eq(table.managerPackageId, this.manager.packageId),
                    "purgedAt" in table[TABLE].columns
                        ? isNull((table as ManagedTable & Record<string, Column>).purgedAt!)
                        : undefined,
                ),
            )) as Select<ManagedTable>[];

        return new Map(rows.map((row) => [row.managerName as string, row]));
    }

    /** Whether to only report an object type's changes. */
    #isDry(declaration: Declared): boolean {
        return this.#options.isDry === true && declaration.database !== undefined;
    }

    /** Create missing records and update changed attached ones. */
    async #write(declaration: Declared, desired: ReadonlyMap<string, unknown>): Promise<Step[]> {
        // read existing records
        const isDry = this.#isDry(declaration);
        const { database, manager, now } = this;
        const table = declaration.object.table as ManagedTable;
        const existing = await this.#records(declaration);
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        const steps: Step[] = [];
        for (const [name, resolved] of desired) {
            // create a missing record
            const row = existing.get(name) as Record<string, unknown> | undefined;
            const values = declaration.values(name, resolved, this) as Record<string, unknown>;
            requireColumns(declaration.object, values);
            const changed = row === undefined ? {} : difference(row, values);
            if (!row) {
                const target = Address.join(declaration.object.name, name);
                steps.push({ action: "create", target, risk: "safe", detail: "declare" });
                if (!isDry) {
                    const created = {
                        id: `${declaration.object.identity}-${v7()}`,
                        createdAt: now,
                        updatedAt: now,
                        ...values,
                        scope: this.scope,
                        ...Manager.values({ ...manager, name }),
                    } as Insert<Table>;
                    await database.insert(table).values(created);
                    const id = (created as Record<string, unknown>).id;
                    await declaration.written?.(this, (await read(database, table, id))!, resolved);
                }
            }
            // update a changed attached record
            else if (
                Manager.isManaging(row) &&
                (Object.keys(changed).length > 0 ||
                    (isDeletable && row.deletionRequestedAt !== null) ||
                    (await declaration.changed?.(database, row as never, resolved)))
            ) {
                const fields = {
                    ...changed,
                    ...(isDeletable && row.deletionRequestedAt !== null
                        ? { deletionRequestedAt: { before: row.deletionRequestedAt, after: null } }
                        : {}),
                };
                steps.push({
                    action: "update",
                    target: Address.join(declaration.object.name, name),
                    risk: "safe",
                    detail: Object.keys(fields).length > 0 ? "change declared fields" : "refresh",
                    fields: fields as Step["fields"],
                });
                if (!isDry) {
                    await database
                        .update(table)
                        .set({
                            ...values,
                            ...("generation" in table[TABLE].columns
                                ? { generation: (row.generation as number) + 1 }
                                : {}),
                            ...(isDeletable ? { deletionRequestedAt: null, deletedBy: null } : {}),
                            ...(declaration.touch?.(row as never, now) ?? {
                                revision: (row.revision as number) + 1,
                                updatedAt: now,
                            }),
                        } as Partial<Insert<Table>>)
                        .where(eq(table.id, row.id));
                    await declaration.written?.(
                        this,
                        (await read(database, table, row.id))!,
                        resolved,
                    );
                }
            }
        }

        return steps;
    }

    /** Retire attached records the manager no longer declares. */
    async #retire(declaration: Declared, desired: ReadonlyMap<string, unknown>): Promise<Step[]> {
        // read existing records
        const isDry = this.#isDry(declaration);
        const { database } = this;
        const table = declaration.object.table as ManagedTable;
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        const steps: Step[] = [];
        for (const [name, row] of await this.#records(declaration)) {
            // skip declared, detached and already retiring records
            const record = row as Record<string, unknown>;
            if (
                desired.has(name) ||
                !Manager.isManaging(record) ||
                (isDeletable && record.deletionRequestedAt !== null) ||
                (await declaration.retire?.isRetiring(database, row as never))
            ) {
                continue;
            }
            const target = Address.join(declaration.object.name, name);
            steps.push({ action: "delete", target, risk: "destructive", detail: "retire" });
            if (isDry) {
                continue;
            }

            // retire, request deletion, or delete
            if (declaration.retire) {
                await declaration.retire.start(this, row as never);
            } else if (isDeletable) {
                await database
                    .update(table)
                    .set({ deletionRequestedAt: this.now, deletedBy: null } as Partial<
                        Insert<Table>
                    >)
                    .where(eq(table.id, row.id));
            } else {
                await database.delete(table).where(eq(table.id, row.id));
            }
        }

        return steps;
    }
}

/** Stop applying until a dependency is ready. */
class Waiting extends Error {}

/** Select an object type's declarations from a document. */
function collect(
    declaration: Declared,
    document: Readonly<Record<string, unknown>>,
): Readonly<Record<string, unknown>> {
    if (declaration.collect) {
        return declaration.collect(document);
    }
    const declared = (document[declaration.object.plural] ?? {}) as Record<string, unknown>;
    const schema = declaration.object.declarationSchema;

    return schema
        ? Object.fromEntries(
              Object.entries(declared).map(([name, value]) => [name, schema.parse(value)]),
          )
        : declared;
}

/** Order declarations so each follows the object types it references. */
function order(declarations: readonly Declared[]): Declared[] {
    // read explicit dependencies
    const explicit = new Map(
        declarations.map((entry) => {
            const dependencies = Array.isArray(entry.after) ? entry.after : [];

            return [
                entry,
                dependencies.flatMap((object) =>
                    declarations.filter((other) => other.object.same(object)),
                ),
            ] as const;
        }),
    );

    // expand "every" to declarations not depending on this one
    const dependsOn = (from: Declared, to: Declared, seen = new Set<Declared>()): boolean => {
        if (seen.has(from)) {
            return false;
        }
        seen.add(from);

        return explicit
            .get(from)!
            .some((dependency) => dependency === to || dependsOn(dependency, to, seen));
    };
    const dependencies = new Map(
        declarations.map((entry) => [
            entry,
            entry.after === "every"
                ? declarations.filter(
                      (other) =>
                          other !== entry && other.after !== "every" && !dependsOn(other, entry),
                  )
                : explicit.get(entry)!,
        ]),
    );

    // visit dependencies depth first, rejecting cycles
    const ordered: Declared[] = [];
    const visiting = new Set<Declared>();
    const visit = (declaration: Declared) => {
        if (ordered.includes(declaration)) {
            return;
        }
        if (visiting.has(declaration)) {
            throw new ObjectError(
                "CYCLIC_DECLARATION",
                `declarations of ${declaration.object.plural} depend on themselves`,
            );
        }
        visiting.add(declaration);
        for (const dependency of dependencies.get(declaration)!) {
            visit(dependency);
        }
        visiting.delete(declaration);
        ordered.push(declaration);
    };
    for (const declaration of declarations) {
        visit(declaration);
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
        throw new TypeError(`declarations of ${object.name} write unknown column ${unknown}`);
    }
}
