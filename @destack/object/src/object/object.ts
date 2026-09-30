import {
    Policy,
    none,
    subjectType,
    type AccessExpression,
    type Elevation,
    type RelationInput,
    type Subject,
    type PolicySubject,
    principal,
    relationsOf,
    type Permission,
    type TableMapping,
} from "@destack/access";
import { Scope, type ObjectReference, type Query } from "@destack/sync";
import { Snapshot } from "@destack/db/log";
import { canonicalize } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import type { Claim, Directory, ObjectClaims } from "@destack/directory";
import { Condition } from "@destack/db/query";
import { Expression } from "@destack/schema/expression";
import { type AuditAction, AuditTarget } from "@destack/audit";
import {
    eq,
    type DatabaseConnection,
    type DatabaseTier,
    type Select,
    type SQL,
    TABLE,
    type Table,
    type TableConstraint,
} from "@destack/db";
import type { Tree } from "@destack/db/tree";
import {
    declaringModule,
    type ModuleMetadata,
    type Package,
    type PackageId,
} from "@destack/package";
import { schema, Version } from "@destack/schema";
import type { AggregateFunction, Field, TextField } from "../field/field.ts";
import { Call, type Handler, type Phases } from "../method/call.ts";
import type { Calls } from "../method/procedure.ts";
import type { Method, MethodKind } from "../method/method.ts";
import type { ServiceRouter } from "@destack/service";
import {
    objectProcedures,
    objectSchema,
    scopeRoute,
    type ObjectProcedures,
    type ScopeRoute,
    type ObjectSchema,
} from "../method/procedure.ts";
import {
    replicaProcedures,
    type ObjectInclude,
    type ObjectQuery,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { compile, compileQueries, join, type Join } from "../query/query.ts";
import { record } from "../trait/record.ts";
import {
    declarable,
    detachable,
    type DeclarableDefinition,
    type DetachableMethodMap,
} from "../trait/declarable.ts";
import { Duration } from "./duration.ts";
import { controlled, type ControlledMethodMap } from "../trait/controlled.ts";
import { nested, type NestedMethodMap, type NestedDefinition } from "../trait/nested.ts";
import { transitions, type TransitionMethodMap } from "../trait/transition.ts";
import {
    recoverable,
    type RecoverableDefinition,
    type RecoverableMethodMap,
} from "../trait/recoverable.ts";
import { expiring, type ExpiryRule } from "../trait/expiring.ts";
import { addressed, copy, type AddressedDefinition } from "../trait/addressed.ts";
import { versioned, type VersionsDefinition } from "../trait/versioned.ts";
import { shareable, type ShareableMethodMap } from "../trait/shareable.ts";
import { suspendable, type SuspendableMethodMap } from "../trait/suspendable.ts";
import { attachments } from "../trait/attachment.ts";
import { tracked, type TrackedDefinition, type TrackedMethodMap } from "../trait/tracked.ts";
import { text, type TextMethodMap } from "../trait/text.ts";
import { Chunk } from "../text/chunk.ts";
import { chunk, chunkRun } from "../text/table.ts";
import type { GateOf, Gated, Trait, TraitObject } from "../trait/trait.ts";
import { INTRINSIC, type Intrinsic } from "./intrinsic.ts";
import { serverTables } from "../stack/db.ts";
import type { ObjectController } from "./controller.ts";
import type { ObjectDeclaration, DeclarationOf } from "../server/stack.ts";
import { camelCase, kebabCase, pascalCase } from "./name.ts";
import { deriveTable, type ConstraintColumns, type ObjectTable, type TraitOf } from "./table.ts";

/** The default ephemeral linger, in milliseconds: 10 s spans two 1–3 s reconnects. */
const LINGER_MILLISECONDS = 10_000;

/** The method kinds writing records. */
const RECORD_KINDS: ReadonlySet<string> = new Set(["create", "update", "delete", "updateMany"]);

/** The permission on a scope's own object that shows the scope. */
export const SCOPE_READ = "read";

/**
 * The permission to act as the principal an object stands for, such as the cell serving a space's zone.
 *
 * The object's identifier is the principal's, and its `parent` column, like a scope row's, is the scope containing the principal.
 */
export const REPRESENT = "represent";

/** The permission to copy the access rows of the scopes above a scope: on the caller's own object living in it, or on the scope's own object. */
export const REPLICATE = "replicate";

/** Every trait, in application order. */
const TRAITS: readonly Trait<any>[] = [
    record,
    nested,
    transitions,
    recoverable,
    expiring,
    addressed,
    versioned,
    controlled,
    declarable,
    detachable,
    shareable,
    suspendable,
    attachments,
    tracked,
    text,
];

/** A trait an object type takes, with the options its definition gives it. */
export interface AppliedTrait {
    /** The trait. */
    readonly trait: Trait<any>;
    /** The trait's options from the definition. */
    readonly options: unknown;
}

/** A relation an object type declares, with object types as subjects. */
export interface ObjectRelationInput {
    /** The subject types the relation accepts. */
    readonly subjects: readonly (RelationInput["subjects"][number] | ObjectType)[];
    /** The permission granting the relation, the object's when absent, none when null. */
    readonly grantedBy?: string | null;
}

/** The scope objects live in: a scope type, or the universe. */
export type ObjectScope =
    | typeof Scope.universe.id
    | ObjectType<Table, any, any, any, any, any, any, any, any>;

/** The traits an object opts into. */
export interface ObjectTraits<Declared = unknown, Permissions extends string = string> {
    /** The parent object type, or "self" for a tree. */
    readonly nested?: NestedDefinition;
    /** Deleted objects stay restorable for a window. */
    readonly recoverable?: RecoverableDefinition<Permissions>;
    /** The rules after which the system removes the objects. */
    readonly expiring?: readonly ExpiryRule[];
    /** Copy each object into its recipient's home. */
    readonly addressed?: AddressedDefinition;
    /** The scopes inside each object's scope copy the objects, those matching the condition when given. */
    readonly inherited?: { readonly where?: Condition };
    /** The objects are immutable, numbered versions of their parent. */
    readonly versioned?: VersionsDefinition;
    /** A system controller reconciles the objects. */
    readonly controlled?: true;
    /** Stacks declare the objects. */
    readonly declarable?: DeclarableDefinition<Declared>;
    /** Holders of the permission detach a declared record from its declaration. */
    readonly detachable?: Gated<Permissions>;
    /** Holders of the permission share the objects. */
    readonly shareable?: Gated<Permissions>;
    /** Holders of the permission suspend and resume a scope object. */
    readonly suspendable?: Gated<Permissions>;
    /** Every change stays in the log, grouped into activities. */
    readonly tracked?: TrackedDefinition<Permissions>;
    /** Whether the audit also records reads. */
    readonly audited?: { readonly reads: true };
}

/** The definition of an object type. */
export interface ObjectDefinition<
    Declared = unknown,
    Permissions extends string = string,
    Methods extends Readonly<Record<string, Method<MethodKind, Permissions | null>>> = {},
    Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope | readonly ObjectScope[],
> extends ObjectTraits<Declared, NoInfer<Permissions>> {
    /** The singular name, unique within the declaring package. */
    readonly name: string;
    /** The prefix of the objects' identifiers, the name when absent. */
    readonly identity?: string;
    /** The plural name. */
    readonly plural: string;
    /** The scope type containing each object, or the types when objects live in several. */
    readonly scope: Scope;
    /** Attributes permission expressions read. */
    readonly attributes?: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** Further relations relationships hold. */
    readonly relations?: Readonly<Record<string, ObjectRelationInput>>;
    /** Permission names roles grant, or named expressions derived from relations. */
    readonly permissions?: readonly Permissions[] | Readonly<Record<Permissions, AccessExpression>>;
    /** Permissions only their expressions grant, never roles. */
    readonly reserved?: readonly NoInfer<Permissions>[];
    /** Sensitive permissions that apply only after the authentication each names. */
    readonly elevated?: Readonly<Partial<Record<NoInfer<Permissions>, Elevation>>>;
    /** Permissions available while the object's scope is suspended. */
    readonly administration?: readonly NoInfer<Permissions>[];
    /** An access-owned type whose identity the objects take. */
    readonly represents?: Policy;
    /** Whether the objects are scopes holding other objects. */
    readonly isScope?: boolean;
    /** The operations callers may execute, keyed by method name. */
    readonly methods?: Methods;
    /** The fields each object holds, for objects declared by fields. */
    readonly fields?: Readonly<Record<string, Field>>;
    /** The aggregates of these objects their holder keeps, by field name. */
    readonly aggregates?: Readonly<Record<string, ObjectAggregate>>;
    /** The attachments the objects take. */
    readonly attachments?: readonly Attachment[];
    /** Indexes, unique constraints and checks over the derived columns. */
    readonly constraints?: (columns: never) => readonly TableConstraint[];
    /** Unique indexes the key index keeps across databases, by name. */
    readonly indexes?: Readonly<Record<string, ObjectIndex>>;
    /** Where the objects live. */
    readonly storage?: ObjectStorage;
    /** The previous names of renamed fields, by current field. */
    readonly moved?: { readonly fields?: Readonly<Record<string, string>> };
    /** The fields each release computes from stored rows and earlier callers' inputs, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** The tier of every database holding the objects, any tier when absent. */
    readonly tier?: DatabaseTier;
    /** The system controller reconciling the objects with work waiting. */
    readonly controller?: ObjectController;
    /** How long ephemeral objects outlive their session, 10 s by default. */
    readonly linger?: Duration;
}

/** Where objects live: durable in the scope's database, or ephemeral in instances' memory. */
export type ObjectStorage = "durable" | "ephemeral";

/** A unique index the key index keeps across databases. */
export interface ObjectIndex {
    /** The indexed fields, in key order. */
    readonly on: readonly string[];
    /** Whether each key names at most one object. */
    readonly unique: true;
    /** The scope the keys are unique within. */
    readonly across: ObjectScope;
}

/** A declared object type with its storage, permissions, methods and declaration schema. */
export class ObjectType<
    Definition extends Table = Table,
    Declared = unknown,
    Permissions extends string = string,
    Methods extends Readonly<Record<string, Method<MethodKind, Permissions | null>>> = Readonly<
        Record<string, Method<MethodKind, Permissions | null>>
    >,
    Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope | readonly ObjectScope[],
    Guarded extends string = string,
    Written extends string = string,
    Sensitive extends string = string,
    Storage extends ObjectStorage = ObjectStorage,
    Text extends string = string,
> {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The singular name. */
    readonly name: string;
    /** The prefix of the objects' identifiers. */
    readonly identity: string;
    /** The plural name. */
    readonly plural: string;
    /** The table holding one record per object. */
    readonly table: Definition;
    /** The scope type containing each object, or the types when objects live in several. */
    readonly scope: Scope;
    /** The scope types containing the objects, none for objects outside every scope. */
    readonly scopes: readonly ObjectType[];
    /** The object's relations and permissions, evaluated by access. */
    readonly policy: Policy;
    /** The fields each object holds, for objects declared by fields. */
    readonly fields: Readonly<Record<string, Field>>;
    /** The fields needing their own read permission. */
    readonly guarded: readonly Guarded[];
    /** The fields callers write. */
    readonly written: readonly Written[];
    /** The columns holding sensitive values. */
    readonly sensitive: readonly Sensitive[];
    /** The text fields, held in chunks. */
    readonly text: readonly Text[];
    /** Whether reads of the objects are audited. */
    readonly isReadAudited: boolean;
    /** The rows the scopes inside each object's scope copy, for inherited objects. */
    readonly inherited?: { readonly where?: Condition };
    /** The ancestor index of a tree of this object type. */
    readonly tree?: Tree;
    /** The tables a database holding the object needs, access tables included. */
    readonly tables: readonly Table[];
    /** The permission names callers may hold on the object. */
    readonly permissions: readonly Permissions[];
    /** The schema of one declaration in a stack, when stacks may declare the object. */
    readonly declarationSchema?: schema.Schema<Declared>;
    /** The operations callers may execute, keyed by method name. */
    readonly methods: Methods;
    /** How deleted objects stay restorable, for recoverable objects. */
    readonly recoverable?: RecoverableDefinition<Permissions>;
    /** When the system removes the objects, for expiring objects. */
    readonly expiring?: readonly ExpiryRule[];
    /** The system controller reconciling the objects with work waiting. */
    readonly controller?: ObjectController;
    /** The relations of the objects to the scope they live in, by the name of the scope's type, that permissions read through. */
    readonly enclosing: readonly string[];
    /** How a stack's declarations of the objects become their managed records. */
    readonly declaration?: ObjectDeclaration;
    /** How addressed objects name their recipient. */
    readonly addressed?: AddressedDefinition;
    /** Whether the objects are numbered versions of their parent. */
    readonly versioned?: VersionsDefinition;
    /** How tracked objects keep their history. */
    readonly tracked?: TrackedDefinition<Permissions>;
    /** The parent of nested objects. */
    readonly parent?: {
        /** The owning object type, or "any" for attachments. */
        readonly object: ObjectType | "any";
        /** Whether an object may have no parent. */
        readonly optional: boolean;
        /** The parent permission a caller needs to create or move an object under it. */
        readonly receive: string;
        /** Delete objects with their parent, or refuse deleting a parent holding any. */
        readonly delete: "cascade" | "restrict";
    };
    /** The aggregates of these objects their holder keeps, by field name. */
    readonly aggregates: Readonly<Record<string, ObjectAggregate>>;
    /** The attachments the objects take. */
    readonly attachments: readonly Attachment[];
    /** The traits the objects take, with their options. */
    readonly traits: readonly AppliedTrait[];
    /** The unique indexes the key index keeps across databases, by name. */
    readonly indexes: Readonly<Record<string, ObjectIndex>>;
    /** Where the objects live. */
    readonly storage: Storage;
    /** How long ephemeral objects outlive their session, in milliseconds. */
    readonly linger?: number;
    /** Where access finds objects stored in a table another package owns. */
    readonly intrinsic?: Omit<TableMapping, "policy">;
    /** The previous names of renamed fields, by current field. */
    readonly moved: Readonly<Record<string, string>>;
    /** The fields each release computes from stored rows and earlier callers' inputs. */
    readonly convert: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;

    /** Retain a definition with its table, methods and traits, and assemble its policy. */
    constructor(
        owner: Package,
        definition: ObjectDefinition<Declared, Permissions, Methods, Scope> & {
            readonly table: Definition;
        },
        traits: readonly AppliedTrait[],
        intrinsic?: Omit<TableMapping, "policy">,
    ) {
        // require a logged table
        if (definition.table[TABLE].retention === "none") {
            throw new TypeError(`object table is not logged: ${definition.table[TABLE].name}`);
        }

        // retain identity, storage and declaration
        this.package = owner;
        this.name = definition.name;
        this.identity = definition.identity ?? definition.name;
        this.plural = definition.plural;
        this.table = definition.table;
        this.scope = definition.scope;
        const scopes: readonly ObjectScope[] = [
            definition.scope as ObjectScope | readonly ObjectScope[],
        ].flat();
        this.scopes = scopes.filter((scope): scope is ObjectType => scope !== Scope.universe.id);
        this.declarationSchema = definition.declarable?.schema;
        this.recoverable = definition.recoverable;
        this.expiring = definition.expiring;
        this.controller = definition.controller;
        this.addressed = definition.addressed;
        this.versioned = definition.versioned;
        this.tracked = definition.tracked;
        for (const [name, method] of Object.entries(definition.methods ?? {}) as [
            string,
            Method,
        ][]) {
            Version.requireUpTo(method.convert ?? {}, owner.version, `${definition.name}.${name}`);
        }
        this.moved = definition.moved?.fields ?? {};
        this.convert = definition.convert ?? {};
        for (const [field, previous] of Object.entries(this.moved)) {
            if (previous in (definition.fields ?? {})) {
                throw new TypeError(
                    `field ${definition.name}.${field} moved from ${previous}, which names a current field`,
                );
            }
        }
        Version.requireUpTo(this.convert, owner.version, definition.name);
        this.isReadAudited = definition.audited?.reads === true;
        this.inherited = definition.inherited;
        this.methods = definition.methods ?? ({} as Methods);
        this.parent = definition.nested && {
            object: definition.nested.in === "self" ? this : definition.nested.in,
            optional: definition.nested.optional ?? false,
            receive: definition.nested.receive,
            delete: definition.nested.delete ?? "cascade",
        };
        this.aggregates = definition.aggregates ?? {};
        this.attachments = definition.attachments ?? [];
        this.traits = traits;
        this.intrinsic = intrinsic;
        this.indexes = definition.indexes ?? {};
        this.storage = (definition.storage ?? "durable") as Storage;
        if (this.storage === "ephemeral") {
            if (definition.linger !== undefined) {
                Duration.require(definition.linger, `linger of ${definition.name}`);
            }
            this.linger =
                definition.linger === undefined
                    ? LINGER_MILLISECONDS
                    : Duration.milliseconds(definition.linger);
        }

        // resolve declared relations
        const relations: Record<string, RelationInput> = {};
        for (const [name, relation] of Object.entries(definition.relations ?? {})) {
            relations[name] = {
                subjects: relation.subjects.map((subject) =>
                    subject instanceof ObjectType
                        ? subject.policy
                        : typeof subject === "string"
                          ? subjectType(owner.id, subject)
                          : subject,
                ),
                ...(relation.grantedBy === undefined ? {} : { grantedBy: relation.grantedBy }),
            };
        }
        const permissions = definition.permissions ?? [];
        const decided = new Set(
            Array.isArray(permissions)
                ? []
                : Object.values(permissions as Readonly<Record<string, AccessExpression>>).flatMap(
                      relationsOf,
                  ),
        );
        // derive relations from fields permissions read
        for (const [property, field] of Object.entries(definition.fields ?? {})) {
            // skip the fields no permission reads
            const name = kebabCase(property);
            if (!decided.has(name)) {
                continue;
            }

            // relate a principal reference to its kind
            if (field.type === "reference" && field.principals !== undefined) {
                relations[name] = { subjects: field.principals, grantedBy: null };
            }
            // relate a plain reference to its target type
            else if (field.type === "reference" && field.target && !field.qualified) {
                relations[name] = { subjects: [field.target().policy], grantedBy: null };
            }
            // relate a subject field to its principal kinds
            else if (field.type === "subject") {
                relations[name] = {
                    subjects: field.principals ?? [
                        principal.user,
                        principal.host,
                        principal.installation,
                    ],
                    grantedBy: null,
                };
            }
        }

        // relate the objects to the scope they live in, under its type's name, when a permission reads through it
        const enclosing = this.scopes.filter(
            (scope) => decided.has(scope.name) && relations[scope.name] === undefined,
        );
        for (const scope of enclosing) {
            if (scope.scopes.length > 0) {
                throw new TypeError(
                    `object ${definition.name} reads through its scope ${scope.name}, which is no object of the universe`,
                );
            }
            relations[scope.name] = { subjects: [scope.policy], grantedBy: null };
        }
        this.enclosing = enclosing.map((scope) => scope.name);
        this.fields = definition.fields ?? {};
        this.text = Object.entries(this.fields)
            .filter(([, declared]) => declared.type === "text")
            .map(([name]) => name as Text);

        // add trait policies
        const names = Array.isArray(permissions) ? permissions : Object.keys(permissions);
        const object = traitObject(this, (definition.represents?.package ?? owner).id);
        const policies = traits.flatMap(({ trait, options }) =>
            trait.policy === undefined ? [] : [trait.policy(options, object, names)],
        );
        const derived: Record<string, AccessExpression> = Object.assign(
            {},
            ...policies.map((policy) => policy.permissions ?? {}),
        );
        Object.assign(relations, ...policies.map((policy) => policy.relations ?? {}));

        // register the policy
        this.permissions = [...names, ...Object.keys(derived)] as Permissions[];
        const represented = definition.represents;
        if (represented && represented.name !== definition.name) {
            throw new TypeError(`object ${definition.name} cannot represent ${represented.name}`);
        }
        this.policy = new Policy(represented?.package ?? owner, {
            name: definition.name,
            attributes: Object.assign(
                {},
                ...policies.map((policy) => policy.attributes ?? {}),
                definition.attributes ?? {},
            ),
            relations: { ...representedRelations(represented), ...relations },
            permissions: {
                ...(represented?.definition.permissions as Readonly<
                    Record<Permissions, AccessExpression>
                >),
                ...(Array.isArray(permissions)
                    ? Object.fromEntries(permissions.map((name) => [name, none()]))
                    : (permissions as Readonly<Record<Permissions, AccessExpression>>)),
                ...derived,
            },
            ...(definition.shareable === undefined ? {} : { grantedBy: definition.shareable.by }),
            ...(definition.reserved === undefined ? {} : { reserved: definition.reserved }),
            ...(definition.elevated === undefined ? {} : { elevated: definition.elevated }),
            ...(definition.administration === undefined
                ? {}
                : { administration: definition.administration }),
            ...(definition.isScope || represented?.definition.scope === true
                ? { scope: true }
                : {}),
            contributes: policies.flatMap((policy) => policy.contributes ?? []),
        });

        // list the tables a database needs
        this.tree = this.table[TABLE].tree;
        this.tables =
            this.storage === "durable"
                ? [
                      this.table,
                      ...serverTables,
                      ...(this.addressed === undefined ? [] : [copy]),
                      ...(this.text.length === 0 ? [] : [chunk, chunkRun]),
                  ]
                : [this.table];

        // refuse method names functions hold
        const shadowed = Object.keys(this.methods).find((name) =>
            Object.getOwnPropertyNames(Function.prototype).includes(name),
        );
        if (shadowed !== undefined) {
            throw new TypeError(
                `object ${this.name} names a method ${shadowed}, which functions hold`,
            );
        }

        // require declared permissions and valid methods
        for (const declared of Object.values(this.methods)) {
            if (declared.permission !== null && !this.permissions.includes(declared.permission)) {
                throw new TypeError(
                    `object ${definition.name} has no permission ${declared.permission}`,
                );
            } else if (
                declared.permission === null &&
                RECORD_KINDS.has(declared.kind) &&
                declared.isSystem !== true
            ) {
                throw new TypeError(
                    `object ${definition.name} ${declared.kind}s without a permission outside the system`,
                );
            }
            declared.validate?.(this);
        }
        for (const [name, declared] of Object.entries(this.fields)) {
            for (const permission of [declared.access?.read, declared.access?.write]) {
                if (
                    permission !== undefined &&
                    !this.permissions.includes(permission as Permissions)
                ) {
                    throw new TypeError(
                        `object ${definition.name} field ${name} has no permission ${permission}`,
                    );
                }
            }
        }

        // require each aggregate's holder field
        for (const [name, aggregate] of Object.entries(this.aggregates)) {
            const holding =
                aggregate.via === undefined
                    ? this.parent?.object
                    : this.fields[aggregate.via]?.target?.();
            if (holding === undefined) {
                throw new TypeError(
                    `aggregate ${name} of ${definition.name} fills no ${aggregate.function} field of its holder`,
                );
            } else if (holding !== "any") {
                holding.requireAggregate(this, name, aggregate);
            }
        }

        // refuse aggregate fields in method inputs
        for (const [name, declared] of Object.entries(this.methods) as [string, Method][]) {
            const shape = (declared.input as { shape?: Record<string, unknown> } | undefined)
                ?.shape;
            for (const field of Object.keys(shape ?? {})) {
                if (this.fields[field]?.aggregate !== undefined) {
                    throw new TypeError(
                        `method ${name} of ${definition.name} writes aggregate field ${field}`,
                    );
                }
            }
        }

        // collect written fields
        this.written = Object.entries(this.fields)
            .filter(
                ([, declared]) =>
                    !declared.isCaller &&
                    declared.aggregate === undefined &&
                    declared.machine === undefined &&
                    declared.type !== "text",
            )
            .map(([name]) => name as Written);

        // collect sensitive columns
        this.sensitive = Object.entries(this.table[TABLE].columns)
            .filter(([, column]) => column.definition.classification === "sensitive")
            .map(([name]) => name as Sensitive);

        // collect guarded fields
        this.guarded = Object.entries(this.fields)
            .filter(([, declared]) => declared.access?.read !== undefined)
            .map(([name]) => name as Guarded);

        // require readable scope types
        for (const scope of this.scopes) {
            if (scope.policy.definition.scope !== true) {
                throw new TypeError(
                    `object ${this.name} lives in ${scope.name}, which is no scope`,
                );
            } else if (!scope.permissions.includes(SCOPE_READ)) {
                throw new TypeError(
                    `object ${this.name} lives in ${scope.name}, which declares no ${SCOPE_READ} permission`,
                );
            }
        }

        // require each index over logged fields
        const { logged } = this.table[TABLE];
        for (const [name, declared] of Object.entries(this.indexes)) {
            const missing = declared.on.find((field) => !Object.hasOwn(this.fields, field));
            const unlogged = declared.on.find((field) => !Object.hasOwn(logged, field));
            if (missing !== undefined) {
                throw new TypeError(`index ${name} of ${this.name} names no field ${missing}`);
            } else if (unlogged !== undefined) {
                throw new TypeError(
                    `index ${name} of ${this.name} names unlogged field ${unlogged}`,
                );
            } else if (
                declared.across !== Scope.universe.id &&
                !this.ancestors.some((ancestor) => ancestor.same(declared.across as ObjectType))
            ) {
                throw new TypeError(
                    `index ${name} of ${this.name} is unique within a scope enclosing its objects or across every scope`,
                );
            }
        }

        // refuse durable-only declarations on ephemeral objects
        if (this.storage === "durable" && definition.linger !== undefined) {
            throw new TypeError(`object ${this.name} lingers without being ephemeral`);
        } else if (this.storage === "ephemeral") {
            const durable = [
                ...traits.filter(({ trait }) => trait.isDurable).map(({ trait }) => trait.key!),
                ...(
                    [
                        "audited",
                        "inherited",
                        "controller",
                        "isScope",
                        "aggregates",
                        "indexes",
                    ] as const
                ).filter((key) => definition[key] !== undefined && definition[key] !== false),
            ];
            if (durable.length > 0) {
                throw new TypeError(`ephemeral object ${this.name} takes no ${durable[0]}`);
            }
            const external = Object.entries(this.methods as Readonly<Record<string, Method>>).find(
                ([, declared]) => declared.prepare !== undefined || declared.settle !== undefined,
            );
            if (external !== undefined) {
                throw new TypeError(
                    `ephemeral object ${this.name} does no external work in method ${external[0]}`,
                );
            }
        }

        // validate each trait
        for (const { trait, options } of traits) {
            trait.validate?.(options, this, definition);
        }
    }

    /** The input conversions of a method: renamed fields, the object's conversions, then the method's own. */
    conversions(name: string): Readonly<Record<Version, Readonly<Record<string, Expression>>>> {
        // rename previous field names in calls of releases before this one
        const renames = Object.fromEntries(
            Object.entries(this.moved).map(([field, previous]) => [
                field,
                Expression.column(previous),
            ]),
        );
        const releases: Record<string, Record<string, Expression>> = Object.keys(renames).length ===
        0
            ? {}
            : { [this.package.version]: renames };

        // merge the object's and the method's assignments by release
        const method = (this.methods as Readonly<Record<string, Method>>)[name];
        for (const conversions of [this.convert, method?.convert ?? {}]) {
            for (const [release, assignments] of Object.entries(conversions)) {
                releases[release] = { ...releases[release], ...assignments };
            }
        }

        return releases;
    }

    /** The audit target name of the objects' method events. */
    get auditTarget(): string {
        return camelCase(this.name);
    }

    /** Derive the audit action `Noun.method` recording one method. */
    audit(method: string, target: string = this.auditTarget): AuditAction {
        // read the method's audit details
        const declared = (this.methods as Readonly<Record<string, Method>>)[method]?.audit;

        return {
            package: this.package,
            name: `${pascalCase(this.name)}.${method}`,
            targets: schema.object({ [target]: AuditTarget }),
            details: declared === undefined ? schema.object({}) : declared.details.partial(),
        };
    }

    /** Build the audit action and values of a call: its object for a targeted method, else the scope's collection. */
    auditCall(method: string, input: Readonly<Record<string, unknown>>, scope: string) {
        // target one object
        const declared = (this.methods as Readonly<Record<string, Method>>)[method];
        if (declared?.target === true) {
            const id = schema.string().parse(input.id);
            const targets = { [this.auditTarget]: { type: this.name, id } };

            return { action: this.audit(method), values: { targets, details: {} } };
        }

        // target the scope's collection
        const targets = { collection: { type: this.plural, id: scope } };

        return { action: this.audit(method, "collection"), values: { targets, details: {} } };
    }

    /** Accept the members of one of this type's relations as subjects. */
    members(relation: string): PolicySubject {
        return this.policy.members(relation);
    }

    /** Accept every object of this type through one relationship. */
    all(): PolicySubject {
        return this.policy.all();
    }

    /** Determine whether a subject is one object of this type. */
    is(subject: Subject): boolean {
        return this.policy.is(subject);
    }

    /** Reference one object in its scope. */
    reference(scope: string, id: string): ObjectReference {
        return this.policy.reference(scope, id);
    }

    /** The table mapping access reads the objects through. */
    get mapping(): TableMapping {
        // copy every inherited row the condition matches
        const inherited =
            this.inherited === undefined
                ? {}
                : { inherited: this.inherited.where ?? Condition.all() };

        // reuse an intrinsic mapping
        if (this.intrinsic !== undefined) {
            return { ...this.intrinsic, policy: this.policy, ...inherited };
        }

        // map each declared relation to its field
        const relations: Record<string, TableMapping["relations"][string]> = {};
        for (const [property, field] of Object.entries(this.fields)) {
            const name = kebabCase(property);
            const isRelation = Object.hasOwn(this.policy.definition.relations, name);

            // hold a subject field of a relation as a subject key
            if (isRelation && field.type === "subject") {
                relations[name] = { column: property, isKey: true };
            }
            // map a principal reference in the universe
            else if (isRelation && field.type === "reference" && field.principals !== undefined) {
                relations[name] = { column: property, scope: Scope.universe.id };
            }
            // map a plain reference by identifier
            else if (isRelation && field.type === "reference" && field.target && !field.qualified) {
                relations[name] =
                    field.target().scope === Scope.universe.id
                        ? { column: property, scope: Scope.universe.id }
                        : { column: property };
            }
        }
        // map each relation to the scope the objects live in, an object of the universe
        for (const name of this.enclosing) {
            relations[name] = { column: "scope", scope: Scope.universe.id };
        }

        // add trait mappings
        const object = traitObject(this, this.policy.package.id);
        const located = this.traits.flatMap(({ trait, options }) =>
            trait.mapping === undefined ? [] : [trait.mapping(options, object)],
        );
        Object.assign(relations, ...located.map((mapping) => mapping.relations));

        // map a principal's self relation
        if (this.isPrincipal && Object.hasOwn(this.policy.definition.relations, "self")) {
            relations.self = { column: "id" };
        }

        // assemble the mapping
        return {
            policy: this.policy,
            table: this.table,
            id: "id",
            scope: "scope",
            attributes: Object.fromEntries(
                Object.keys(this.policy.definition.attributes).map((name) => [
                    name,
                    camelCase(name),
                ]),
            ),
            relations,
            ...inherited,
            ...Object.assign({}, ...located.map(({ relations: _relations, ...placed }) => placed)),
        };
    }

    /** Require a matching aggregate field with readers that may list every measured row. */
    requireAggregate(measured: ObjectType, name: string, aggregate: ObjectAggregate): void {
        // require this type's field
        if (this.fields[name]?.aggregate !== aggregate.function) {
            throw new TypeError(
                `aggregate ${name} of ${measured.name} fills no ${aggregate.function} field of ${this.name}`,
            );
        }

        // require its readers to list every measured row
        const guard = this.fields[name]!.access?.read;
        const readers = new Set(
            guard === undefined
                ? Object.values(this.methods as Readonly<Record<string, Method>>)
                      .filter((method) => method.kind === "get" || method.kind === "list")
                      .flatMap((method) => (method.permission === null ? [] : [method.permission]))
                : [guard],
        );
        const listing = measured.listing?.name;
        const link = aggregate.via ?? "parent";
        const permissions = measured.policy.definition.permissions;
        const uncovered = [...readers].filter(
            (reader) =>
                listing === undefined || !covers(permissions, listing, link, reader, new Set()),
        );
        if (uncovered.length > 0) {
            throw new TypeError(
                `aggregate ${name} of ${measured.name} measures rows that readers of ${this.name} may not list: list them through ${link} ${uncovered.join(", ")}`,
            );
        }
    }

    /** The permission listing the objects needs, absent without a list method. */
    get listing(): Permission | undefined {
        return methodPermission(this, "list");
    }

    /** The permission reading one object needs, the listing's without a get method, or none. */
    get reading(): Permission | undefined {
        return methodPermission(this, "get") ?? this.listing;
    }

    /** Whether the objects represent a principal kind. */
    get isPrincipal(): boolean {
        return Object.values(principal).some(
            (kind) =>
                kind.definition.packageId === this.policy.definition.packageId &&
                kind.name === this.policy.name,
        );
    }

    /** Match the objects living in one scope, as SQL. */
    inScope(scope: string): SQL {
        return eq(this.table[TABLE].columns.scope!, scope);
    }

    /** Take these objects as attachments of a host. */
    attach(options: { readonly by: string }): Attachment {
        if (this.parent?.object !== "any") {
            throw new TypeError(`object ${this.name} is not nested in any parent`);
        }

        return { object: this, by: options.by };
    }

    /** Build calls of the objects' mutating methods, to run later in the scope they are sent to or the one their input adds. */
    calls<Self extends ObjectType>(this: Self): Calls<Self> {
        const methods = Object.entries(this.methods as Readonly<Record<string, Method>>);
        const calls = methods
            .filter(([, method]) => method.mutates)
            .map(([name]) => [
                name,
                (input: Readonly<Record<string, unknown>>) => Call.record(this, name, input),
            ]);

        return Object.fromEntries(calls) as unknown as Calls<Self>;
    }

    /** Wrap methods' effects in handlers. */
    handle<Self extends ObjectType>(
        this: Self,
        handlers: {
            readonly [Name in keyof Self["methods"]]?:
                | Handler<Self["table"]>
                | Phases<Self["table"]>;
        },
    ): Self {
        // wrap each named method's effect
        const methods: Record<string, Method> = { ...this.methods };
        for (const [name, handler] of Object.entries(handlers) as [string, Handler | Phases][]) {
            const declared = methods[name];
            if (declared === undefined) {
                throw new TypeError(`object ${this.name} has no method ${name}`);
            }
            methods[name] = declared.handle(handler);
        }

        // copy the object type with the handled methods
        return this.with({ methods: methods as Self["methods"] });
    }

    /** Set how a stack's declarations of the objects become their managed records. */
    declare<Self extends ObjectType, Collected = DeclarationOf<Self>, Resolved = Collected>(
        this: Self,
        declaration: ObjectDeclaration<Self, Collected, Resolved>,
    ): Self {
        return this.with({ declaration: declaration as ObjectDeclaration });
    }

    /** Reconcile or follow the objects with work waiting as the system. */
    control<Self extends ObjectType>(
        this: Self,
        controller: ObjectController<Select<Self["table"]>>,
    ): Self {
        return this.with({ controller: controller as unknown as ObjectController });
    }

    /** Copy the object type with some members changed, sharing its table. */
    with<Self extends ObjectType>(
        this: Self,
        changes: Partial<
            Pick<
                Self,
                "methods" | "recoverable" | "expiring" | "traits" | "controller" | "declaration"
            >
        >,
    ): Self {
        return Object.assign(
            Object.create(Object.getPrototypeOf(this) as object) as Self,
            this,
            changes,
        );
    }

    /** Reference one of the object's declared permissions. */
    permission(name: Permissions): Permission {
        if (!this.permissions.includes(name)) {
            throw new TypeError(`object ${this.name} has no permission ${name}`);
        }

        return this.policy.permission(name);
    }

    /** Where the objects' routes name their scope. */
    get route(): ScopeRoute {
        return scopeRoute(this);
    }

    /** The schemas of the objects' rows and of the inputs their methods take. */
    get schema(): ObjectSchema {
        return objectSchema(this);
    }

    /** Whether a controller reconciles the objects to their desired generation. */
    get isControlled(): boolean {
        return this.traits.some((applied) => applied.trait === controlled);
    }

    /** The scope types enclosing each object, nearest first, for objects in one scope type. */
    get ancestors(): readonly ObjectType[] {
        const ancestors: ObjectType[] = [];
        for (let scope = this.scope; scope instanceof ObjectType; scope = scope.scope) {
            ancestors.push(scope);
        }

        return ancestors;
    }

    /** Read the directory's identity of one of the type's unique indexes. */
    index(name: string): string {
        return `${this.policy.definition.packageId}/${this.name}/${name}`;
    }

    /** Key values in one of the type's indexes, within the scope the index is unique across. */
    claim(name: string, values: readonly unknown[], scope?: string): Pick<Claim, "index" | "key"> {
        // require the index and a needed scope
        const declared = this.indexes[name];
        if (declared === undefined) {
            throw new TypeError(`object ${this.name} has no index ${name}`);
        }
        const within = declared.across === Scope.universe.id ? null : scope;
        if (within === undefined) {
            throw new TypeError(`index ${name} of ${this.name} keys values within a scope`);
        }

        return { index: this.index(name), key: canonicalize([within, ...values]) };
    }

    /** List the names a row claims in the type's indexes with enclosing scopes from a snapshot. */
    async claims(row: Readonly<Record<string, unknown>>, snapshot: Snapshot): Promise<Claim[]> {
        // key each index the row holds every value of
        const scope = String(row.scope);
        const claims: Claim[] = [];
        for (const [name, declared] of Object.entries(this.indexes)) {
            // skip rows missing a value
            const values = declared.on.map((field) => row[field]);
            if (values.some((value) => value === null || value === undefined)) {
                continue;
            }

            // key the values within the scope the index is unique across
            const within = await this.within(declared.across, scope, snapshot);
            claims.push({
                index: this.index(name),
                key: canonicalize([within, ...values]),
                objectId: String(row.id),
                scope,
            });
        }

        return claims;
    }

    /** Describe the names an object claims after a write, none after deletion. */
    async owned(
        objectId: string,
        row: Readonly<Record<string, unknown>> | undefined,
        snapshot: Snapshot,
    ): Promise<ObjectClaims> {
        return {
            indexes: Object.keys(this.indexes).map((name) => this.index(name)),
            objectId,
            claims: row === undefined ? [] : await this.claims(row, snapshot),
        };
    }

    /** Look up the object owning values in one of the type's indexes. */
    async lookup(
        directory: Directory,
        name: string,
        values: readonly unknown[],
        scope?: string,
    ): Promise<ObjectReference | undefined> {
        const { index, key } = this.claim(name, values, scope);
        const found = await directory.owner(index, key);

        return found === undefined ? undefined : this.reference(found.scope, found.objectId);
    }

    /** Read the names the indexed rows an open transaction wrote claim, one entry per written object. */
    static async written(
        transaction: DatabaseConnection,
        objects: readonly ObjectType[],
    ): Promise<ObjectClaims[]> {
        // read each written row's last image
        const indexed = objects.filter((object) => Object.keys(object.indexes).length > 0);
        const byTable = new Map(indexed.map((object) => [object.table as Table, object]));
        const owned = new Map<string, ObjectClaims>();
        const snapshot = Snapshot.live(transaction);
        if (indexed.length > 0) {
            for (const change of await transaction.log.written([...byTable.keys()])) {
                const object = byTable.get(change.table)!;
                const objectId = String(change.key.id);
                const row = change.after as Readonly<Record<string, unknown>> | undefined;
                owned.set(
                    `${object.name}/${objectId}`,
                    await object.owned(objectId, row, snapshot),
                );
            }
        }

        return [...owned.values()];
    }

    /** Find the scope an index is unique across, from the scope an object lives in. */
    async within(
        across: ObjectType["indexes"][string]["across"],
        scope: string,
        snapshot: Snapshot,
    ): Promise<string | null> {
        // key global indexes without a scope, and indexes across the object's own scope by it
        if (across === Scope.universe.id) {
            return null;
        } else if (across.same(this.ancestors[0])) {
            return scope;
        }

        // find the enclosing scope of the declared type
        const { packageId, name } = across.policy.definition;
        const chain = await Scope.chain(snapshot, scope);
        const enclosing = chain.find(
            (link) => link.object.packageId === packageId && link.object.type === name,
        );
        if (enclosing === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `scope ${scope} has no enclosing ${name}`,
            });
        }

        return enclosing.object.id;
    }

    /** Read the scopes a query of the objects reads from a scope chain, nearest first. */
    scopesOf(chain: readonly string[]): string[] {
        return this.inherited === undefined ? [chain[0]!] : [...chain];
    }

    /** Decide whether another type shares this one's table. */
    same(other: ObjectType | "any" | undefined): boolean {
        return other instanceof ObjectType && other.table === this.table;
    }

    /** Compile a query or include of the objects. */
    query(shape: ObjectInclude, objects: readonly ObjectType[]): Omit<Query, "scopes"> {
        return compile(this, shape, objects);
    }

    /** Resolve the join an include name makes. */
    join(name: string, objects: readonly ObjectType[]): Join {
        return join(this, name, objects);
    }

    /** Compile a scope's named object queries. */
    static queries(
        objects: readonly ObjectType[],
        queries: Readonly<Record<string, ObjectQuery>> | undefined,
        chain: readonly string[],
    ): Record<string, Query> {
        return compileQueries(objects, queries, chain);
    }

    /** The procedures of the object's methods. */
    get procedures(): ServiceRouter {
        return objectProcedures(this);
    }

    /** List object types with the chunk type their text fields need. */
    static served<Object extends ObjectType>(objects: readonly Object[]): Object[] {
        const owners = objects.filter((object) => object.text.length > 0);

        return owners.length === 0 ? [...objects] : [...objects, Chunk.object(owners) as Object];
    }

    /** Decide whether objects of another type attach to these objects. */
    holds(child: ObjectType): boolean {
        return (
            this.attachments.some((attachment) => child.same(attachment.object)) ||
            (child.table === chunk && this.text.length > 0)
        );
    }

    /** The procedures every object type shares. */
    get shared(): { readonly replica: ReplicaProcedures } {
        return { replica: replicaProcedures };
    }
}

/** An aggregate of objects their holder keeps in one field. */
export interface ObjectAggregate {
    /** The aggregate function. */
    readonly function: AggregateFunction;
    /** The aggregated field, for sums, minimums and maximums. */
    readonly value?: string;
    /** The values the aggregated objects' fields hold. */
    readonly where?: Readonly<Record<string, string | number | boolean | null>>;
    /** The reference field naming the holding object, the parent when absent. */
    readonly via?: string;
}

/** A type of attachments a host takes. */
export interface Attachment {
    /** The attachment type, nested in any parent. */
    readonly object: ObjectType;
    /** The host permission whose holders attach. */
    readonly by: string;
}

/** An object definition deriving its table from fields and traits. */
export type ObjectFieldsDefinition<
    Name extends string,
    Fields extends Readonly<Record<string, Field>>,
    Permissions extends string,
    Methods extends Readonly<Record<string, Method<MethodKind, Permissions | null>>>,
    Scope extends ObjectScope | readonly ObjectScope[],
    Identity extends string = Name,
    Storage extends ObjectStorage = "durable",
> = Omit<
    ObjectDefinition<unknown, Permissions, Methods, Scope>,
    "name" | "identity" | "constraints" | "storage" | keyof ObjectTraits
> & {
    /** The singular name, unique within the declaring package. */
    readonly name: Name;
    /** The prefix of the objects' identifiers, the name when absent. */
    readonly identity?: Identity;
    /** The fields each object holds, by property name. */
    readonly fields: Fields;
    /** Where the objects live. */
    readonly storage?: Storage;
    /** Indexes, unique constraints and checks over the derived columns. */
    readonly constraints?: (
        columns: ConstraintColumns<Name, Scope, Fields, Identity, Storage>,
    ) => readonly TableConstraint[];
};

/** The shape of one stack declaration of a declarable object. */
export type DeclaredOf<Traits> =
    TraitOf<Traits, "declarable"> extends DeclarableDefinition<infer Declared> ? Declared : never;

/** The methods an object's traits derive beside its declared ones. */
export type TraitMethods<Fields, Traits> = TransitionMethodMap<Fields> &
    RecoverableMethodMap<TraitOf<Traits, "recoverable">> &
    ControlledMethodMap<TraitOf<Traits, "controlled">> &
    NestedMethodMap<TraitOf<Traits, "nested">> &
    ShareableMethodMap<GateOf<TraitOf<Traits, "shareable">>> &
    SuspendableMethodMap<GateOf<TraitOf<Traits, "suspendable">>> &
    DetachableMethodMap<GateOf<TraitOf<Traits, "detachable">>> &
    TrackedMethodMap<TraitOf<Traits, "tracked">> &
    TextMethodMap<Fields>;

/** Declare an object type from its fields. */
export function defineObject<
    const Name extends string,
    Fields extends Readonly<Record<string, Field>>,
    Permissions extends string = never,
    const Methods extends Readonly<
        Record<string, Method<MethodKind, NoInfer<Permissions> | null>>
    > = {},
    const Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope,
    const Traits extends ObjectTraits<unknown, NoInfer<Permissions>> = {},
    const Identity extends string = Name,
    const Storage extends ObjectStorage = "durable",
>(
    definition: ObjectFieldsDefinition<
        Name,
        Fields,
        Permissions,
        Methods,
        Scope,
        Identity,
        Storage
    > &
        Traits,
    module?: ModuleMetadata,
): ObjectType<
    ObjectTable<Name, Scope, Fields, Traits, Identity, Storage>,
    DeclaredOf<Traits>,
    Permissions,
    Methods & TraitMethods<Fields, Traits>,
    Scope,
    GuardedField<Fields>,
    WrittenField<Fields>,
    SensitiveField<Fields>,
    Storage,
    TextFieldName<Fields>
>;
/** Declare an object type over a table another package owns. */
export function defineObject<
    Definition extends Table,
    Declared = never,
    Permissions extends string = never,
    const Methods extends Readonly<Record<string, Method<MethodKind, Permissions | null>>> = {},
    const Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope,
>(
    definition: Omit<
        ObjectDefinition<Declared, Permissions, Methods, Scope>,
        | "identity"
        | "nested"
        | "recoverable"
        | "versioned"
        | "controlled"
        | "detachable"
        | "shareable"
        | "suspendable"
        | "tracked"
        | "attachments"
        | "constraints"
        | "aggregates"
    > & { readonly [INTRINSIC]: Intrinsic<Definition> },
    module?: ModuleMetadata,
): ObjectType<
    Definition,
    Declared,
    Permissions,
    Methods,
    Scope,
    never,
    never,
    never,
    "durable",
    never
>;
/** Declare an object type. */
export function defineObject(
    definition: ObjectDefinition & { readonly [INTRINSIC]?: Intrinsic },
    module?: ModuleMetadata,
): ObjectType {
    // collect the traits
    const owner = declaringModule(module, "defineObject").package;
    const intrinsic = definition[INTRINSIC];
    const traits = TRAITS.flatMap((trait) => {
        const options = intrinsic === undefined ? trait.options(definition) : undefined;

        return options === undefined ? [] : [{ trait, options }];
    });

    // derive the table
    const packageId = (definition.represents?.package ?? owner).id;
    const table = intrinsic?.table ?? deriveTable(definition, traits, packageId, module);

    // add trait methods
    const declared = (definition.methods ?? {}) as Readonly<Record<string, Method>>;
    const derived = traits.map(({ trait, options }) => trait.methods(options, declared));
    const clash = derived.flatMap(Object.keys).find((name) => Object.hasOwn(declared, name));
    if (clash !== undefined) {
        throw new TypeError(
            `object ${definition.name} declares method ${clash}, which a trait derives`,
        );
    }
    const methods = Object.assign({ ...declared }, ...derived);

    return new ObjectType(owner, { ...definition, table, methods }, traits, intrinsic?.mapping);
}

/** The name of a field whose values need their own read permission. */
export type GuardedField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly guarded: true } ? Name : never;
}[keyof Fields & string];

/** The name of a text field. */
export type TextFieldName<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends TextField ? Name : never;
}[keyof Fields & string];

/** The name of a field holding sensitive values. */
export type SensitiveField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly isSensitive: true }
        ? Name
        : never;
}[keyof Fields & string];

/** The name of a field callers write. */
export type WrittenField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly written: false }
        ? never
        : Name;
}[keyof Fields & string];

/** Type the procedures a service routes to object types. */
declare module "@destack/service" {
    /** Route object types to the procedures their methods derive. */
    interface RoutedProcedures<_Declaration> {
        /** The procedures of an object type's methods. */
        readonly object: _Declaration extends ObjectType ? ObjectProcedures<_Declaration> : never;
    }
}

/** Decide whether a permission admits the holders of a related object's permission. */
function covers(
    permissions: Readonly<Record<string, AccessExpression>>,
    name: string,
    relation: string,
    permission: string,
    seen: Set<string>,
): boolean {
    // follow each permission once
    if (seen.has(name)) {
        return false;
    }
    seen.add(name);

    // search unions and named permissions
    const admits = (expression: AccessExpression): boolean =>
        expression.kind === "union"
            ? expression.expressions.some(admits)
            : expression.kind === "permission"
              ? covers(permissions, expression.name, relation, permission, seen)
              : expression.kind === "through" &&
                expression.relation === relation &&
                expression.permission === permission;

    return permissions[name] !== undefined && admits(permissions[name]);
}

/** Read a represented policy's relations as input. */
function representedRelations(represented: Policy | undefined): Record<string, RelationInput> {
    return Object.fromEntries(
        Object.entries(represented?.definition.relations ?? {}).map(([name, relation]) => [
            name,
            { subjects: relation.subjects, grantedBy: relation.grantedBy ?? null },
        ]),
    );
}

/** Read the permission a type's standard method of a kind needs, absent without one. */
function methodPermission(object: ObjectType, kind: MethodKind): Permission | undefined {
    const method = Object.values(object.methods as Readonly<Record<string, Method>>).find(
        (declared) => declared.kind === kind,
    );

    return method?.permission === undefined || method.permission === null
        ? undefined
        : object.permission(method.permission);
}

/** Describe an object type to its traits. */
function traitObject(type: ObjectType, packageId: PackageId): TraitObject {
    return {
        name: type.name,
        identity: type.identity,
        packageId,
        table: () => type.table,
        storage: type.storage,
    };
}
