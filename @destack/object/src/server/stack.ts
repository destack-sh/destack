import { Address, type Plan, type Step } from "@destack/resource";
import { Scope } from "@destack/sync";
import { Authorizer, Manager, type Policy } from "@destack/access";
import {
    and,
    type ColumnValue,
    type DatabaseConnection,
    eq,
    type Insert,
    type Row,
    type ColumnBuilder,
    type TableColumnMap,
    isNull,
    type Select,
    sql,
    TABLE,
    type Table,
} from "@destack/db";
import {
    canonicalize,
    found,
    Identifier,
    schema,
    type JsonObject,
    type JsonValue,
} from "@destack/schema";
import { ObjectError } from "../error/error.ts";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import { isManaging, managedColumns } from "../trait/declarable.ts";
import type { RecordBuilderMap } from "../trait/record.ts";
import type { FieldColumn } from "../field/field.ts";
import type { Directory } from "@destack/directory";
import { Reservation } from "../claim/index.ts";
import { Moved } from "@destack/directory";
import type { ObjectServer } from "./server.ts";
import type { Invoke, Invoker } from "../method/call.ts";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";

/** The columns a declared record has, whichever table keeps it: its identity, revision and manager. */
type ManagedColumns = TableColumnMap<
    Pick<RecordBuilderMap, "id" | "createdAt" | "updatedAt" | "revision"> & {
        scope: ColumnBuilder<FieldColumn<string, true, false>>;
    } & ReturnType<typeof managedColumns>
>;

/** One section of a stack document: declarations by name. */
const DECLARED = schema.record(schema.string(), schema.unknown());

/** A table of declared records, a derived object's or another package's. */
export type ManagedTable = Table<string, ManagedColumns> & ManagedColumns;

/** How a stack's declarations of one object type become its managed records. */
export interface ObjectDeclaration<
    Object extends ObjectType = ObjectType,
    Collected = unknown,
    Resolved = unknown,
> {
    /** The database with the records, the document's database when absent. */
    readonly database?: DatabaseConnection;
    /** The object types applied before and retired after this one, or "every". */
    readonly after?: readonly ObjectType[] | "every";
    /** The document keys read, the object's plural by default. */
    readonly keys?: readonly string[];
    /** Select the declarations from a document, the object's plural collection by default. */
    collect?(document: JsonObject): Readonly<Record<string, Collected>>;
    /** Resolve references within a declaration, returning it unchanged by default. */
    resolve?(name: string, declared: Collected, stack: Stack): Promise<Resolved>;
    /** Map a resolved declaration to the columns it determines. */
    values(name: string, resolved: Resolved, stack: Stack): Partial<Insert<Object["table"]>>;
    /** Write a changed record's revision and update time. */
    touch?(row: Select<Object["table"]>, now: number): Partial<Insert<Object["table"]>>;
    /** Report changes kept outside the record's own columns. */
    changed?(
        database: DatabaseConnection,
        row: Select<Object["table"]>,
        resolved: Resolved,
    ): Promise<boolean>;
    /** Write records a created or updated record owns. */
    written?(stack: Stack, row: Select<Object["table"]>, resolved: Resolved): Promise<void>;
    /** Create or change a declared record through the owner of its table, in place of the stack's own write. */
    keep?(stack: Stack, manager: Manager, resolved: Resolved): Promise<void>;
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
    NonNullable<Object["lifecycle"]["declarationSchema"]>,
] extends [never]
    ? never
    : schema.Infer<NonNullable<Object["lifecycle"]["declarationSchema"]>>;

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
    readonly document: JsonObject;
    /** Report the changes to records in other databases without writing them. */
    readonly isDry?: boolean;
    /** Further policies of declared objects. */
    readonly policies?: readonly Policy[];
    /** The directory with the claims of unique indexes, kept by the account service. */
    readonly directory?: Directory;
    /** The object server running the declarations' system calls. */
    readonly server?: Pick<ObjectServer, "invoker">;
    /** Open the build of a package's release in the scope, the installation's when given. */
    readonly release?: Stack["release"];
}

/** An object type beside how its declarations become records. */
type TypeDeclaration = ObjectDeclaration & { readonly object: ObjectOf<{ table: ManagedTable }> };

/** A stack applied in one scope: its manager's document, and the lookups and system methods its declarations use. */
export class Stack {
    /** The scope containing the declared records, such as a space identifier. */
    readonly scope: string;
    /** The installation and package applying the stack. */
    readonly manager: Omit<Manager, "name">;
    /** The stack's document, keyed by collection. */
    readonly document: JsonObject;
    /** The time the stack is applied, in UTC milliseconds. */
    readonly now: number;
    /** The policies validating declared access. */
    readonly authorizer: Authorizer;
    /** The transaction writing one object type's records, the stack's database outside one. */
    readonly database: DatabaseConnection;
    /** What the host applies the stack with. */
    readonly #options: ApplyOptions;
    /** The declared object types in dependency order. */
    readonly #ordered: readonly TypeDeclaration[];
    /** The object type whose records the transaction writes, absent outside one. */
    readonly #writing: TypeDeclaration | undefined;

    /** Keep a stack's inputs, its ordered object types and the transaction of one of them. */
    private constructor(
        options: ApplyOptions,
        ordered: readonly TypeDeclaration[],
        authorizer: Authorizer,
        now: number,
        database: DatabaseConnection,
        writing?: TypeDeclaration,
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
            // require a declaration on a declarable type
            const declaration = object.declaration;
            const managed = managedType(object);
            if (declaration === undefined || managed === undefined) {
                throw new TypeError(`object ${object.name} has no declaration`);
            }

            return { ...declaration, object: managed };
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
            for (const name of Object.keys(DECLARED.parse(value))) {
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
        const table = target.object.table;
        const [row] = await database
            .select({ id: table.id })
            .from(table)
            .where(
                and(
                    eq(table.scope, this.scope),
                    eq(table.managerInstallationId, this.manager.installationId),
                    eq(table.managerPackageId, this.manager.packageId),
                    eq(table.managerName, name),
                ),
            );

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
        throw new DependencyWait(message);
    }

    /** Call an object type's system methods in the transaction and scope. */
    invoke<Object extends ObjectType>(object: Object): Invoker<Object> {
        return this.invoker()<Object>(object);
    }

    /** Call object types' system methods in the stack's transaction as one function. */
    invoker(): Invoke {
        // refuse every invocation without the server running system calls
        const { server } = this.#options;
        if (server === undefined) {
            return (object) => {
                throw new TypeError(
                    `the stack invokes ${object.name}, but no object server runs it`,
                );
            };
        }

        return server.invoker(this);
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
        const resolved = new Map<TypeDeclaration, ReadonlyMap<string, unknown>>();
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
                if (error instanceof DependencyWait) {
                    return { steps, deferred: error.message };
                }
                throw error;
            }
        }

        // retire in reverse order
        for (const declaration of this.#ordered.toReversed()) {
            await this.#transact(declaration, async (stack) => {
                steps.push(...(await stack.#retire(declaration, found(resolved, declaration))));
            });
        }

        return { steps };
    }

    /** Run a step in a transaction on an object type's database, reserving and confirming its index keys. */
    async #transact(
        declaration: TypeDeclaration,
        step: (stack: Stack) => Promise<void>,
    ): Promise<void> {
        // pick the database and the directory
        const options = this.#options;
        const root = declaration.database ?? options.database;
        const directory = options.isDry === true ? undefined : options.directory;
        const requestId = crypto.randomUUID();
        let reservation: Reservation | undefined;
        try {
            reservation = await root.transaction(async (database) => {
                // guard the scope chain and refuse a moved scope
                const chain = await Scope.guard(database, this.scope);
                for (const link of chain) {
                    if (link.movedTo !== undefined) {
                        throw Moved.error({ scope: link.object.id, cell: link.movedTo });
                    }
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
    #select(object: ObjectType | string): TypeDeclaration {
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
    async #records(declaration: TypeDeclaration): Promise<Map<string, Select<ManagedTable>>> {
        // read the records the manager declared, leaving out purged ones
        const table = declaration.object.table;
        const rows = await this.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.scope, this.scope),
                    eq(table.managerInstallationId, this.manager.installationId),
                    eq(table.managerPackageId, this.manager.packageId),
                    "purgedAt" in table[TABLE].columns
                        ? isNull(table[TABLE].column("purgedAt"))
                        : undefined,
                ),
            );

        // key each record by the name its manager declared it under
        return new Map(
            rows.map((row) => {
                if (row.managerName === null) {
                    throw new TypeError(
                        `${declaration.object.name} ${row.id} has a manager but no name`,
                    );
                }

                return [row.managerName, row];
            }),
        );
    }

    /** Whether to only report an object type's changes. */
    #isDry(declaration: TypeDeclaration): boolean {
        return this.#options.isDry === true && declaration.database !== undefined;
    }

    /** Create missing records and update changed attached ones. */
    async #write(
        declaration: TypeDeclaration,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<Step[]> {
        // read existing records
        const existing = await this.#records(declaration);
        const steps: Step[] = [];
        for (const [name, resolved] of desired) {
            // create a missing record, or update a changed attached record
            const row = existing.get(name);
            const values = declaration.values(name, resolved, this);
            requireColumns(declaration.object, values);
            if (row === undefined) {
                steps.push(await this.#createDeclared(declaration, name, resolved, values));
            } else {
                steps.push(
                    ...(await this.#updateDeclared(declaration, name, resolved, row, values)),
                );
            }
        }

        return steps;
    }

    /** Create the record of a declaration missing one. */
    async #createDeclared(
        declaration: TypeDeclaration,
        name: string,
        resolved: unknown,
        values: ReturnType<TypeDeclaration["values"]>,
    ): Promise<Step> {
        // plan the creation
        const { database, manager, now } = this;
        const target = Address.join(declaration.object.name, name);
        const step: Step = { action: "create", target, risk: "safe", detail: "declare" };
        if (this.#isDry(declaration)) {
            return step;
        }

        // keep the record through its table's owner, or insert it under the manager
        if (declaration.keep) {
            await declaration.keep(this, { ...manager, name }, resolved);

            return step;
        }
        const table = declaration.object.table;
        const columns: Table = table;
        const id = declaration.object.generateId();
        await database.insert(columns).values({
            id,
            createdAt: now,
            updatedAt: now,
            ...values,
            scope: this.scope,
            ...Manager.values({ ...manager, name }),
        });
        await declaration.written?.(this, await read(database, table, id), resolved);

        return step;
    }

    /** Update an attached record whose declaration changed, restoring it from a deletion request. */
    async #updateDeclared(
        declaration: TypeDeclaration,
        name: string,
        resolved: unknown,
        row: Select<ManagedTable>,
        values: ReturnType<TypeDeclaration["values"]>,
    ): Promise<Step[]> {
        // skip a detached or unchanged record
        const { database } = this;
        const table = declaration.object.table;
        const changed = difference(table, row, values);
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        const record: Row = row;
        const deletion = isDeletable
            ? jsonOf(table, "deletionRequestedAt", record["deletionRequestedAt"])
            : null;
        if (
            !isManaging(row) ||
            (Object.keys(changed).length === 0 &&
                deletion === null &&
                (await declaration.changed?.(database, row, resolved)) !== true)
        ) {
            return [];
        }

        // plan the update of the changed fields
        const fields = {
            ...changed,
            ...(deletion === null
                ? {}
                : { deletionRequestedAt: { before: deletion, after: null } }),
        };
        const step: Step = {
            action: "update",
            target: Address.join(declaration.object.name, name),
            risk: "safe",
            detail: Object.keys(fields).length > 0 ? "change declared fields" : "refresh",
            fields,
        };
        if (this.#isDry(declaration)) {
            return [step];
        }

        // keep the record through its table's owner, or write the declared values as a new revision
        if (declaration.keep) {
            await declaration.keep(this, { ...this.manager, name }, resolved);

            return [step];
        }
        await this.#rewrite(declaration, row, values);
        await declaration.written?.(this, await read(database, table, row.id), resolved);

        return [step];
    }

    /** Write a declaration's values over an attached record as a new revision, lifting a deletion request. */
    async #rewrite(
        declaration: TypeDeclaration,
        row: Select<ManagedTable>,
        values: ReturnType<TypeDeclaration["values"]>,
    ): Promise<void> {
        // write the values with the next generation and revision
        const table = declaration.object.table;
        const columns: Table = table;
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        await this.database
            .update(columns)
            .set({
                ...values,
                ...("generation" in table[TABLE].columns
                    ? { generation: sql`${table[TABLE].column("generation")} + 1` }
                    : {}),
                ...(isDeletable ? { deletionRequestedAt: null, deletedBy: null } : {}),
                ...(declaration.touch?.(row, this.now) ?? {
                    revision: row.revision + 1,
                    updatedAt: this.now,
                }),
            })
            .where(eq(table.id, row.id));
    }

    /** Retire attached records the manager no longer declares. */
    async #retire(
        declaration: TypeDeclaration,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<Step[]> {
        // read existing records
        const isDry = this.#isDry(declaration);
        const { database } = this;
        const table = declaration.object.table;
        const columns: Table = table;
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        const steps: Step[] = [];
        for (const [name, row] of await this.#records(declaration)) {
            // skip declared, detached and already retiring records
            const record: Row = row;
            if (
                desired.has(name) ||
                !isManaging(row) ||
                (isDeletable && record["deletionRequestedAt"] !== null) ||
                (await declaration.retire?.isRetiring(database, row)) === true
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
                await declaration.retire.start(this, row);
            } else if (isDeletable) {
                await database
                    .update(columns)
                    .set({ deletionRequestedAt: this.now, deletedBy: null })
                    .where(eq(table.id, row.id));
            } else {
                await database.delete(table).where(eq(table.id, row.id));
            }
        }

        return steps;
    }
}

/** Stop applying until a dependency is ready. */
class DependencyWait extends Error {}

/** Select an object type's declarations from a document. */
function collect(
    declaration: TypeDeclaration,
    document: JsonObject,
): Readonly<Record<string, unknown>> {
    if (declaration.collect) {
        return declaration.collect(document);
    }
    const declared = DECLARED.parse(document[declaration.object.plural] ?? {});
    const declarationSchema = declaration.object.lifecycle.declarationSchema;

    return declarationSchema
        ? Object.fromEntries(
              Object.entries(declared).map(([name, value]) => [
                  name,
                  declarationSchema.parse(value),
              ]),
          )
        : declared;
}

/** Order declarations so each follows the object types it references. */
function order(declarations: readonly TypeDeclaration[]): TypeDeclaration[] {
    // read explicit dependencies
    const explicit = explicitDependencies(declarations);

    // expand "every" to declarations not depending on this one
    const dependencies = new Map(
        declarations.map((entry) => [
            entry,
            entry.after === "every"
                ? declarations.filter(
                      (other) =>
                          other !== entry &&
                          other.after !== "every" &&
                          !dependsOn(explicit, other, entry, new Set()),
                  )
                : found(explicit, entry),
        ]),
    );

    // visit dependencies depth first, rejecting cycles
    const ordered: TypeDeclaration[] = [];
    const visiting = new Set<TypeDeclaration>();
    for (const declaration of declarations) {
        visit(declaration, dependencies, ordered, visiting);
    }

    return ordered;
}

/** Read the declarations each declaration names to run after. */
function explicitDependencies(
    declarations: readonly TypeDeclaration[],
): Map<TypeDeclaration, readonly TypeDeclaration[]> {
    return new Map(
        declarations.map((entry) => {
            const dependencies =
                entry.after === undefined || entry.after === "every" ? [] : entry.after;

            return [
                entry,
                dependencies.flatMap((object) =>
                    declarations.filter((other) => other.object.same(object)),
                ),
            ] as const;
        }),
    );
}

/** Decide whether one declaration depends on another through explicit dependencies. */
function dependsOn(
    explicit: ReadonlyMap<TypeDeclaration, readonly TypeDeclaration[]>,
    from: TypeDeclaration,
    to: TypeDeclaration,
    seen: Set<TypeDeclaration>,
): boolean {
    // stop at a declaration seen before
    if (seen.has(from)) {
        return false;
    }
    seen.add(from);

    return found(explicit, from).some(
        (dependency) => dependency === to || dependsOn(explicit, dependency, to, seen),
    );
}

/** Add a declaration after its dependencies, rejecting cycles. */
function visit(
    declaration: TypeDeclaration,
    dependencies: ReadonlyMap<TypeDeclaration, readonly TypeDeclaration[]>,
    ordered: TypeDeclaration[],
    visiting: Set<TypeDeclaration>,
): void {
    // skip a declaration added before
    if (ordered.includes(declaration)) {
        return;
    }

    // refuse a declaration depending on itself
    if (visiting.has(declaration)) {
        throw new ObjectError(
            "CYCLIC_DECLARATION",
            `declarations of ${declaration.object.plural} depend on themselves`,
        );
    }

    // add the dependencies first
    visiting.add(declaration);
    for (const dependency of found(dependencies, declaration)) {
        visit(dependency, dependencies, ordered, visiting);
    }
    visiting.delete(declaration);
    ordered.push(declaration);
}

/** Read one record a declaration just wrote, by identifier. */
async function read(
    database: DatabaseConnection,
    table: ManagedTable,
    id: Identifier<string>,
): Promise<Select<ManagedTable>> {
    const [row] = await database.select().from(table).where(eq(table.id, id));
    if (row === undefined) {
        throw new TypeError(`the written ${table[TABLE].name} ${id} is missing`);
    }

    return row;
}

/** Read an object type as its declarable table types it, absent for a type stacks never declare. */
export function managedType(object: ObjectType): ObjectOf<{ table: ManagedTable }> | undefined;
/**
 * Read an object type as its declarable table types it, by the trait's option.
 *
 * @construct defineObject derives the manager columns into the table of every object whose definition sets `declarable`, which gives it a declaration schema.
 */
export function managedType(object: ObjectType): ObjectType | undefined {
    return object.lifecycle.declarationSchema === undefined ? undefined : object;
}

/** Collect the declared values that differ from a row, in their JSON form. */
function difference(
    table: Table,
    row: Row,
    values: Partial<Insert<Table>>,
): Record<string, { before: JsonValue; after: JsonValue }> {
    const changes: Record<string, { before: JsonValue; after: JsonValue }> = {};
    for (const [key, value] of Object.entries(values)) {
        const before = jsonOf(table, key, row[key]);
        const after = jsonOf(table, key, value);
        if (canonicalize(before) !== canonicalize(after)) {
            changes[key] = { before, after };
        }
    }

    return changes;
}

/** Write a column value in its JSON form, null for a missing one. */
function jsonOf(table: Table, column: string, value: ColumnValue | undefined): JsonValue {
    return value === undefined || value === null
        ? null
        : table[TABLE].column(column).definition.toJson(value);
}

/** Refuse values naming unknown columns. */
function requireColumns(object: ObjectType, values: object): void {
    const columns = object.table[TABLE].columns;
    const unknown = Object.keys(values).find((key) => !Object.hasOwn(columns, key));
    if (unknown !== undefined) {
        throw new TypeError(`declarations of ${object.name} write unknown column ${unknown}`);
    }
}
