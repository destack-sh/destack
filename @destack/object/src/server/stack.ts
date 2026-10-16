import { Address, type Plan, type Step } from "@destack/resource";
import { Scope, type Subject, type Call } from "@destack/sync";
import { AccessContext, Authorizer, Manager, type Policy } from "@destack/access";
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
    Table,
} from "@destack/db";
import {
    canonicalize,
    found,
    Identifier,
    JsonValue,
    present,
    schema,
    type JsonObject,
} from "@destack/schema";
import { ObjectError } from "../error/error.ts";
import { type ObjectOf, ObjectType } from "../object/object.ts";
import type { ObjectDescription } from "../object/description.ts";
import { declarable, managedColumns } from "../trait/declarable.ts";
import type { RecordBuilderMap } from "../trait/record.ts";
import type { FieldColumn } from "../field/field.ts";
import { type ClaimHome, Reservation } from "../claim/reservation.ts";
import type { ObjectStore } from "./store.ts";
import type { Authorization } from "./authorization.ts";
import { type ResultsOf } from "../method/context.ts";
import { Caller } from "@destack/access";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { Claimant } from "../claim/claim.ts";

/** The columns of a declared record in any table: identity, revision and manager. */
type ManagedColumns = TableColumnMap<
    Pick<RecordBuilderMap, "id" | "createdAt" | "updatedAt" | "revision"> & {
        scope: ColumnBuilder<FieldColumn<string, true, false>>;
    } & ReturnType<typeof managedColumns>
>;

/** One section of a stack document: declarations by name. */
const DECLARED = schema.record(schema.string(), schema.unknown());

/** The fields of a described type's definition that place its declarations in a stack document. */
const DESCRIBED = schema.looseObject({
    /** The type's name. */
    name: schema.string(),
    /** The type's plural, the document key of its declarations. */
    plural: schema.string(),
    /** How stacks declare the type, absent on a type stacks do not declare. */
    declarable: schema.unknown().exactOptional(),
    /** The type its records nest in, whose declarations contain the type's own. */
    nested: schema.unknown().exactOptional(),
});

/** The columns a declaration determines. */
type DeclaredValues = Partial<Insert<Table>>;

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
    /** Select the declarations from a document. */
    readonly collect?: (
        document: JsonObject,
    ) => Readonly<Record<string, Collected>> | Promise<Readonly<Record<string, Collected>>>;
    /** Resolve references within a declaration, returning it unchanged by default. */
    resolve?(name: string, declared: Collected, stack: Stack): Promise<Resolved>;
    /** The columns a declaration sets once, refusing another value later. */
    readonly immutable?: readonly (keyof Insert<Object["table"]> & string)[];
    /** Map a resolved declaration to the columns it determines, itself by default. */
    values?(name: string, resolved: Resolved, stack: Stack): Partial<Insert<Object["table"]>>;
    /** Report changes stored outside the record's own columns. */
    changed?(
        transaction: Transaction,
        row: Select<Object["table"]>,
        resolved: Resolved,
    ): Promise<boolean>;
    /** Write records a created or updated record owns. */
    written?(stack: Stack, row: Select<Object["table"]>, resolved: Resolved): Promise<void>;
    /** Create or change a declared record through its table's owner, instead of the stack. */
    upsert?(stack: Stack, manager: Manager, resolved: Resolved): Promise<void>;
    /** Retire records through their own deletion, archive them, or retain them as they are. */
    readonly retire?:
        | "archived"
        | {
              /** Report whether a record is already retiring. */
              isRetiring(transaction: Transaction, row: Select<Object["table"]>): Promise<boolean>;
              /** Begin retiring a record the manager no longer declares. */
              start(stack: Stack, row: Select<Object["table"]>): Promise<void>;
          }
        | "retained";
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
    /** The principal running the declarations' calls, such as the space applying a stack. */
    readonly as: Subject;
    /** The scope containing the declared records. */
    readonly scope: string;
    /** The time the stack applies at, in UTC epoch milliseconds. */
    readonly now: number;
    /** The stack's document, keyed by collection. */
    readonly document: JsonObject;
    /** Further policies of declared objects. */
    readonly policies?: readonly Policy[];
    /** The claim type recording the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimHome;
    /** The object store running the declarations' calls, applying records where they live. */
    readonly store: Pick<ObjectStore, "mutate" | "authorizeHere" | "isReplica">;
    /** Open a package release's build in the scope via a snapshot, the installation's if given. */
    readonly release?: (
        snapshot: Snapshot,
        packageId: PackageId,
        installation?: Identifier<"installation">,
    ) => Promise<BuildReader>;
    /** Read the object types the applied declarations install, served once the store reloads. */
    readonly installing?: (snapshot: Snapshot) => Promise<readonly ObjectDescription[]>;
}

/** An object type beside how its declarations become records. */
type TypeDeclaration = DeclarationHandler & { readonly object: ObjectOf<{ table: ManagedTable }> };

/** A stack applied in one scope: its manager's document, and its declarations' lookups. */
export class Stack {
    /** The scope containing the declared records, such as a space identifier. */
    readonly scope: string;
    /** The installation and package applying the stack. */
    readonly manager: Omit<Manager, "name">;
    /** The principal running the declarations' calls, such as the space applying a stack. */
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
    /** The document keys no served type collects, which types the stack installs may collect. */
    readonly #installing: readonly string[];
    /** The scope enclosing the stack's scope, read on first use. */
    #enclosing: Promise<string> | undefined;

    /** Hold a stack's inputs, its ordered object types and the transaction writing them. */
    private constructor(
        options: ApplyOptions,
        declared: Declared,
        authorizer: Authorizer,
        now: number,
        transaction: Transaction,
    ) {
        // hold the stack
        this.scope = options.scope;
        this.manager = options.manager;
        this.as = options.as;
        this.document = options.document;
        this.now = now;
        this.authorizer = authorizer;

        // hold the transaction and what applies it
        this.transaction = transaction;
        this.#options = options;
        this.#ordered = declared.ordered;
        this.#installing = declared.installing;
    }

    /** Plan a stack's steps, addressed `<type>/<name>`, in a transaction its apply commits. */
    static async plan(options: ApplyOptions): Promise<StackPlan> {
        // write the steps into an open transaction
        const declared = await Stack.#declare(options);
        const transaction = await Transaction.open(options.database);

        return Stack.#plan(options, declared, transaction);
    }

    /** Plan and apply a stack in one transaction, again while it loses a race. */
    static async apply(options: ApplyOptions): Promise<Plan> {
        // plan and reserve in each attempt, releasing the names a lost attempt reserved
        const declared = await Stack.#declare(options);
        let reservation: Reservation | undefined;
        try {
            const plan = await Transaction.run(options.database, async (transaction) => {
                // release a lost attempt's names, then plan and reserve this attempt's
                await reservation?.release();
                const planned = await Stack.#plan(options, declared, transaction);
                reservation = await planned.reserve();

                return planned.plan;
            });

            // confirm the reserved names once committed
            await reservation?.confirm();

            return plan;
        } catch (error) {
            // release the names of a failed application
            await reservation?.release();
            throw error;
        }
    }

    /** Write a stack's declared steps into a transaction, planning the ones it waits on. */
    static async #plan(
        options: ApplyOptions,
        declared: Declared,
        transaction: Transaction,
    ): Promise<StackPlan> {
        const stack = await Stack.#open(options, declared, transaction);

        return new StackPlan(options, transaction, await stack.#apply());
    }

    /** Order a stack's declarations, refusing uncollected and multiply collected entries. */
    static async #declare(options: ApplyOptions): Promise<Declared> {
        // pair each object type with its declaration
        const declared = options.objects.map((object) => {
            // require a declaration on a declarable type
            const declaration = object.declaration;
            if (declaration === undefined || !object.isManaged()) {
                throw new TypeError(`object ${object.name} has no declaration`);
            }

            // require an active column on a type archiving undeclared records
            if (declaration.retire === "archived" && !("active" in object.table[TABLE].columns)) {
                throw new TypeError(
                    `object ${object.name} archives records without an active column`,
                );
            }

            return { ...declaration, object };
        });

        // require claims for declared objects with indexes
        const indexed = declared.find((entry) => Object.keys(entry.object.indexes).length > 0);
        if (indexed !== undefined && options.claims === undefined) {
            throw new TypeError(
                `object ${indexed.object.name} declares indexes but has no claim type for their names`,
            );
        }

        // leave collections no declaration collects for the types the stack installs, and reject entries that none or several collect
        const ordered = Stack.#order(declared);
        const installing: string[] = [];
        for (const [key, value] of Object.entries(options.document)) {
            const readers = ordered.filter((entry) =>
                (entry.keys ?? [entry.object.plural]).includes(key),
            );
            if (readers.length === 0 && options.installing === undefined) {
                throw new ObjectError("UNSUPPORTED_DECLARATION", `this server cannot apply ${key}`);
            } else if (readers.length === 0) {
                installing.push(key);
                continue;
            }
            const collected = await Promise.all(
                readers.map(async (entry) =>
                    Object.keys(await Stack.#collect(entry, options.document, ordered)),
                ),
            );
            for (const name of Object.keys(DECLARED.parse(value))) {
                const count = collected.filter((names) => names.includes(name)).length;
                if (count === 0) {
                    throw new ObjectError(
                        "UNSUPPORTED_DECLARATION",
                        `this server cannot apply ${key} ${name}`,
                    );
                } else if (count > 1) {
                    throw new TypeError(`several object types collect ${key} ${name}`);
                }
            }
        }

        return { ordered, installing };
    }

    /** Open a stack over a database transaction with its ordered declarations, refusing moves. */
    static async #open(
        options: ApplyOptions,
        declared: Declared,
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
                throw new CallError("MISDIRECTED_REQUEST", {
                    message: `${link.object.id} moves to ${link.movedTo}`,
                    data: { scope: link.object.id, machine: link.movedTo },
                });
            }
        }

        return new Stack(options, declared, authorizer, options.now, transaction);
    }

    /** Read a nested type's declarations inline under its parent's, named `<parent>/<child>`. */
    static nested(
        object: ObjectType,
        parent: ObjectType,
        document: JsonObject,
    ): [string, unknown][] {
        return Object.entries(DECLARED.parse(document[parent.plural] ?? {})).flatMap(
            ([key, value]) =>
                Object.entries(DECLARED.parse(DECLARED.parse(value)[object.plural] ?? {})).map(
                    ([child, declared]): [string, unknown] => [Address.join(key, child), declared],
                ),
        );
    }

    /** Read the scope enclosing the stack's scope, such as a space's account, once per apply. */
    enclosing(): Promise<string> {
        // read the scope's row once
        this.#enclosing ??= Scope.read(this.transaction.view(), [this.scope]).then(
            ([row]) => present(row, `the scope row of ${this.scope}`).parent,
        );

        return this.#enclosing;
    }

    /** Read the name of a nested declaration's parent, its name before the last segment. */
    static parent(name: string): string {
        // refuse a name declared outside a parent
        const separator = name.lastIndexOf("/");
        if (separator < 0) {
            throw new TypeError(`declaration ${name} has no parent`);
        }

        return name.slice(0, separator);
    }

    /** Resolve an object type the machine applies declarations of, by name. */
    object(name: string): ObjectType {
        return this.#select(name).object;
    }

    /** Find the declared object type of an identifier, refusing unmatched identifiers. */
    findType(id: string): ObjectType {
        const target = this.#ordered.find((entry) => entry.object.identifies(id));
        if (!target) {
            throw new ObjectError(
                "UNSUPPORTED_DECLARATION",
                `this server cannot apply a type of ${id}`,
            );
        }

        return target.object;
    }

    /** Find the identifier of the record the manager declared under a name. */
    async find(object: ObjectType | string, name: string): Promise<string | undefined> {
        // match by manager and name
        const table = this.#select(object).object.table;
        const [row] = await this.transaction
            .view()
            .read(table)
            .findMany({
                where: {
                    scope: this.scope,
                    managerInstallationId: this.manager.installationId,
                    managerPackageId: this.manager.packageId,
                    managerName: name,
                },
            });

        return row?.id;
    }

    /** Find the identifier of the record declared under a name, or reject the declaration. */
    async require(object: ObjectType | string, name: string): Promise<string> {
        // wait for a record's replica to arrive, and refuse an unknown record
        const id = await this.find(object, name);
        const target = this.#select(object).object;
        if (
            id === undefined &&
            (await this.#options.store.isReplica(target, this.transaction.view(), this.scope))
        ) {
            this.wait(`${target.name} ${name} waits for its replica`);
        } else if (id === undefined) {
            throw new ObjectError("INVALID_DECLARATION", `unknown ${target.name}: ${name}`);
        }

        return id;
    }

    /** Stop applying until a dependency is ready, reporting why. */
    wait(message: string): never {
        throw new DependencyWait(message);
    }

    /** Run calls as the stack's principal in its transaction, returning results in order. */
    mutate<const Calls extends readonly Call[]>(calls: Calls): Promise<ResultsOf<Calls>> {
        return this.#options.store.mutate(
            this.caller,
            { id: RequestId.create(this.now), calls },
            { transaction: this.transaction, scope: this.scope },
        );
    }

    /** The caller the stack's calls run as: its principal, at the stack's time. */
    get caller(): AccessContext {
        return AccessContext.of(Caller.of(this.as), this.now);
    }

    /** Open a package release's build in the scope, the installation's if given, via the step. */
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
                    await Stack.#collect(declaration, this.document, this.#ordered),
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

        // plan the declarations of the types the stack installs and wait until the store serves them
        if (this.#installing.length > 0) {
            const installing = this.#installing.join(", ");
            steps.push(...(await this.#planInstalling()));

            return { steps, deferred: `${installing} wait for the store to serve their types` };
        }

        // retire in reverse order
        for (const declaration of this.#ordered.toReversed()) {
            steps.push(...(await this.#retire(declaration, found(resolved, declaration))));
        }

        return { steps };
    }

    /** Plan creating the declarations of the types the stack installs. */
    async #planInstalling(): Promise<Step[]> {
        // read the declarable types the applied declarations install
        const installing = present(this.#options.installing, "the reader of the installed types");
        const descriptions = await installing(this.transaction.view());
        const types = descriptions
            .map((description) => DESCRIBED.parse(description.definition))
            .filter((type) => type.declarable !== undefined);

        // plan each declaration of a collection one of them collects, and the declarations nested in it
        const steps: Step[] = [];
        for (const key of this.#installing) {
            const type = types.find((each) => each.plural === key && each.nested === undefined);
            if (type === undefined) {
                throw new ObjectError("UNSUPPORTED_DECLARATION", `this server cannot apply ${key}`);
            }
            for (const [name, value] of Object.entries(DECLARED.parse(this.document[key]))) {
                steps.push(Stack.#creation(type.name, name));
                for (const [plural, children] of Object.entries(DECLARED.parse(value))) {
                    // plan the declarations of a type nested in it
                    const child = types.find(
                        (each) => each.plural === plural && each.nested !== undefined,
                    );
                    if (child !== undefined) {
                        const names = Object.keys(DECLARED.parse(children));
                        steps.push(
                            ...names.map((each) =>
                                Stack.#creation(child.name, Address.join(name, each)),
                            ),
                        );
                    }
                }
            }
        }

        return steps;
    }

    /** Select an object type's declaration by type or name. */
    #select(object: ObjectType | string): TypeDeclaration {
        const target = this.#ordered.find((entry) =>
            typeof object === "string" ? entry.object.name === object : entry.object.same(object),
        );
        if (!target) {
            const plural = typeof object === "string" ? object : object.plural;
            throw new ObjectError("UNSUPPORTED_DECLARATION", `this server cannot apply ${plural}`);
        }

        return target;
    }

    /** Read the manager's records of one object type in the scope, by declaration name. */
    async #records(declaration: TypeDeclaration): Promise<Map<string, Select<ManagedTable>>> {
        // read the records the manager declared, leaving out purged ones
        const table = declaration.object.table;
        const managed = await this.transaction.read(table).findMany({
            where: {
                scope: this.scope,
                managerInstallationId: this.manager.installationId,
                managerPackageId: this.manager.packageId,
            },
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
            const values = {
                ...(declaration.values?.(name, resolved, this) ??
                    Table.decode(declaration.object.table, JsonValue.object(resolved))),
                ...(await this.#supplied(declaration.object, name)),
            };
            Stack.#requireColumns(declaration.object, values);
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

    /** Read the columns the stack supplies a type declared from its create input. */
    async #supplied(object: ObjectType, name: string): Promise<DeclaredValues> {
        // supply nothing to a type declaring its own schema
        const options = object.definition.declarable;
        if (options === undefined || "schema" in options) {
            return {};
        }

        // take the parent declared under the name's first segments
        const parent = object.parent?.object;
        const parentId =
            parent === undefined || parent === "any"
                ? {}
                : { parentId: await this.require(parent, Stack.parent(name)) };

        // take the scope enclosing the stack's scope as the owner
        const { ownerColumn } = object;
        const owner = ownerColumn === undefined ? {} : { [ownerColumn]: await this.enclosing() };

        return { ...parentId, ...owner };
    }

    /** Create the record of a declaration missing one. */
    async #createDeclared(
        declaration: TypeDeclaration,
        name: string,
        resolved: unknown,
        values: DeclaredValues,
    ): Promise<Step> {
        // plan the creation
        const { transaction, manager } = this;
        const step = Stack.#creation(declaration.object.name, name);

        // upsert the record through its table's owner, or apply it under a fresh identifier where its rows live
        const { object } = declaration;
        if (declaration.upsert) {
            await declaration.upsert(this, { ...manager, name }, resolved);

            return step;
        }
        const id = object.generateId();
        await this.#applyRecord(object, name, values, id);
        if (!(await this.#options.store.isReplica(object, transaction.view(), this.scope))) {
            await declaration.written?.(
                this,
                await Stack.#read(transaction, object.table, id),
                resolved,
            );
        }

        return step;
    }

    /** Update an attached record with a changed declaration, restoring it from deletion. */
    async #updateDeclared(
        declaration: TypeDeclaration,
        name: string,
        resolved: unknown,
        row: Select<ManagedTable>,
        values: DeclaredValues,
    ): Promise<Step[]> {
        // skip a detached or unchanged record
        const { transaction } = this;
        const table = declaration.object.table;
        const changed = Stack.#difference(table, row, values);
        const isDeletable = "deletedAt" in table[TABLE].columns;
        const record: Row = row;
        const deletion = isDeletable
            ? Stack.#jsonOf(table, "deletedAt", record["deletedAt"])
            : null;
        if (
            !declarable.isManaging(row) ||
            (Object.keys(changed).length === 0 &&
                deletion === null &&
                (await declaration.changed?.(transaction, row, resolved)) !== true)
        ) {
            return [];
        }

        // plan the update of the changed fields
        const fields = {
            ...changed,
            ...(deletion === null ? {} : { deletedAt: { before: deletion, after: null } }),
        };
        const step: Step = {
            action: "update",
            target: Address.join(declaration.object.name, name),
            risk: "safe",
            detail: Object.keys(fields).length > 0 ? "change declared fields" : "refresh",
            fields,
        };
        // upsert the record through its table's owner, or apply the declared values as its next revision where its rows live
        if (declaration.upsert) {
            await declaration.upsert(this, { ...this.manager, name }, resolved);

            return [step];
        }
        await this.#applyRecord(declaration.object, name, values);
        if (
            !(await this.#options.store.isReplica(
                declaration.object,
                transaction.view(),
                this.scope,
            ))
        ) {
            await declaration.written?.(
                this,
                await Stack.#read(transaction, table, row.id),
                resolved,
            );
        }

        return [step];
    }

    /** Apply a declared record's values where it lives, with a fresh identifier if new. */
    async #applyRecord(
        object: ObjectType,
        name: string,
        values: DeclaredValues,
        id?: string,
    ): Promise<void> {
        const table = object.table;
        const json = Object.fromEntries(
            Object.entries(values).map(([column, value]) => [
                column,
                Stack.#jsonOf(table, column, value),
            ]),
        );
        await this.mutate([
            object.call("apply", {
                ...(id === undefined ? {} : { id }),
                manager: { ...this.manager, name },
                values: json,
            }),
        ]);
    }

    /** Allow the stack's principal in its scope in the stack's transaction, for access writes. */
    authorization(): Promise<Authorization> {
        return this.#options.store.authorizeHere(this.transaction, this.scope, this.caller);
    }

    /** Retire attached records the manager no longer declares. */
    async #retire(
        declaration: TypeDeclaration,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<Step[]> {
        // retain the records of a type whose records stay once undeclared
        const { retire } = declaration;
        if (retire === "retained") {
            return [];
        }
        const isArchived = retire === "archived";

        // read existing records
        const { transaction } = this;
        const table = declaration.object.table;
        const isDeletable = "deletedAt" in table[TABLE].columns;
        const steps: Step[] = [];
        for (const [name, row] of await this.#records(declaration)) {
            // skip declared, detached and already retiring records
            const record: Row = row;
            if (
                desired.has(name) ||
                !declarable.isManaging(row) ||
                (isDeletable && record["deletedAt"] !== null) ||
                (isArchived && record["active"] === false) ||
                (typeof retire === "object" && (await retire.isRetiring(transaction, row)))
            ) {
                continue;
            }
            const target = Address.join(declaration.object.name, name);
            const manager = { ...this.manager, name };

            // archive the record inactive, retained for what references it
            if (isArchived) {
                steps.push({ action: "update", target, risk: "safe", detail: "archive" });
                await this.mutate([
                    declaration.object.call("apply", {
                        manager,
                        values: { active: false },
                    }),
                ]);
            }
            // retire through the declaration
            else if (retire !== undefined) {
                steps.push({ action: "delete", target, risk: "destructive", detail: "retire" });
                await retire.start(this, row);
            }
            // retire where the record's rows live
            else {
                steps.push({ action: "delete", target, risk: "destructive", detail: "retire" });
                await this.mutate([declaration.object.call("apply", { manager, values: null })]);
            }
        }

        return steps;
    }

    /** Plan creating a declared record of a type under its declared name. */
    static #creation(type: string, name: string): Step {
        return {
            action: "create",
            target: Address.join(type, name),
            risk: "safe",
            detail: "declare",
        };
    }

    /** Select an object type's declarations from a document, a nested type's under its parent's. */
    static async #collect(
        declaration: TypeDeclaration,
        document: JsonObject,
        declared: readonly TypeDeclaration[],
    ): Promise<Readonly<Record<string, unknown>>> {
        // read a collection the type selects itself
        if (declaration.collect) {
            return declaration.collect(document);
        }

        // read a type declared from its create input with a parent under the parent's declarations
        const { object } = declaration;
        const above = object.parent?.object;
        const parent = above === "any" ? undefined : above;
        const options = object.definition.declarable;
        const isInline = parent !== undefined && options !== undefined && "from" in options;
        if (isInline && !declared.some((entry) => entry.object.same(parent))) {
            throw new TypeError(
                `${object.plural} are declared under ${parent.plural}, but this stack applies no declarations of them`,
            );
        }
        const entries = isInline
            ? Stack.nested(object, parent, document)
            : Object.entries(DECLARED.parse(document[object.plural] ?? {}));

        // parse each with the type's declaration schema, leaving out its children's collections
        const declarationSchema = object.lifecycle.declarationSchema;
        const children = declared
            .filter((entry) => {
                const nesting = entry.object.parent?.object;

                return nesting !== undefined && nesting !== "any" && nesting.same(object);
            })
            .map((entry) => entry.object.plural);

        return Object.fromEntries(
            entries.map(([name, value]) => [
                name,
                declarationSchema
                    ? declarationSchema.parse(Stack.#without(value, children))
                    : value,
            ]),
        );
    }

    /** Leave a declaration's children's collections out of it. */
    static #without(value: unknown, children: readonly string[]): unknown {
        const declared = DECLARED.parse(value);

        return Object.fromEntries(
            Object.entries(declared).filter(([key]) => !children.includes(key)),
        );
    }

    /** Order declarations so each follows the object types it references. */
    static #order(declarations: readonly TypeDeclaration[]): TypeDeclaration[] {
        // read explicit dependencies
        const explicit = Stack.#explicitDependencies(declarations);

        // expand "every" to declarations independent of this one
        const dependencies = new Map(
            declarations.map((entry) => [
                entry,
                entry.after === "every"
                    ? declarations.filter(
                          (other) =>
                              other !== entry &&
                              other.after !== "every" &&
                              !Stack.#dependsOn(explicit, other, entry, new Set()),
                      )
                    : found(explicit, entry),
            ]),
        );

        // visit dependencies depth first, rejecting cycles
        const ordered: TypeDeclaration[] = [];
        const visiting = new Set<TypeDeclaration>();
        for (const declaration of declarations) {
            Stack.#visit(declaration, dependencies, ordered, visiting);
        }

        return ordered;
    }

    /** Read the declarations each declaration lists to run after. */
    static #explicitDependencies(
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
    static #dependsOn(
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
            (dependency) => dependency === to || Stack.#dependsOn(explicit, dependency, to, seen),
        );
    }

    /** Add a declaration after its dependencies, rejecting cycles. */
    static #visit(
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
            Stack.#visit(dependency, dependencies, ordered, visiting);
        }
        visiting.delete(declaration);
        ordered.push(declaration);
    }

    /** Read one record a declaration just wrote, by identifier. */
    static async #read(
        transaction: Transaction,
        table: ManagedTable,
        id: Identifier<string>,
    ): Promise<Select<ManagedTable>> {
        const row = await transaction.read(table).findFirst({ where: { id } });
        if (row === undefined) {
            throw new TypeError(`the written ${table[TABLE].name} ${id} is missing`);
        }

        return row;
    }

    /** Collect the declared values that differ from a row, in their JSON form. */
    static #difference(
        table: Table,
        row: Row,
        values: Partial<Insert<Table>>,
    ): Record<string, { before: JsonValue; after: JsonValue }> {
        const changes: Record<string, { before: JsonValue; after: JsonValue }> = {};
        for (const [key, value] of Object.entries(values)) {
            const before = Stack.#jsonOf(table, key, row[key]);
            const after = Stack.#jsonOf(table, key, value);
            if (canonicalize(before) !== canonicalize(after)) {
                changes[key] = { before, after };
            }
        }

        return changes;
    }

    /** Write a column value in its JSON form, null for a missing one. */
    static #jsonOf(table: Table, column: string, value: ColumnValue | undefined): JsonValue {
        return value === undefined || value === null
            ? null
            : table[TABLE].column(column).definition.toJson(value);
    }

    /** Refuse values for unknown columns. */
    static #requireColumns(object: ObjectType, values: object): void {
        const columns = object.table[TABLE].columns;
        const unknown = Object.keys(values).find((key) => !Object.hasOwn(columns, key));
        if (unknown !== undefined) {
            throw new TypeError(`declarations of ${object.name} write unknown column ${unknown}`);
        }
    }
}

/** A stack's planned steps over the open transaction that applies them unchanged. */
export class StackPlan {
    /** The planned steps, and the wait deferring the rest. */
    readonly plan: Plan;
    /** What the machine applies the stack with. */
    readonly #options: ApplyOptions;
    /** The open transaction with the planned writes. */
    readonly #transaction: Transaction;

    /** Hold a plan with the transaction holding its writes. */
    constructor(options: ApplyOptions, transaction: Transaction, plan: Plan) {
        this.plan = plan;
        this.#options = options;
        this.#transaction = transaction;
    }

    /** Reserve the names the written objects claim, held by the commit, absent without claims. */
    async reserve(): Promise<Reservation | undefined> {
        // reserve before the commit, and write the reservation in it
        const { claims, objects, now } = this.#options;
        const claimants = await Claimant.written(this.#transaction, objects);
        const reservation =
            claims &&
            (await Reservation.reserve(claims, claimants, objects, {
                id: crypto.randomUUID(),
                now,
            }));
        this.#transaction.record(async (committing) => {
            await reservation?.commit(committing);
        });

        return reservation;
    }

    /** Apply the planned steps unchanged, refusing a plan another commit made stale. */
    async apply(): Promise<void> {
        // reserve the names the written objects claim before the commit
        const reservation = await this.reserve();

        // commit, releasing the names once the commit fails
        try {
            await this.#transaction.commit();
        } catch (error) {
            await reservation?.release();
            throw error;
        }

        // confirm the reserved names
        await reservation?.confirm();
    }
}

/** Stop applying until a dependency is ready. */
class DependencyWait extends Error {}

/** A stack's declarations in dependency order, and the keys left to the types it installs. */
interface Declared {
    /** The declared object types in dependency order. */
    readonly ordered: readonly TypeDeclaration[];
    /** The document keys no served type collects. */
    readonly installing: readonly string[];
}
