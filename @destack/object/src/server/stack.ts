import { Address, type Plan, type Step } from "@destack/resource";
import { ObjectReference, Scope, type Subject, type Call, RequestId } from "@destack/sync";
import { AccessContext, anyone, Authorizer, Manager, type Policy, Caller } from "@destack/access";
import {
    type ColumnValue,
    type DatabaseConnection,
    Transaction,
    type Insert,
    type Row,
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
    nth,
    present,
    schema,
    type JsonObject,
} from "@destack/schema";
import { ObjectError } from "../error/error.ts";
import { type ObjectOf, ObjectType } from "../object/object.ts";
import { Provisioned } from "../trait/provisioned.ts";
import {
    type StackScope,
    type StackSuppliedOf,
    StackObjectReference,
    StackReference,
    StackSubject,
} from "../stack/reference.ts";
import type { DeclarationName, PackageId } from "@destack/package";
import type { ManagedTable } from "../trait/record.ts";
import { InputSchema } from "../method/input.ts";
import { type ClaimHome, Reservation } from "../claim/reservation.ts";
import type { ObjectStore } from "./store.ts";
import type { Authorization } from "./authorization.ts";
import { RecordedRow, type ResultsOf } from "../method/context.ts";
import type { MethodName, OmitEach, ParsedInput } from "../method/procedure.ts";

import { CallSite } from "../method/site.ts";
import type { BuildReader } from "@destack/package/manifest";
import { Claimant } from "../claim/claim.ts";
import type {} from "@destack/package/import-meta";

/** One section of a stack document: declarations by name. */
const DECLARED = schema.record(schema.string(), schema.unknown());

/** A document's records a manager takes over: existing identifiers by `<plural>/<name>`. */
const ADOPTED = schema.record(schema.string(), schema.string().min(1));

/** A document's records a manager stops managing and leaves in place, by `<plural>/<name>`. */
const RELEASED = schema.array(schema.string());

/** The document keys of adoption and release, which no object type declares. */
const MANAGEMENT_KEYS: ReadonlySet<string> = new Set(["adopted", "released"]);

/** How a stack's declarations of one object type become its managed records. */
export interface DeclarationHandler<Object extends ObjectType = ObjectType> {
    /** The object types applied before and retired after this one, or "every". */
    readonly after?: readonly ObjectType[] | "every";
    /** Resolve references within a declaration, returning it unchanged by default. */
    resolve?(
        name: string,
        declared: DeclarationOf<Object>,
        plan: StackPlan,
    ): Promise<DeclarationOf<Object>>;
    /** The columns a declaration sets once, refusing another value later. */
    readonly immutable?: readonly (keyof Insert<Object["table"]> & string)[];
    /** What undeclaring a record does: delete it by default, archive it inactive, or retain it. */
    readonly retire?: "archived" | "retained";
}

/** The declaration a type accepts from managers: its resolved create input. */
export type DeclarationOf<Object extends ObjectType> =
    "create" extends MethodName<Object>
        ? OmitEach<ParsedInput<Object, "create">, StackSuppliedOf<Object>>
        : never;

/** What a machine applies a stack with. */
export interface StackOptions {
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
    /** The stack's document, keyed by collection. */
    readonly document: JsonObject;
    /** The document the stack applied last, empty before its first apply. */
    readonly applied: JsonObject;
    /** Further policies of declared objects. */
    readonly policies?: readonly Policy[];
    /** The claim type recording the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimHome;
    /** The object store running the declarations' calls, applying records where they live. */
    readonly store: Pick<ObjectStore, "mutate" | "query" | "authorizeHere" | "isReplica">;
    /** Open a package release's build in the scope via a snapshot, the installation's if given. */
    readonly release?: (
        snapshot: Snapshot,
        packageId: PackageId,
        installation?: Identifier<"installation">,
    ) => Promise<BuildReader>;
    /** Read the object types the applied declarations install, served once the store reloads. */
    readonly installing?: (snapshot: Snapshot) => Promise<readonly ObjectType[]>;
}

/** An object type beside how its declarations become records. */
type TypeDeclaration = DeclarationHandler & { readonly object: ObjectOf<{ table: ManagedTable }> };

/** A manager's declared records in a scope: its document, and the document it applied last. */
export class Stack {
    /** The scope containing the declared records, such as a space identifier. */
    readonly scope: string;
    /** The installation and package declaring the records. */
    readonly manager: Omit<Manager, "name">;
    /** The principal running the declarations' calls, such as the space applying a stack. */
    readonly as: Subject;
    /** The stack's document, keyed by collection. */
    readonly document: JsonObject;
    /** The document the stack applied last, empty before its first apply. */
    readonly applied: JsonObject;
    /** The policies validating declared access. */
    readonly authorizer: Authorizer;
    /** What the machine applies the stack with. */
    readonly options: StackOptions;
    /** The declared object types in dependency order. */
    readonly ordered: readonly TypeDeclaration[];
    /** The document's collections no served type declares, left to the types the stack installs. */
    readonly installing: readonly string[];

    /** Create a stack, ordering its declarations and refusing undeclarable collections. */
    constructor(options: StackOptions) {
        // store the stack
        this.scope = options.scope;
        this.manager = options.manager;
        this.as = options.as;
        this.document = options.document;
        this.applied = options.applied;
        this.options = options;

        // order the declarations under the declared object types' policies
        const declared = Stack.#declare(options);
        this.ordered = declared.ordered;
        this.installing = declared.installing;
        this.authorizer = new Authorizer([
            ...options.objects.map((object) => object.policy),
            ...(options.policies ?? []),
        ]);
    }

    /** Plan the stack's steps at a time, in a transaction the plan's apply commits. */
    async plan(now: number): Promise<StackPlan> {
        // write the steps into an open transaction
        const transaction = await Transaction.open(this.options.database);

        return await StackPlan.open(this, now, transaction);
    }

    /** Plan and apply the stack at a time in one transaction, again while it loses a race. */
    apply(now: number): Promise<Plan> {
        return Transaction.retry(async () => {
            const planned = await this.plan(now);
            await planned.apply();

            return planned.plan;
        });
    }

    /** Order a stack's declarations, refusing collections no type declares or installs. */
    static #declare(options: StackOptions): Declared {
        // pair each object type with its declaration
        const declared = options.objects.map((object) => {
            // require a managed type
            const declaration = object.declaration;
            if (declaration === undefined || !object.isDeclarable()) {
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

        // order the types the document declares, then those it only retires
        const isDeclared = (entry: TypeDeclaration) =>
            Object.hasOwn(options.document, entry.object.plural);
        const ordered = [
            ...Stack.#order(declared.filter(isDeclared), "refuse"),
            ...Stack.#order(
                declared.filter((entry) => !isDeclared(entry)),
                "skip",
            ),
        ];

        // leave the collections no served type declares to the types the stack installs
        const installing = Object.keys(options.document).filter(
            (key) =>
                !MANAGEMENT_KEYS.has(key) && !ordered.some((entry) => entry.object.plural === key),
        );
        if (installing.length > 0 && options.installing === undefined) {
            throw new ObjectError("UNSUPPORTED_DECLARATION", {
                message: `this server cannot apply ${nth(installing, 0)}`,
            });
        }

        return { ordered, installing };
    }

    /** Order declarations so each follows the object types it references. */
    static #order(
        declarations: readonly TypeDeclaration[],
        cycles: "refuse" | "skip",
    ): TypeDeclaration[] {
        // read explicit dependencies
        const explicit = Stack.#explicitDependencies(declarations);

        // expand "every" to the declarations of lower tiers independent of this one
        const dependencies = new Map(
            declarations.map((entry) => [
                entry,
                Stack.#tier(entry) > 0
                    ? declarations.filter(
                          (other) =>
                              other !== entry &&
                              Stack.#tier(other) < Stack.#tier(entry) &&
                              !Stack.#dependsOn(explicit, other, entry, new Set()),
                      )
                    : found(explicit, entry),
            ]),
        );

        // visit dependencies depth first, rejecting cycles
        const ordered: TypeDeclaration[] = [];
        const visiting = new Set<TypeDeclaration>();
        for (const declaration of declarations) {
            Stack.#visit(declaration, dependencies, ordered, visiting, cycles);
        }

        return ordered;
    }

    /** Read the declarations each declaration lists to run after. */
    static #explicitDependencies(
        declarations: readonly TypeDeclaration[],
    ): Map<TypeDeclaration, readonly TypeDeclaration[]> {
        return new Map(
            declarations.map((entry) => {
                const after = Stack.#after(entry);
                const dependencies = after === "every" ? [] : after;

                return [
                    entry,
                    dependencies.flatMap((object) =>
                        declarations.filter((other) => other.object.same(object)),
                    ),
                ] as const;
            }),
        );
    }

    /** Read what a declaration runs after: its handler's, parent's and referenced types, or all. */
    static #after(declaration: TypeDeclaration): readonly ObjectType[] | "every" {
        // follow every type for an object or subject reference
        const { object, after = [] } = declaration;
        const references = Object.values(object.declaredReferences);
        if (after === "every" || references.some((reference) => typeof reference === "string")) {
            return "every";
        }

        // follow the handler's types and each referenced type
        const referenced = references.filter(
            (reference): reference is ObjectType => typeof reference !== "string",
        );

        return [...after, ...referenced];
    }

    /** Rank a declaration: after its types, after all types, or after those too for any record. */
    static #tier(declaration: TypeDeclaration): 0 | 1 | 2 {
        const references = Object.values(declaration.object.declaredReferences);
        if (references.some((reference) => typeof reference === "string")) {
            return 2;
        } else if (declaration.after === "every") {
            return 1;
        } else {
            return 0;
        }
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
        cycles: "refuse" | "skip",
    ): void {
        // skip a declaration added before
        if (ordered.includes(declaration)) {
            return;
        }

        // refuse a declaration depending on itself, or skip the edge closing the cycle
        if (visiting.has(declaration) && cycles === "skip") {
            return;
        } else if (visiting.has(declaration)) {
            throw new ObjectError("CYCLIC_DECLARATION", {
                message: `declarations of ${declaration.object.plural} depend on themselves`,
            });
        }

        // add the dependencies first
        visiting.add(declaration);
        for (const dependency of found(dependencies, declaration)) {
            Stack.#visit(dependency, dependencies, ordered, visiting, cycles);
        }
        visiting.delete(declaration);
        ordered.push(declaration);
    }
}

/** A stack's planned steps in the open transaction that applies them. */
export class StackPlan {
    /** The stack the plan writes. */
    readonly stack: Stack;
    /** The time the stack is applied, in UTC milliseconds. */
    readonly now: number;
    /** The open transaction with the planned writes. */
    readonly transaction: Transaction;
    /** The planned steps, and the wait deferring the rest, written once on open. */
    #plan: Plan = { steps: [] };
    /** The scope enclosing the stack's scope, read on first use. */
    #enclosing: Promise<string> | undefined;

    /** Store a plan's stack, time and transaction. */
    private constructor(stack: Stack, now: number, transaction: Transaction) {
        this.stack = stack;
        this.now = now;
        this.transaction = transaction;
    }

    /** Write a stack's declared steps into a transaction, planning the ones it waits on. */
    static async open(stack: Stack, now: number, transaction: Transaction): Promise<StackPlan> {
        // refuse a moved scope
        for (const link of await Scope.chain(transaction.view(), stack.scope)) {
            if (link.movedTo !== undefined) {
                throw new ObjectError("SCOPE_MOVED", {
                    message: `${link.object.id} moves to ${link.movedTo}`,
                    data: { scope: link.object.id, machine: link.movedTo },
                });
            }
        }

        // write the steps
        const planned = new StackPlan(stack, now, transaction);
        planned.#plan = await planned.#apply();

        return planned;
    }

    /** The planned steps, and the wait deferring the rest. */
    get plan(): Plan {
        return this.#plan;
    }

    /** Read the scope enclosing the stack's scope, such as a space's account, once per apply. */
    enclosing(): Promise<string> {
        // read the scope's row once
        this.#enclosing ??= Scope.read(this.transaction.view(), [this.stack.scope]).then(
            ([row]) => present(row, `the scope row of ${this.stack.scope}`).parent,
        );

        return this.#enclosing;
    }

    /** Resolve an object type the machine applies declarations of, by name. */
    object(name: string): ObjectType {
        return this.#select(name).object;
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
                    scope: this.stack.scope,
                    managerInstallationId: this.stack.manager.installationId,
                    managerPackageId: this.stack.manager.packageId,
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

        return id ?? (await this.#missing(target, name, this.stack.scope));
    }

    /** Resolve a declaration into its type's create input: its name and its references. */
    async resolveInput(
        object: ObjectType,
        name: string,
        declared: unknown,
    ): Promise<Record<string, unknown>> {
        // take the written fields but the references, and fill a name field with the declaration's name
        const written = DECLARED.parse(declared);
        const references = Object.entries(object.declaredReferences);
        const referenced = new Set(references.map(([field]) => ObjectType.declaredField(field)));
        const input: Record<string, unknown> = Object.fromEntries(
            Object.entries(written).filter(([field]) => !referenced.has(field)),
        );
        if (
            "name" in InputSchema.fields(object.input("create")).shape &&
            input["name"] === undefined
        ) {
            input["name"] = name;
        }

        // resolve each reference into its create field, an absent object reference to the stack's scope
        for (const [field, reference] of references) {
            const value = written[ObjectType.declaredField(field)];
            // resolve an object, the stack's scope when absent
            if (reference === "object") {
                input[field] = await this.resolveObject(
                    value === undefined ? undefined : StackObjectReference.parse(value),
                );
            }
            // resolve a present subject
            else if (reference === "subject" && value !== undefined) {
                input[field] = await this.resolveSubject(StackSubject.parse(value));
            }
            // resolve a present reference to one type's record
            else if (reference !== "subject" && value !== undefined) {
                const declaredReference = StackReference.of(reference.identity).parse(value);
                input[field] = await this.resolveRecord(reference, declaredReference);
            }
        }

        // wait for a provisioned parent's provider before writing into it
        const parent = object.parent?.object;
        const parentId = input["parentId"];
        if (
            parent !== undefined &&
            parent !== "any" &&
            Provisioned.is(parent) &&
            typeof parentId === "string"
        ) {
            const [row] = await this.transaction
                .read(parent.table)
                .findMany({ where: { id: parent.identifier(parentId) } });
            if (row === undefined || !Provisioned.isProvisioned(row)) {
                this.wait(`${parent.name} ${parentId} is not provisioned`);
            }
        }

        return input;
    }

    /** Resolve a declared reference to one type's record, by declared name or identifier. */
    async resolveRecord(
        type: ObjectType,
        reference: DeclarationName | { readonly id: string },
    ): Promise<string> {
        return typeof reference === "string"
            ? await this.require(type, reference)
            : await this.#requireIn(type, reference.id, this.stack.scope);
    }

    /** Resolve a declared object: declared, existing, or the stack's scope when absent. */
    async resolveObject(reference: StackObjectReference | undefined): Promise<ObjectReference> {
        // take the stack's scope
        if (reference === undefined) {
            return await Scope.object(this.transaction.view(), this.stack.scope);
        }
        // take an existing record or principal by its identifier
        else if ("id" in reference) {
            return await this.#identify(reference.id, undefined);
        }
        // take a record the stack declares
        else {
            const type = this.object(reference.kind);

            return type.reference(this.stack.scope, await this.require(type, reference.name));
        }
    }

    /** Resolve a declared subject to the principal, subject set or wildcard it refers to. */
    async resolveSubject(declared: StackSubject): Promise<Subject> {
        // relate every caller through the wildcard
        if ("anyone" in declared) {
            return anyone.reference("*", "*");
        }

        // relate a record or principal by identifier, through a relation when given
        const relation = declared.relation === undefined ? {} : { relation: declared.relation };
        if ("id" in declared) {
            return { ...(await this.#identify(declared.id, declared.scope)), ...relation };
        }
        // relate a record the stack declares
        else if ("kind" in declared) {
            return { ...(await this.resolveObject(declared)), ...relation };
        }
        // relate the stack's scope or the one enclosing it
        else {
            const scope =
                declared.scope === "enclosing" ? await this.enclosing() : this.stack.scope;

            return { ...(await Scope.object(this.transaction.view(), scope)), ...relation };
        }
    }

    /** Resolve an identifier to a global principal, or to a record in a scope of the stack. */
    async #identify(
        id: Identifier<string>,
        scope: StackScope | undefined,
    ): Promise<ObjectReference> {
        // read the identifier's policy, and the applied type the stack reads its records of
        const policy = this.stack.authorizer.policyOf(id);
        const isGlobal = policy.definition.isGlobal === true;
        const applied = this.stack.ordered.find((entry) => entry.object.identifies(id))?.object;
        const within = scope === "enclosing" ? await this.enclosing() : this.stack.scope;

        // refuse a scope for a global principal
        if (isGlobal && scope !== undefined) {
            throw new ObjectError("INVALID_DECLARATION", {
                message: `${policy.name} ${id} is global, outside any scope`,
            });
        }
        // take a global principal in the universe
        else if (isGlobal) {
            return policy.reference(Scope.universe.id, id);
        }
        // require a record of an applied type in its scope
        else if (applied !== undefined) {
            return policy.reference(within, await this.#requireIn(applied, id, within));
        }
        // refer to a record of another known type, which access checks as relationships use it
        else {
            return policy.reference(within, id);
        }
    }

    /** Require a record of a type by identifier in a scope. */
    async #requireIn(type: ObjectType, id: string, scope: string): Promise<string> {
        const [row] = await this.transaction
            .view()
            .read(type.table)
            .findMany({ where: { id, scope } });

        return row === undefined ? await this.#missing(type, id, scope) : id;
    }

    /** Wait for a missing record's replica to arrive in a scope, or refuse the declaration. */
    async #missing(type: ObjectType, key: string, scope: string): Promise<never> {
        // wait for a replica's record, and refuse a record its home lacks
        if (await this.stack.options.store.isReplica(type, this.transaction.view(), scope)) {
            return this.wait(`${type.name} ${key} waits for its replica`);
        } else {
            throw new ObjectError("INVALID_DECLARATION", {
                message: `unknown ${type.name}: ${key}`,
            });
        }
    }

    /** Stop applying until a dependency is ready, reporting why. */
    wait(message: string): never {
        throw new DependencyWait(message);
    }

    /** Run calls as a declaration's manager in the stack's transaction, results in order. */
    mutate<const Calls extends readonly Call[]>(
        calls: Calls,
        name: string,
    ): Promise<ResultsOf<Calls>> {
        return this.stack.options.store.mutate(
            this.caller,
            { id: RequestId.create(this.now), calls },
            {
                transaction: this.transaction,
                scope: this.stack.scope,
                manager: { ...this.stack.manager, name },
            },
        );
    }

    /** Run a query as the stack's principal in its transaction. */
    query(call: Call): Promise<unknown> {
        return this.stack.options.store.query(this.caller, call, CallSite.store, {
            transaction: this.transaction,
        });
    }

    /** The caller the stack's calls run as: its principal, at the stack's time. */
    get caller(): AccessContext {
        return AccessContext.of(Caller.of(this.stack.as), this.now);
    }

    /** Open a package release's build in the scope, the installation's if given, via the step. */
    release(packageId: PackageId, installation?: Identifier<"installation">): Promise<BuildReader> {
        // require the machine opening releases
        const { release } = this.stack.options;
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
        for (const declaration of this.stack.ordered) {
            try {
                const desired = new Map<string, unknown>();
                for (const [name, declared] of Object.entries(
                    StackPlan.#declarations(declaration, this.stack.document),
                )) {
                    const created = await this.resolveInput(declaration.object, name, declared);
                    desired.set(
                        name,
                        declaration.resolve
                            ? await declaration.resolve(name, created, this)
                            : created,
                    );
                }
                resolved.set(declaration, desired);
                await this.#manage(declaration);
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
        if (this.stack.installing.length > 0) {
            const installing = this.stack.installing.join(", ");
            steps.push(...(await this.#planInstalling()));

            return { steps, deferred: `${installing} wait for the store to serve their types` };
        }

        // retire in reverse order
        for (const declaration of this.stack.ordered.toReversed()) {
            steps.push(...(await this.#retire(declaration, found(resolved, declaration))));
        }

        return { steps };
    }

    /** Plan creating the declarations of the types the stack installs. */
    async #planInstalling(): Promise<Step[]> {
        // read the types the applied declarations install, and the ones stacks declare by collection
        const installing = present(
            this.stack.options.installing,
            "the reader of the installed types",
        );
        const types = (await installing(this.transaction.view())).filter(
            (type) => type.declaration !== undefined,
        );

        // plan each declaration of a collection one of them declares
        const steps: Step[] = [];
        for (const key of this.stack.installing) {
            const type = types.find((each) => each.plural === key);
            if (type === undefined) {
                throw new ObjectError("UNSUPPORTED_DECLARATION", {
                    message: `this server cannot apply ${key}`,
                });
            }
            for (const name of Object.keys(DECLARED.parse(this.stack.document[key]))) {
                steps.push(StackPlan.#creation(type.name, name));
            }
        }

        return steps;
    }

    /** Select an object type's declaration by type or name. */
    #select(object: ObjectType | string): TypeDeclaration {
        const target = this.stack.ordered.find((entry) =>
            typeof object === "string" ? entry.object.name === object : entry.object.same(object),
        );
        if (!target && this.stack.installing.length > 0) {
            const plural = typeof object === "string" ? object : object.plural;
            throw new DependencyWait(`${plural} wait for the store to serve the installed types`);
        } else if (!target) {
            const plural = typeof object === "string" ? object : object.plural;
            throw new ObjectError("UNSUPPORTED_DECLARATION", {
                message: `this server cannot apply ${plural}`,
            });
        }

        return target;
    }

    /** Read the manager's records of one object type in the scope, by declaration name. */
    async #records(declaration: TypeDeclaration): Promise<Map<string, Select<ManagedTable>>> {
        // read the records the manager declared, leaving out purged ones
        const table = declaration.object.table;
        const rows = await this.transaction.read(table).findMany({
            where: {
                scope: this.stack.scope,
                managerInstallationId: this.stack.manager.installationId,
                managerPackageId: this.stack.manager.packageId,
                detachedAt: { isNull: true },
                ...("purgedAt" in table[TABLE].columns ? { purgedAt: { isNull: true } } : {}),
            },
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

    /** Take over the records the document adopts of a type, and let go of the ones it releases. */
    async #manage(declaration: TypeDeclaration): Promise<void> {
        // read the type's adopted and released entries
        const { object } = declaration;
        const prefix = `${object.plural}/`;
        const adopted = Object.entries(ADOPTED.parse(this.stack.document["adopted"] ?? {})).filter(
            ([address]) => address.startsWith(prefix),
        );
        const released = RELEASED.parse(this.stack.document["released"] ?? []).filter((address) =>
            address.startsWith(prefix),
        );
        const existing = await this.#records(declaration);
        const declared = new Set(
            Object.keys(DECLARED.parse(this.stack.document[object.plural] ?? {})),
        );

        // adopt an unmanaged record under its declaration's name, once
        for (const [address, id] of adopted) {
            const name = address.slice(prefix.length);
            const manager = { ...this.stack.manager, name };
            const [row] = await this.transaction.read(object.table).findMany({
                where: { id: schema.anyIdentifier().parse(id), scope: this.stack.scope },
            });

            // refuse an unknown record, an undeclared name, a name of another record, or a record another declaration manages
            if (row === undefined) {
                throw new ObjectError("INVALID_DECLARATION", {
                    message: `${object.name} ${name} adopts unknown ${id}`,
                });
            } else if (!declared.has(name)) {
                throw new ObjectError("INVALID_DECLARATION", {
                    message: `${object.name} ${name} adopts ${id} but declares no ${name}`,
                });
            } else if (existing.has(name) && existing.get(name)?.id !== id) {
                throw new ObjectError("INVALID_DECLARATION", {
                    message: `${object.name} ${name} already declares another record`,
                });
            } else if (Manager.isManaged(row) && !Manager.manages(row, manager)) {
                throw new ObjectError("INVALID_DECLARATION", {
                    message: `${object.name} ${id} is managed by another declaration`,
                });
            }
            // adopt it unless already adopted
            else if (!Manager.isManaged(row)) {
                await this.#writeManager(object, row, {
                    ...Manager.values(manager),
                    detachedAt: null,
                });
            }
        }

        // release a managed record and leave it in place, refusing a declared one, as released ones stay listed
        for (const address of released) {
            const name = address.slice(prefix.length);
            const row = existing.get(name);
            if (declared.has(name)) {
                throw new ObjectError("INVALID_DECLARATION", {
                    message: `${object.name} ${name} is both declared and released`,
                });
            } else if (row !== undefined) {
                await this.#writeManager(object, row, { detachedAt: this.now });
            }
        }
    }

    /** Read a record again after a call changed it. */
    async #reread(
        declaration: TypeDeclaration,
        row: Select<ManagedTable>,
    ): Promise<Select<ManagedTable>> {
        const [reread] = await this.transaction
            .read(declaration.object.table)
            .findMany({ where: { id: row.id, scope: this.stack.scope } });

        return present(reread, `${declaration.object.name} ${row.id}`);
    }

    /** Write a record's manager columns at its next revision, as the stack's subject. */
    async #writeManager(object: ObjectType, row: Row, management: Row): Promise<void> {
        // stamp the next revision, and the time and author where the table records them
        const columns = object.table[TABLE].columns;
        const stamped = {
            ...management,
            revision: RecordedRow.parse(row).revision + 1,
            ...(Object.hasOwn(columns, "updatedAt") ? { updatedAt: this.now } : {}),
            ...(Object.hasOwn(columns, "updatedById")
                ? ObjectReference.columns("updatedBy", this.stack.as)
                : {}),
        };

        // write it, updating the type's readers
        const change = { table: object.table, before: row, after: { ...row, ...stamped } };
        await this.transaction.write(change);
        object.updateReaders(this.transaction, change, this.now);
    }

    /** Create missing records and update changed attached ones, through the type's verbs. */
    async #write(
        declaration: TypeDeclaration,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<Step[]> {
        // create a missing record, or update a changed attached record against what the stack applied last
        const existing = await this.#records(declaration);
        const applied = StackPlan.#declarations(declaration, this.stack.options.applied);
        const steps: Step[] = [];
        for (const [name, resolved] of desired) {
            // declare a record of an archiving type active, as retiring archives it
            const input = JsonValue.object({
                ...(declaration.retire === "archived" ? { active: true } : {}),
                ...DECLARED.parse(resolved),
                ...(await this.#supplied(declaration.object)),
            });
            const row = existing.get(name);
            if (row === undefined) {
                steps.push(await this.#create(declaration, name, input));
            } else {
                const written = Object.keys(DECLARED.parse(applied[name] ?? {}));
                steps.push(...(await this.#update(declaration, name, row, input, written)));
            }
        }

        return steps;
    }

    /** Read the owner a stack supplies a declaration: the scope enclosing the stack's. */
    async #supplied(object: ObjectType): Promise<JsonObject> {
        const { ownerColumn } = object;

        return ownerColumn === undefined ? {} : { [ownerColumn]: await this.enclosing() };
    }

    /** Create the record of a declaration missing one, as its manager. */
    async #create(declaration: TypeDeclaration, name: string, input: JsonObject): Promise<Step> {
        const { object } = declaration;
        await this.mutate(
            [object.call("create", { ...object.in(this.stack.scope), ...input })],
            name,
        );

        return StackPlan.#creation(object.name, name);
    }

    /** Update an attached record whose declared fields changed, as a three-way merge. */
    async #update(
        declaration: TypeDeclaration,
        name: string,
        loaded: Select<ManagedTable>,
        input: JsonObject,
        written: readonly string[],
    ): Promise<Step[]> {
        // restore a deleted record, reading it again as restored
        const { object } = declaration;
        const values: Row = loaded;
        const isDeleted = "deletedAt" in values && values["deletedAt"] !== null;
        if (isDeleted) {
            await this.mutate([object.call("restore", object.target(loaded))], name);
        }
        const row = isDeleted ? await this.#reread(declaration, loaded) : loaded;

        // compare the declared fields with the record as its type reads it
        const current = DECLARED.parse(await this.query(object.call("get", object.target(row))));
        const created = Object.keys(InputSchema.fields(object.input("create")).shape);
        const byDeclared = new Map(
            created.map((field) => [ObjectType.declaredField(field), field]),
        );
        const compared = new Set([
            ...Object.keys(input),
            ...written.flatMap((field) => {
                const createdField = byDeclared.get(field);

                return createdField === undefined || object.supplied.includes(createdField)
                    ? []
                    : [createdField];
            }),
        ]);
        const defaults = object.table[TABLE].defaults({});
        const fields: Record<string, { before: JsonValue; after: JsonValue }> = {};
        for (const field of compared) {
            // compare each field written now or last, a field the record leaves out as null and one the document leaves out at its default
            const before = JsonValue.of(current[field] ?? null);
            const after = JsonValue.of(input[field] ?? defaults[field] ?? null);
            if (canonicalize(before) !== canonicalize(after)) {
                fields[field] = { before, after };
            }
        }
        if (Object.keys(fields).length === 0 && !isDeleted) {
            return [];
        }

        // refuse changing a field the declaration sets once
        const fixed = Object.keys(fields).find((field) =>
            (declaration.immutable ?? []).some((column) => column === field),
        );
        if (fixed !== undefined) {
            throw new ObjectError("INVALID_DECLARATION", {
                message: `${object.name} ${name} cannot change ${fixed}: declared ${canonicalize(fields[fixed]?.after ?? null)}, applied ${canonicalize(fields[fixed]?.before ?? null)}`,
            });
        }

        // replace a record whose type does not update a changed field: detach and retire it, then declare anew
        const updatable =
            "update" in object.methods ? InputSchema.fields(object.input("update")).shape : {};
        const target = Address.join(object.name, name);
        if (Object.keys(fields).some((field) => !(field in updatable))) {
            const detached = { ...row, detachedAt: this.now };
            await this.#writeManager(object, row, { detachedAt: this.now });
            await this.#retireRecord(declaration, name, detached);
            await this.#create(declaration, name, input);

            // replace destructively unless the type archives or retains what it retires
            const risk = declaration.retire === undefined ? "destructive" : "safe";

            return [{ action: "replace", target, risk, detail: "replace", fields }];
        }

        // update the changed fields as the record's manager
        const changed = Object.fromEntries(
            Object.entries(fields).map(([field, value]) => [field, value.after]),
        );
        if (Object.keys(changed).length > 0) {
            await this.mutate([object.call("update", { ...object.target(row), ...changed })], name);
        }

        // list the restore and the update of the changed fields
        const restored: Step = {
            action: "restore",
            target,
            risk: "safe",
            detail: "restore",
            fields: {
                deletedAt: {
                    before: StackPlan.#jsonOf(object.table, "deletedAt", values["deletedAt"]),
                    after: null,
                },
            },
        };
        const updated: Step = {
            action: "update",
            target,
            risk: "safe",
            detail: "change declared fields",
            fields,
        };

        return [
            ...(isDeleted ? [restored] : []),
            ...(Object.keys(changed).length > 0 ? [updated] : []),
        ];
    }

    /** Allow the stack's principal in its scope in the stack's transaction, for access writes. */
    authorization(): Promise<Authorization> {
        return this.stack.options.store.authorizeHere(
            this.transaction,
            this.stack.scope,
            this.caller,
        );
    }

    /** Retire the attached records the manager no longer declares, as the type retires them. */
    async #retire(
        declaration: TypeDeclaration,
        desired: ReadonlyMap<string, unknown>,
    ): Promise<Step[]> {
        // retire each undeclared record
        const steps: Step[] = [];
        for (const [name, row] of await this.#records(declaration)) {
            if (!desired.has(name)) {
                steps.push(...(await this.#retireRecord(declaration, name, row)));
            }
        }

        return steps;
    }

    /** Retire one record as its type retires records: delete it, archive it, or retain it. */
    async #retireRecord(
        declaration: TypeDeclaration,
        name: string,
        row: Select<ManagedTable>,
    ): Promise<Step[]> {
        // skip a retained record, and one already retiring
        const { retire, object } = declaration;
        const values: Row = row;
        const isDeletable = "deletedAt" in object.table[TABLE].columns;
        if (
            retire === "retained" ||
            (isDeletable && values["deletedAt"] !== null) ||
            (retire === "archived" && values["active"] === false)
        ) {
            return [];
        }
        const target = Address.join(object.name, name);

        // archive the record inactive, retained for what refers to it
        if (retire === "archived") {
            await this.mutate(
                [object.call("update", { ...object.target(row), active: false })],
                name,
            );

            return [{ action: "update", target, risk: "safe", detail: "archive" }];
        }
        // delete it through its type
        else {
            await this.mutate([object.call("delete", object.target(row))], name);

            return [{ action: "delete", target, risk: "destructive", detail: "retire" }];
        }
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

    /** Read an object type's declarations from a document, parsed by its declaration schema. */
    static #declarations(
        declaration: TypeDeclaration,
        document: JsonObject,
    ): Readonly<Record<string, unknown>> {
        // parse each entry of the type's collection with its declaration schema
        const { object } = declaration;
        const declarationSchema = object.lifecycle.declarationSchema;
        const entries = Object.entries(DECLARED.parse(document[object.plural] ?? {}));

        return Object.fromEntries(
            entries.map(([name, value]) => [
                name,
                declarationSchema ? declarationSchema.parse(value) : value,
            ]),
        );
    }

    /** Write a column value in its JSON form, null for a missing one. */
    static #jsonOf(table: Table, column: string, value: ColumnValue | undefined): JsonValue {
        return value === undefined || value === null
            ? null
            : table[TABLE].column(column).definition.toJson(value);
    }

    /** Reserve the names the written objects claim until the commit, absent without claims. */
    async #reserve(): Promise<Reservation | undefined> {
        // reserve before the commit, and write the reservation in it
        const { claims, objects } = this.stack.options;
        const { now } = this;
        const claimants = await Claimant.written(this.transaction, objects);
        const reservation =
            claims &&
            (await Reservation.reserve(claims, claimants, objects, {
                id: crypto.randomUUID(),
                now,
            }));
        this.transaction.record(async (committing) => {
            await reservation?.commit(committing);
        });

        return reservation;
    }

    /** Apply the planned steps unchanged, refusing a plan another commit made stale. */
    async apply(): Promise<void> {
        // reserve the names the written objects claim before the commit
        const reservation = await this.#reserve();

        // commit, releasing the names once the commit fails
        try {
            await this.transaction.commit();
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

/** A stack's declarations in dependency order, and the entries left to the types it installs. */
interface Declared {
    /** The declared object types in dependency order. */
    readonly ordered: readonly TypeDeclaration[];
    /** The document's collections no served type declares. */
    readonly installing: readonly string[];
}
