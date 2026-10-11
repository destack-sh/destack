import { Address, type Plan, type Step } from "@destack/resource";
import { Scope, type Subject } from "@destack/sync";
import { Authorizer, Manager, type Policy } from "@destack/access";
import {
    type ColumnValue,
    type DatabaseConnection,
    Transaction,
    type Insert,
    type Row,
    type ColumnBuilder,
    type TableColumnMap,
    type Select,
    type Snapshot,
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
import { type ObjectOf, ObjectType } from "../object/object.ts";
import { isManaging, managedColumns } from "../trait/declarable.ts";
import type { RecordBuilderMap } from "../trait/record.ts";
import type { FieldColumn } from "../field/field.ts";
import { type ClaimStore, Reservation } from "../claim/store.ts";
import type { ObjectStore } from "./store.ts";
import type { Authorization } from "./authorization.ts";
import { Call, type ObjectCall, type ResultsOf } from "../method/call.ts";
import { Caller } from "@destack/access";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { Claimant } from "../claim/claim.ts";

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
export interface DeclarationHandler<
    Object extends ObjectType = ObjectType,
    Collected = unknown,
    Resolved = unknown,
> {
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
    /** Report changes kept outside the record's own columns. */
    changed?(
        transaction: Transaction,
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
        isRetiring(transaction: Transaction, row: Select<Object["table"]>): Promise<boolean>;
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

/** What a machine applies a stack with. */
export interface ApplyOptions {
    /** The database with the stack's scope and records. */
    readonly database: DatabaseConnection;
    /** The object types the machine applies declarations of. */
    readonly objects: readonly ObjectType[];
    /** The installation and package applying the stack. */
    readonly manager: Omit<Manager, "name">;
    /** The principal the declarations' calls run as, such as the space applying an installation's stack. */
    readonly as: Subject;
    /** The scope containing the declared records. */
    readonly scope: string;
    /** The time the stack applies at, in UTC epoch milliseconds. */
    readonly now: number;
    /** The stack's document, keyed by collection. */
    readonly document: JsonObject;
    /** Further policies of declared objects. */
    readonly policies?: readonly Policy[];
    /** The claims of unique indexes, kept by the account service. */
    readonly claims?: ClaimStore;
    /** The object store running the declarations' calls, applying each record where its rows live. */
    readonly store?: Pick<ObjectStore, "mutate" | "admit">;
    /** Open the build of a package's release in the scope through a snapshot, the installation's when given. */
    readonly release?: (
        snapshot: Snapshot,
        packageId: PackageId,
        installation?: Identifier<"installation">,
    ) => Promise<BuildReader>;
}

/** An object type beside how its declarations become records. */
type TypeDeclaration = DeclarationHandler & { readonly object: ObjectOf<{ table: ManagedTable }> };

/** A stack applied in one scope: its manager's document, and the lookups and calls its declarations use. */
export class Stack {
    /** The scope containing the declared records, such as a space identifier. */
    readonly scope: string;
    /** The installation and package applying the stack. */
    readonly manager: Omit<Manager, "name">;
    /** The principal the declarations' calls run as, such as the space applying an installation's stack. */
    readonly as: Subject;
    /** The stack's document, keyed by collection. */
    readonly document: JsonObject;
    /** The time the stack is applied, in UTC milliseconds. */
    readonly now: number;
    /** The policies validating declared access. */
    readonly authorizer: Authorizer;
    /** The transaction the stack's records are written through. */
    readonly transaction: Transaction;
    /** What the machine applies the stack with. */
    readonly #options: ApplyOptions;
    /** The declared object types in dependency order. */
    readonly #ordered: readonly TypeDeclaration[];

    /** Keep a stack's inputs, its ordered object types and the transaction writing them. */
    private constructor(
        options: ApplyOptions,
        ordered: readonly TypeDeclaration[],
        authorizer: Authorizer,
        now: number,
        transaction: Transaction,
    ) {
        // keep the stack
        this.scope = options.scope;
        this.manager = options.manager;
        this.as = options.as;
        this.document = options.document;
        this.now = now;
        this.authorizer = authorizer;

        // keep the transaction and what applies it
        this.transaction = transaction;
        this.#options = options;
        this.#ordered = ordered;
    }

    /** Plan a stack: the steps applying it would take, as steps addressed `<type>/<name>`, writing nothing. */
    static async plan(options: ApplyOptions): Promise<Plan> {
        const ordered = Stack.#declare(options);
        const transaction = await Transaction.open(options.database);

        return (await Stack.#open(options, ordered, transaction)).#apply();
    }

    /** Apply a stack: write each object type's declared records and retire undeclared ones in one transaction, reserving and confirming the names they claim, as steps addressed `<type>/<name>`. */
    static async apply(options: ApplyOptions): Promise<Plan> {
        // run the steps over one transaction, again while it loses a race, reserving the names the written objects claim
        const ordered = Stack.#declare(options);
        const { claims } = options;
        const requestId = crypto.randomUUID();
        let reservation: Reservation | undefined;
        let plan: Plan;
        try {
            plan = await Transaction.run(options.database, async (transaction) => {
                // apply the steps and reserve the claimed names as the transaction commits
                const stack = await Stack.#open(options, ordered, transaction);
                const applied = await stack.#apply();
                const claimants = await Claimant.written(transaction, options.objects);
                transaction.record(async (committing) => {
                    reservation =
                        claims &&
                        (await Reservation.reserve(claims, committing, claimants, options.objects, requestId));
                });

                return applied;
            });
        }
        // release the names once the commit fails
        catch (error) {
            await claims?.release(requestId);
            throw error;
        }

        // confirm the reserved names
        await reservation?.confirm();

        return plan;
    }

    /** Order a stack's declarations, refusing collections and entries no declaration collects and entries several collect. */
    static #declare(options: ApplyOptions): TypeDeclaration[] {
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

        // require claims for declared objects with indexes
        const indexed = declared.find((entry) => Object.keys(entry.object.indexes).length > 0);
        if (indexed !== undefined && options.claims === undefined) {
            throw new TypeError(
                `object ${indexed.object.name} declares indexes but no claim store keeps their names`,
            );
        }

        // reject collections and entries that no declaration collects, and entries several collect
        const ordered = order(declared);
        for (const [key, value] of Object.entries(options.document)) {
            const readers = ordered.filter((entry) =>
                (entry.keys ?? [entry.object.plural]).includes(key),
            );
            if (readers.length === 0) {
                throw new ObjectError("UNSUPPORTED_DECLARATION", `this server applies no ${key}`);
            }
            const collected = readers.map((entry) => Object.keys(collect(entry, options.document)));
            for (const name of Object.keys(DECLARED.parse(value))) {
                const count = collected.filter((names) => names.includes(name)).length;
                if (count === 0) {
                    throw new ObjectError(
                        "UNSUPPORTED_DECLARATION",
                        `this server applies no ${key} ${name}`,
                    );
                } else if (count > 1) {
                    throw new TypeError(`several object types collect ${key} ${name}`);
                }
            }
        }

        return ordered;
    }

    /** Open a stack over a transaction of its database with its ordered declarations, refusing a moved scope. */
    static async #open(
        options: ApplyOptions,
        ordered: TypeDeclaration[],
        transaction: Transaction,
    ): Promise<Stack> {
        // apply in order under the declared object types' policies
        const authorizer = new Authorizer([
            ...options.objects.map((object) => object.policy),
            ...(options.policies ?? []),
        ]);

        // refuse a moved scope
        for (const link of await Scope.chain(transaction.view(), options.scope)) {
            if (link.movedTo !== undefined) {
                throw new ServiceError("MISDIRECTED_REQUEST", {
                    message: `${link.object.id} moves to ${link.movedTo}`,
                    data: { scope: link.object.id, machine: link.movedTo },
                });
            }
        }

        return new Stack(options, ordered, authorizer, options.now, transaction);
    }

    /** Resolve an object type the machine applies declarations of, by name. */
    object(name: string): ObjectType {
        return this.#select(name).object;
    }

    /** Find the declared object type whose objects an identifier refers to, refusing an identifier none matches. */
    typeOf(id: string): ObjectType {
        const target = this.#ordered.find((entry) => entry.object.identifies(id));
        if (!target) {
            throw new ObjectError(
                "UNSUPPORTED_DECLARATION",
                `this server applies no type of ${id}`,
            );
        }

        return target.object;
    }

    /** Find the identifier of the record the manager declared under a name. */
    async find(object: ObjectType | string, name: string): Promise<string | undefined> {
        // match by manager and name
        const table = this.#select(object).object.table;
        const [row] = await this.transaction.images(table, {
            scope: this.scope,
            managerInstallationId: this.manager.installationId,
            managerPackageId: this.manager.packageId,
            managerName: name,
        });

        return row?.id;
    }

    /** Find the identifier of the record declared under a name, or reject the declaration. */
    async require(object: ObjectType | string, name: string): Promise<string> {
        // wait for a record kept where its rows live until its copy arrives, and refuse an unknown one
        const id = await this.find(object, name);
        const target = this.#select(object).object;
        if (id === undefined && (await target.isCopy(this.transaction.view(), this.scope))) {
            this.wait(`${target.name} ${name} waits for its copy`);
        } else if (id === undefined) {
            throw new ObjectError("INVALID_DECLARATION", `unknown ${target.name}: ${name}`);
        }

        return id;
    }

    /** Stop applying until a dependency is ready, reporting why. */
    wait(message: string): never {
        throw new DependencyWait(message);
    }

    /** Run calls as the stack's principal in the stack's transaction, each result in order: copied rows' changes go toward their home once it commits. */
    mutate<const Calls extends readonly ObjectCall[]>(calls: Calls): Promise<ResultsOf<Calls>>;
    /**
     * Run calls as the stack's principal in the stack's transaction.
     *
     * @construct each result is the result its call's method answers, which ObjectCall carries from type.call.
     */
    mutate(calls: readonly ObjectCall[]): Promise<unknown[]> {
        return this.#store().mutate(
            this.caller,
            { id: RequestId.create(this.now), calls },
            { transaction: this.transaction, scope: this.scope },
        );
    }

    /** The caller the stack's calls run as: its principal, at the stack's time. */
    get caller(): Caller {
        return Caller.of(this.as, this.now);
    }

    /** Open the build of a package's release in the scope, the installation's when given, reading through the step's database. */
    release(packageId: PackageId, installation?: Identifier<"installation">): Promise<BuildReader> {
        // require the machine opening releases
        const { release } = this.#options;
        if (release === undefined) {
            throw new TypeError(
                `the stack opens the release of ${packageId}, but no machine opens releases`,
            );
        }

        return release(this.transaction.view(), packageId, installation);
    }

    /** Write each object type's declarations and retire undeclared records. */
    async #apply(): Promise<Plan> {
        // write each type in order, resolving before writing the difference
        const steps: Step[] = [];
        const resolved = new Map<TypeDeclaration, ReadonlyMap<string, unknown>>();
        for (const declaration of this.#ordered) {
            try {
                const desired = new Map<string, unknown>();
                for (const [name, declared] of Object.entries(
                    collect(declaration, this.document),
                )) {
                    desired.set(
                        name,
                        declaration.resolve
                            ? await declaration.resolve(name, declared, this)
                            : declared,
                    );
                }
                resolved.set(declaration, desired);
                steps.push(...(await this.#write(declaration, desired)));
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
            steps.push(...(await this.#retire(declaration, found(resolved, declaration))));
        }

        return { steps };
    }

    /** Select an object type's declaration by type or name. */
    #select(object: ObjectType | string): TypeDeclaration {
        const target = this.#ordered.find((entry) =>
            typeof object === "string" ? entry.object.name === object : entry.object.same(object),
        );
        if (!target) {
            const plural = typeof object === "string" ? object : object.plural;
            throw new ObjectError("UNSUPPORTED_DECLARATION", `this server applies no ${plural}`);
        }

        return target;
    }

    /** Read the manager's records of one object type in the scope, by declaration name. */
    async #records(declaration: TypeDeclaration): Promise<Map<string, Select<ManagedTable>>> {
        // read the records the manager declared, leaving out purged ones
        const table = declaration.object.table;
        const managed = await this.transaction.rows(table, {
            scope: this.scope,
            managerInstallationId: this.manager.installationId,
            managerPackageId: this.manager.packageId,
        });
        const rows = managed.filter((row) => {
            const record: Row = row;

            return !("purgedAt" in table[TABLE].columns) || record["purgedAt"] === null;
        });

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
        const { transaction, manager } = this;
        const target = Address.join(declaration.object.name, name);
        const step: Step = { action: "create", target, risk: "safe", detail: "declare" };

        // keep the record through its table's owner, or apply it under a fresh identifier where its rows live
        const { object } = declaration;
        if (declaration.keep) {
            await declaration.keep(this, { ...manager, name }, resolved);

            return step;
        }
        const id = object.generateId();
        await this.#applyRecord(object, name, values, id);
        if (!(await object.isCopy(transaction.view(), this.scope))) {
            await declaration.written?.(this, await read(transaction, object.table, id), resolved);
        }

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
        const { transaction } = this;
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
                (await declaration.changed?.(transaction, row, resolved)) !== true)
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
        // keep the record through its table's owner, or apply the declared values as its next revision where its rows live
        if (declaration.keep) {
            await declaration.keep(this, { ...this.manager, name }, resolved);

            return [step];
        }
        await this.#applyRecord(declaration.object, name, values);
        if (!(await declaration.object.isCopy(transaction.view(), this.scope))) {
            await declaration.written?.(this, await read(transaction, table, row.id), resolved);
        }

        return [step];
    }

    /** Apply a declared record's values where its rows live, under a fresh identifier for a new record. */
    async #applyRecord(
        object: ObjectType,
        name: string,
        values: ReturnType<TypeDeclaration["values"]>,
        id?: string,
    ): Promise<void> {
        const table = object.table;
        const json = Object.fromEntries(
            Object.entries(values).map(([column, value]) => [column, jsonOf(table, column, value)]),
        );
        await this.mutate([
            Call.record(object, "apply", {
                ...(id === undefined ? {} : { id }),
                manager: { ...this.manager, name },
                values: json,
            }),
        ]);
    }

    /** Admit the stack's principal in its scope inside the stack's transaction, as its access declarations write. */
    authorization(): Promise<Authorization> {
        return this.#store().admit(this.transaction, this.scope, this.caller);
    }

    /** Require the object store running the declarations' calls. */
    #store(): Pick<ObjectStore, "mutate" | "admit"> {
        const { store } = this.#options;
        if (store === undefined) {
            throw new TypeError("the stack runs calls, but no object store runs them");
        }

        return store;
    }

    /** Retire attached records the manager no longer declares. */
    async #retire(
        declaration: TypeDeclaration,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<Step[]> {
        // read existing records
        const { transaction } = this;
        const table = declaration.object.table;
        const isDeletable = "deletionRequestedAt" in table[TABLE].columns;
        const steps: Step[] = [];
        for (const [name, row] of await this.#records(declaration)) {
            // skip declared, detached and already retiring records
            const record: Row = row;
            if (
                desired.has(name) ||
                !isManaging(row) ||
                (isDeletable && record["deletionRequestedAt"] !== null) ||
                (await declaration.retire?.isRetiring(transaction, row)) === true
            ) {
                continue;
            }
            const target = Address.join(declaration.object.name, name);
            steps.push({ action: "delete", target, risk: "destructive", detail: "retire" });

            // retire through the declaration, or where the record's rows live
            if (declaration.retire) {
                await declaration.retire.start(this, row);
            } else {
                await this.mutate([
                    Call.record(declaration.object, "apply", {
                        manager: { ...this.manager, name },
                        values: null,
                    }),
                ]);
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

/** Read the declarations each declaration lists to run after. */
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
    transaction: Transaction,
    table: ManagedTable,
    id: Identifier<string>,
): Promise<Select<ManagedTable>> {
    const row = await transaction.row(table, { id });
    if (row === null) {
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

/** Refuse values for unknown columns. */
function requireColumns(object: ObjectType, values: object): void {
    const columns = object.table[TABLE].columns;
    const unknown = Object.keys(values).find((key) => !Object.hasOwn(columns, key));
    if (unknown !== undefined) {
        throw new TypeError(`declarations of ${object.name} write unknown column ${unknown}`);
    }
}
