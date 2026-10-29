import { AuditTarget } from "../audit/execution.ts";
import { ObjectError } from "../error/error.ts";
import { Issues } from "../error/detail.ts";
import {
    Policy,
    none,
    type AccessExpression,
    type Elevation,
    type RelationInput,
    type PolicySubject,
    universe,
    type Permission,
    type TableMapping,
} from "@destack/access";
import {
    Scope,
    type ObjectReference,
    ObjectTypeReference,
    type Query,
    type Subject,
    Call,
} from "@destack/sync";
import {
    Expression,
    Condition,
    eq,
    inArray,
    DatabaseError,
    Key,
    type Row,
    type RowImage,
    type Select,
    sql,
    type SQL,
    TABLE,
    type Table,
    type Transaction,
    type Tree,
    type ReconcileController,
} from "@destack/db";
import {
    nth,
    Duration,
    Identifier,
    type UuidEnvironment,
    type JsonObject,
    present,
    schema,
    toJsonSchema,
    Version,
    JsonValue,
    mapValues,
    zip,
} from "@destack/schema";

import type { AuditAction } from "../audit/action.ts";
import { ModuleMetadata, type Package } from "@destack/package";
import {
    type Delivery,
    EventKind,
    EventReader,
    type EventRetention,
    EventTable,
} from "@destack/event";
import { Field, type FieldsByColumn, type ReferenceOf, type TextField } from "../field/field.ts";
import { type Handler, type TypesOf } from "../method/context.ts";
import { CallSite } from "../method/site.ts";
import { Method, type MethodKind, type Phase } from "../method/method.ts";
import { MethodBuilder } from "../method/record.ts";
import { InputSchema } from "../method/input.ts";
import {
    callableProcedures,
    objectProcedures,
    objectSchema,
    scopeRoute,
    type CallInput,
    type MethodOutput,
    type MethodName,
    type MethodProcedure,
    type ScopeInput,
    type TargetInput,
    type ObjectProcedures,
    type ScopeRoute,
    type ObjectSchema,
} from "../method/procedure.ts";
import { storeProcedures, type ObjectQuery, type StoreProcedures } from "../replica/replica.ts";
import type { RowFollow, Shape } from "../replica/shape.ts";
import { compile, compileQueries } from "../query/query.ts";
import { record } from "../trait/record.ts";
import { series, type SeriesMethodMap } from "../trait/series.ts";
import {
    type DeclaredReference,
    StackObjectReference,
    StackReference,
    StackSubject,
} from "../stack/reference.ts";
import {
    controlled,
    type ControlledDefinition,
    type ControlledMethodMap,
} from "../trait/controlled.ts";
import { attach, detach } from "../method/manager.ts";
import { nested, type NestedMethodMap, type NestedDefinition } from "../trait/nested.ts";
import { transitions, type TransitionMethodMap } from "../trait/transition.ts";
import {
    recoverable,
    type RecoverableDefinition,
    type RecoverableMethodMap,
} from "../trait/recoverable.ts";
import type { MethodLimits } from "../method/limits.ts";
import { expiring, type ExpiryRule } from "../trait/expiring.ts";
import { versioned, type VersionsDefinition } from "../trait/versioned.ts";
import {
    type RoleFieldsOf,
    type RolePermission,
    type RolePermissionsOf,
    Roles,
    type RolesDefinition,
    shareable,
    type ShareableDefinition,
    type ShareableMethodMap,
    type SharingGateOf,
} from "../trait/shareable.ts";
import { suspendable, type SuspendableMethodMap } from "../trait/suspendable.ts";
import { fencing, type FencingMethodMap } from "../trait/fencing.ts";
import { readable, type ReadableMethodMap } from "../trait/readable.ts";
import { writable, type WritableMethodMap } from "../trait/writable.ts";
import { bindable, type BindableMethodMap } from "../trait/bindable.ts";
import {
    Provisioned,
    type ProvisionedDefinition,
    type ProvisionedFieldsOf,
    type ProvisionedMethodsOf,
    type ProvisionedPermissionOf,
    type ProvisionedSharingOf,
    type ProvisionedTraitsOf,
} from "../trait/provisioned.ts";
import { attachments } from "../trait/attachment.ts";
import { tracked, type TrackedDefinition, type TrackedMethodMap } from "../trait/tracked.ts";
import { text, type TextMethodMap } from "../trait/text.ts";
import {
    Presentable,
    type PresentableDefinition,
    type PresentationFieldsOf,
    type Presentation,
} from "../trait/presentable.ts";
import {
    Orderable,
    orderable,
    type OrderableDefinition,
    type OrderedFieldsOf,
    type Ordering,
} from "../trait/orderable.ts";
import { ObjectSearch, search, type SearchDefinition } from "../trait/search.ts";
import { Chunk } from "../text/chunk.ts";
import { chunk, chunkRun } from "../text/table.ts";
import type { GateOf, Gated, Trait } from "../trait/trait.ts";
import { SYSTEM_TABLES } from "../stack/db.ts";
import { eventTables } from "@destack/event/stack";
import type { ObjectController } from "./controller.ts";
import { Consumer, type ReaderHandler } from "./consumer.ts";
import type { ObjectStore } from "../server/store.ts";

import type { DeclarationHandler } from "../server/stack.ts";
import type { ManagedTable } from "../trait/record.ts";
import { Casing } from "./name.ts";
import { ObjectPolicy } from "./access.ts";
import { type ConstraintColumn, ObjectTable, type TraitOf, type TraitTable } from "./table.ts";
import type { ObjectConstraint } from "./constraint.ts";

import type {} from "@destack/package/import-meta";

/** The schemas and procedures an object type derives from its definition, built lazily. */
interface Derived {
    /** The schemas of the objects' rows and method inputs. */
    schema?: ObjectSchema;
    /** The procedures of the methods, by method. */
    readonly procedure: Map<string, MethodProcedure>;
    /** The inputs of the methods as clients call them, by method. */
    readonly input: Map<string, InputSchema>;
    /** The table mapping access reads the objects through. */
    mapping?: TableMapping;
    /** The traits the objects take. */
    traits?: readonly TraitInstance[];
    /** The schema of one stack declaration, derived from the create input. */
    declaration?: schema.Schema;
    /** The references a declaration gives, by created field. */
    declaredReferences?: Readonly<Record<string, DeclaredReference>>;
    /** The fields a declaration sets, which a managed record's manager owns. */
    declaredFields?: ReadonlySet<string>;
}

/** The scope and identifier of an object's row, read from a row of any object table. */
export const ObjectKey = schema.looseObject({ scope: schema.string(), id: schema.string() });

/** The options an object type over another package's table takes none of. */
const FOREIGN_REFUSED = [
    "identity",
    "recoverable",
    "versioned",
    "controlled",
    "shareable",
    "suspendable",
    "fencing",
    "tracked",
    "attachments",
    "aggregates",
    "constraints",
    "predefined",
] as const;

/** The kinds of the methods that write only an object's access rows. */
export const ACCESS_KINDS: ReadonlySet<MethodKind> = new Set([
    "grant",
    "revoke",
    "invite",
    "accept",
    "withdraw",
]);

/** The kinds of the methods sharing an object's row through its relationships and invitations. */
const SHARING_KINDS: ReadonlySet<MethodKind> = new Set([
    ...ACCESS_KINDS,
    "relationships",
    "invitations",
    "explain",
]);

/** The columns every event has beside its keys: scope, identity, time and data. */
const EVENT_COLUMN_NAMES = ["scope", "id", "time", "data"] as const;

/** A column every event has beside its keys. */
type EventColumn = (typeof EVENT_COLUMN_NAMES)[number];

/** The columns every event has beside its keys, as a set. */
const EVENT_COLUMNS: ReadonlySet<string> = new Set(EVENT_COLUMN_NAMES);

/** The relation to an object's scope of any scope type, read as `through("scope", permission)`. */
export const SCOPE = "scope";

/** The permission on a scope's object that shows the scope. */
export const SCOPE_READ = "read";

/** The permission finding a scope without reading it, such as a guest's of a space. */
export const SCOPE_LIST = "list";

/** The permission to replicate the access rows of the scopes above a scope. */
export const REPLICATE = "replicate";

/** The permission to follow replicas on a principal's behalf, decided for the principal. */
export const FOLLOW = "follow";

/** Create an empty object inheriting from a prototype. */
const inherit: (prototype: object | null) => object = Object.create;

/** Read the prototype an object inherits from. */
const readPrototype: (value: object) => object | null = Object.getPrototypeOf;

/** A trait an object type takes, with the options its definition gives it. */
export interface TraitInstance {
    /** The trait. */
    readonly trait: Trait<unknown>;
    /** The trait's options from the definition. */
    readonly options: unknown;
}

/** A relation an object type declares, with object types as subjects. */
export interface ObjectRelationInput {
    /** The subject types the relation accepts. */
    readonly subjects: readonly (RelationInput["subjects"][number] | ObjectType)[];
    /** The permission granting the relation, the object's when absent, none when null. */
    readonly grantedBy?: string | null;
    /** Whether the relation's relationships show only to holders of the permission granting it. */
    readonly concealed?: true;
    /** Whether each object keeps at least one relationship of the relation, as its owners. */
    readonly isRequired?: true;
}

/** The scope objects live in: a scope type, a principal's policy, or the universe. */
export type ObjectScope = typeof Scope.universe.id | ObjectType | Policy;
/** The scope objects live in: a scope type, or the universe. */
export const ObjectScope = {
    /** Decide whether objects live in several scopes. */
    isList(scope: ObjectScope | readonly ObjectScope[]): scope is readonly ObjectScope[] {
        return Array.isArray(scope);
    },

    /** List the policies of the scope types objects live in, leaving out the universe. */
    policies(scope: ObjectScope | readonly ObjectScope[]): Policy[] {
        return [scope]
            .flat()
            .flatMap((type) =>
                type === Scope.universe.id ? [] : [type instanceof ObjectType ? type.policy : type],
            );
    },

    /** Decide whether two scope declarations share a scope type, by policy. */
    isBeside(
        scope: ObjectScope | readonly ObjectScope[],
        other: ObjectScope | readonly ObjectScope[],
    ): boolean {
        const keys = (declared: ObjectScope | readonly ObjectScope[]) =>
            ObjectScope.policies(declared).map((policy) =>
                ObjectTypeReference.key(policy.typeReference),
            );
        const others = keys(other);

        return keys(scope).some((key) => others.includes(key));
    },
};

/** The permission names an object type grants. */
export type PermissionOf<Type extends ObjectType> =
    Type extends ObjectType<infer Configuration> ? Configuration["permissions"] : never;

/** The prefix of an object type's identifiers, as other types' signatures refer to it. */
export type IdentityOf<Type> = Type extends { readonly identity: infer Identity extends string }
    ? Identity
    : never;

/** A scope in signatures: "universe", its scope type's identity, or any string for several. */
export type ScopeIdentityOf<Scope> = Scope extends readonly unknown[]
    ? string
    : Scope extends ObjectType
      ? IdentityOf<Scope>
      : Scope extends Policy<string, infer Kind>
        ? Kind
        : Scope;

/** The permissions of an object type: role-granted names, or relation-derived expressions. */
export type ObjectPermissions<Permissions extends string = string> =
    | readonly Permissions[]
    | Readonly<Record<Permissions, AccessExpression>>;
/** The permissions an object type declares. */
export const ObjectPermissions = {
    /** Decide whether permissions are declared as a list of names. */
    isList(permissions: ObjectPermissions): permissions is readonly string[] {
        return Array.isArray(permissions);
    },

    /** Read declared permissions as expressions by name, a listed permission granted by nothing. */
    expressions(permissions: ObjectPermissions): Readonly<Record<string, AccessExpression>> {
        return ObjectPermissions.isList(permissions)
            ? Object.fromEntries(permissions.map((name) => [name, none()]))
            : permissions;
    },
};

/** The traits an object opts into. */
export interface ObjectTraits<Permissions extends string = string> {
    /** The parent object type, or "self" for a tree. */
    readonly nested?: NestedDefinition;
    /** Deleted objects stay restorable for a window. */
    readonly recoverable?: RecoverableDefinition<Permissions>;
    /** The rules after which their store removes the objects. */
    readonly expiring?: readonly ExpiryRule[];
    /** The objects' natural key, by which callers and sources identify them in place of ids. */
    readonly key?: schema.Schema<string>;
    /** The scopes inside each object's scope that replica the objects, matching any condition. */
    readonly inherited?: { readonly where?: Condition };
    /** The objects are immutable, numbered versions of their parent. */
    readonly versioned?: VersionsDefinition;
    /** A controller reconciles the objects. */
    readonly controlled?: ControlledDefinition;
    /** Installations bind to the objects, becoming their consumers. */
    readonly bindable?: true | Gated<Permissions>;
    /** The objects are a kind's resources, which its providers provision. */
    readonly provisioned?: ProvisionedDefinition;
    /** The create input fields that refer to other records beyond its reference fields. */
    readonly references?: Readonly<Record<string, DeclaredReference>>;
    /** The create input fields a stack fills for each declaration, beside its identity. */
    readonly supplied?: readonly string[];
    /** Holders of the permission share the objects. */
    readonly shareable?: ShareableDefinition<Permissions>;
    /** Holders of the permission suspend and resume a scope object. */
    readonly suspendable?: Gated<Permissions>;
    /** Holders of the permission fence the objects' scope while it moves, and lift the fence. */
    readonly fencing?: Gated<Permissions>;
    /** Every change stays in the log, grouped into activities. */
    readonly tracked?: TrackedDefinition<Permissions>;
    /** Whether the audit also records reads. */
    readonly audited?: { readonly reads: true };
    /** How pickers, mentions and titles show the objects: title, subtitle, icon, accent, cover. */
    readonly presentable?: PresentableDefinition;
    /** How the objects are ordered among their siblings. */
    readonly orderable?: OrderableDefinition;
    /** The fields full-text search ranks, lists filter and lists order the objects by. */
    readonly search?: SearchDefinition;
}

/** The definition of an object type. */
export interface ObjectDefinition<
    Permissions extends string = string,
    Methods extends Readonly<Record<string, Method>> = {},
    Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope | readonly ObjectScope[],
    Granted extends string = Permissions,
> extends ObjectTraits<NoInfer<Granted>> {
    /** The singular name, unique within the declaring package. */
    readonly name: string;
    /** The prefix of the objects' identifiers, the name when absent. */
    readonly identity?: string;
    /** The plural name. */
    readonly plural: string;
    /** Where the objects live in another package's table, absent to derive the type's table. */
    readonly mapping?: Omit<TableMapping, "policy">;
    /** The scope types the objects live in, any scope when absent. */
    readonly scope?: Scope;
    /** The reference relation to each object's owner when it lives in another scope. */
    readonly owner?: string;
    /** Attributes permission expressions read. */
    readonly attributes?: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** Further relations stored in relationships. */
    readonly relations?: Readonly<Record<string, ObjectRelationInput>>;
    /** Permission names roles grant, or named expressions derived from relations. */
    readonly permissions?: ObjectPermissions<Permissions>;
    /** Permissions only their expressions grant, never roles. */
    readonly reserved?: readonly NoInfer<Granted>[];
    /** Sensitive permissions that apply only after the authentication each requires. */
    readonly elevated?: Readonly<Partial<Record<NoInfer<Granted>, Elevation>>>;
    /** Permissions available while the object's scope is suspended. */
    readonly administration?: readonly NoInfer<Granted>[];
    /** Permissions available while the object's scope moves to another machine. */
    readonly moving?: readonly NoInfer<Granted>[];
    /** An access type whose identity the objects take. */
    readonly represents?: Policy;
    /** Whether the objects are scopes containing other objects. */
    readonly isScope?: boolean;
    /** Whether each object is a partition, as only a scope type is. */
    readonly partition?: true;
    /** The operations callers may execute, keyed by method name. */
    readonly methods?: Methods;
    /** How often each method runs per key, by method name. */
    readonly limits?: Readonly<Record<string, MethodLimits>>;
    /** The fields each object has, for objects declared by fields. */
    readonly fields?: Readonly<Record<string, Field>>;
    /** The aggregates of these objects their holder maintains, by field name. */
    readonly aggregates?: Readonly<Record<string, ObjectAggregate>>;
    /** The attachments the objects take. */
    readonly attachments?: readonly Attachment[];
    /** Checks, indexes, unique constraints and foreign keys over the columns, by name. */
    readonly constraints?: Readonly<Record<string, ObjectConstraint>>;
    /** Unique indexes the key index has across databases, by name. */
    readonly indexes?: Readonly<Record<string, ObjectIndex>>;
    /** Where the objects live: a kind, or the event log with its settings. */
    readonly storage?: Exclude<ObjectStorage, "events"> | EventStorage;
    /** The event log readers the store has a slot of per row value, by slot name. */
    readonly readers?: Readonly<Record<string, ReaderDefinition>>;
    /** The previous names of renamed fields, by current field. */
    readonly moved?: { readonly fields?: Readonly<Record<string, string>> };
    /** The fields each release computes from stored rows and earlier inputs, by release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** The rows every database has identical, which no call writes. */
    readonly predefined?: readonly Row[];
}

/** An event log reader with one slot per distinct value of a row column. */
export interface ReaderDefinition {
    /** The column whose distinct values each have a separate slot. */
    readonly by: "id" | "scope";
    /** Where a new slot starts: the creating commit's log position, or the log's start. */
    readonly start: "latest" | "earliest";
    /** How long the log retains events past a slot's place once it advances. */
    readonly hold: Duration;
    /** How long a page's first event waits before a reader takes the page, none when absent. */
    readonly batch?: Duration;
}

/** A package's setting of days a scope places, as data the type's description stores. */
export interface RetentionSetting {
    /** The setting's package-local name. */
    readonly name: string;
    /** The scope type that places its values. */
    readonly scope: "user" | "space" | "device";
    /** The schema of its days. */
    readonly schema: schema.Schema<number>;
    /** The days where no placed value applies. */
    readonly default: number;
}

/** Where objects live: durable, ephemeral, external, local, or the event log. */
export type ObjectStorage = "durable" | "ephemeral" | "external" | "local" | "events";

/** The event log with a type's objects, with when its events flush and how long they stay. */
export interface EventStorage {
    /** The event log. */
    readonly kind: "events";
    /** When a scope's hot events flush into a segment: at an age of the oldest, or at a count. */
    readonly flush: { readonly maxAge: Duration; readonly maxRows: number };
    /** How long segments stay: a duration, or the package's setting of days each scope reads. */
    readonly retention: Duration | { readonly setting: RetentionSetting };
    /** Whether each event is recorded exactly once, or at most once outside the caller's work. */
    readonly delivery: Delivery;
    /** Whether segments are locked and chained for the retention, as audit trails are. */
    readonly isLocked?: boolean;
    /** The subject field of the person whose key seals personal values, such as an actor. */
    readonly subject?: string;
    /** The data entry recording a distribution of values, which quantile folds merge. */
    readonly distribution?: string;
}

/** The storage kind of a written storage: its name, or the kind of the settings it gives. */
export type StorageKind<Written> = Written extends { readonly kind: infer Kind } ? Kind : Written;

/** How a type's objects are declared, deleted, expired, versioned, tracked and converted. */
export interface ObjectLifecycle<Permissions extends string = string> {
    /** The schema of one declaration in a stack, when stacks may declare the objects. */
    readonly declarationSchema: schema.Schema | undefined;
    /** How deleted objects stay restorable, for recoverable objects. */
    readonly recoverable: RecoverableDefinition<Permissions> | undefined;
    /** When their store removes the objects, for expiring objects. */
    readonly expiring: readonly ExpiryRule[] | undefined;
    /** Whether the objects are numbered versions of their parent. */
    readonly versioned: VersionsDefinition | undefined;
    /** How tracked objects record their history. */
    readonly tracked: TrackedDefinition<Permissions> | undefined;
    /** The previous names of renamed fields, by current field. */
    readonly moved: Readonly<Record<string, string>>;
    /** The fields each release computes from stored rows and earlier callers' inputs. */
    readonly convert: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** A definition as an object type retains it, with identity, storage and field lists resolved. */
type ObjectTypeDefinition<Configuration extends ObjectConfiguration> = ObjectDefinition<
    Configuration["permissions"],
    Configuration["methods"]
> & {
    readonly name: Configuration["name"];
    readonly identity: Configuration["identity"];
    readonly plural: Configuration["plural"];
    readonly table: Configuration["table"];
    readonly methods: Configuration["methods"];
    readonly guarded: readonly string[];
    readonly written: readonly string[];
    readonly updated: readonly string[];
    readonly sensitive: readonly string[];
    readonly text: readonly string[];
    readonly source: ObjectDefinition;
    readonly events: Configuration["events"];
};

/** A definition as a type stores it: expanded by traits, lists present, built members apart. */
export type ObjectAssembly<Configuration extends ObjectConfiguration = ObjectConfiguration> =
    ObjectTypeDefinition<Configuration> & {
        readonly scope: ObjectScope | readonly ObjectScope[];
        readonly fields: Configuration["fields"];
        readonly aggregates: Readonly<Record<string, ObjectAggregate>>;
        readonly attachments: readonly Attachment[];
        readonly indexes: Readonly<Record<string, ObjectIndex>>;
    };

/** An object type's code: method handlers by name, its controller and its stack handler. */
export interface ObjectImplementation<Self extends ObjectType = ObjectType> {
    /** The handler of each method, by method name. */
    readonly methods?: {
        readonly [Name in keyof Self["methods"]]?: Handler<
            TypesOf<Self["methods"][Name], Self["table"]>
        >;
    };
    /** Reconcile or follow the objects with work waiting as the store's principal. */
    readonly controller?: ObjectController<Self>;
    /** How a stack's declarations of the objects become their managed records. */
    readonly declaration?: DeclarationHandler<Self>;
}

/** A unique index the key index has across databases. */
export interface ObjectIndex {
    /** The indexed fields, in key order. */
    readonly on: readonly string[];
    /** The scope type of the keys' uniqueness: the universe or an enclosing type, read lazily. */
    readonly across: () => ObjectType | typeof universe;
    /** The namespace shared with other types' same-named indexes, the type's name if absent. */
    readonly namespace?: string;
}

/** A method as callers call it: kind, permission, route, and input and output as JSON Schema. */
export interface MethodDescription {
    /** The method's kind. */
    readonly kind: MethodKind;
    /** The permission the caller needs. */
    readonly permission: string;
    /** Whether the method changes state. */
    readonly mutates: boolean;
    /** The HTTP method and path callers call the method at. */
    readonly route: { readonly method: string; readonly path: string };
    /** The input as JSON Schema, without the request a change replays under. */
    readonly input: JsonObject;
    /** The output as JSON Schema. */
    readonly output: JsonObject;
}

/** An object as readers see it: logged fields and texts, without guarded fields they lack. */
export type InstanceOf<Type extends ObjectType> =
    Type extends ObjectType<infer Configuration>
        ? Omit<RowImage<Configuration["table"]>, GuardedField<Configuration["fields"]>> &
              Partial<
                  Pick<
                      RowImage<Configuration["table"]>,
                      GuardedField<Configuration["fields"]> & keyof RowImage<Configuration["table"]>
                  >
              > & {
                  readonly [Name in TextFieldName<Configuration["fields"]>]: string;
              }
        : never;

/** An object type's signature: table, fields, methods, permissions, scope, parent, storage. */
export interface ObjectConfiguration {
    /** The singular name. */
    readonly name: string;
    /** The plural name, also the name of the relation from a parent or referenced type. */
    readonly plural: string;
    /** The prefix of the objects' identifiers. */
    readonly identity: string;
    /** The table with one record per object. */
    readonly table: Table;
    /** The create input fields that refer to other records beyond its reference fields. */
    readonly references: Readonly<Record<string, DeclaredReference>>;
    /** The create input fields a stack fills for each declaration, beside its identity. */
    readonly supplied: string;
    /** The permissions the type grants. */
    readonly permissions: string;
    /** The methods, by name. */
    readonly methods: Readonly<Record<string, Method>>;
    /** The objects' scope: "universe", its scope type's identity, or any string for several. */
    readonly scope: string;
    /** Where the objects live. */
    readonly storage: ObjectStorage;
    /** The event log's kind of an events type, undefined for every other storage. */
    readonly events: EventKind | undefined;
    /** The fields, by name, whose signatures say which are guarded, written, sensitive or text. */
    readonly fields: Readonly<Record<string, Field>>;
    /** The parent type's identity, "self" for a tree, "any" for attachments, undefined for none. */
    readonly parent: string | undefined;
    /** The reference field to the objects' owner, undefined for objects without an owner. */
    readonly owner: string | undefined;
    /** The identities of the attachment types the objects take, never for none. */
    readonly attachments: string;
}

/** An object type of a declared signature, each absent member at its widest. */
export type ObjectOf<Declared extends Partial<ObjectConfiguration>> = ObjectType<{
    readonly [Key in keyof ObjectConfiguration]: Declared extends {
        readonly [Name in Key]: infer Value extends ObjectConfiguration[Key];
    }
        ? Value
        : ObjectConfiguration[Key];
}>;

/** The brand of a call's object type, which exists in types alone. */
declare const CALLED_OBJECT: unique symbol;

/** A method call of an object type, typed by the type it calls and the result it answers. */
export type ObjectCall<Type extends ObjectType = ObjectType, Result = unknown> = Call<Result> & {
    /** The called object type, in types alone. */
    readonly [CALLED_OBJECT]?: Type;
};

/** A declared object type with its storage, permissions, methods and declaration schema. */
export class ObjectType<Configuration extends ObjectConfiguration = ObjectConfiguration> {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The assembled definition: expanded by traits, table and methods built, lists present. */
    readonly definition: ObjectAssembly<Configuration>;
    /** The tables a database with the object needs, access tables included. */
    readonly tables: readonly Table[];
    /** The executable operations by method name: the definition's, with their handlers. */
    readonly methods: Configuration["methods"];
    /** The object's relations and permissions, evaluated by access. */
    readonly policy: Policy;
    /** The permission names callers may have on the object. */
    readonly permissions: readonly string[];
    /** The controller reconciling pending objects, absent until implemented with one. */
    readonly controller?: ObjectController;
    /** How stack declarations become managed records, absent for types they cannot declare. */
    readonly declaration: DeclarationHandler | undefined;

    /** The scope types the objects live in, none for universe objects and objects in any scope. */
    readonly scopes: readonly ObjectType[];
    /** The parent of nested objects. */
    readonly parent:
        | {
              /** The owner object type, or "any" for attachments. */
              readonly object: ObjectType | "any";
              /** Whether an object may have no parent. */
              readonly optional: boolean;
              /** The parent permission a caller needs to create or move an object under it. */
              readonly receive: string;
              /** Delete objects with their parent, trash them with it, or refuse deleting it. */
              readonly delete: "cascade" | "trash" | "restrict";
          }
        | undefined;
    /** The roles the objects are shared through, if they are shared through roles. */
    readonly roles: RolesDefinition | undefined;
    /** How pickers, mentions and titles show the objects, absent for a type presenting none. */
    readonly presentation: Presentation | undefined;
    /** How the objects are ordered among their siblings, absent for a type ordering none. */
    readonly ordering: Ordering | undefined;
    /** The fields search ranks and lists filter and order the objects by, if any. */
    readonly search: ObjectSearch | undefined;
    /** The event log's kind of an events type, absent for every other storage. */
    /** The schema of the objects' identifiers: natural key, or generated typed identifiers. */
    readonly idSchema: schema.Schema<string>;
    /** The event log's kind of an events type, none for another storage. */
    private readonly events: Configuration["events"];
    /** The schemas, procedures and mapping derived from the definition, built lazily. */
    private derived: Derived = ObjectType.#underived();
    /** Whether the authored members and traits were validated, which copies inherit. */
    private isValid = false;

    /** Retain a definition with its table and methods, and assemble its policy. */
    constructor(owner: Package, definition: ObjectTypeDefinition<Configuration>) {
        // require a logged table, and store the definition with its lists present
        ObjectType.#requireLogged(definition.table);
        this.package = owner;
        this.definition = {
            ...definition,
            scope: definition.scope ?? [],
            fields: definition.fields ?? {},
            aggregates: definition.aggregates ?? {},
            attachments: definition.attachments ?? [],
            indexes: definition.indexes ?? {},
        };

        // retain the identifier schema and the methods, requiring the release's conversions
        this.idSchema = definition.key ?? schema.identifier(definition.identity);
        this.methods = definition.methods;
        ObjectType.#requireConversions(definition, owner);
        this.declaration = ObjectType.#declarationOf(definition);

        // compute the scopes, nesting, sharing, presentation, ordering, search and the event kind once
        this.scopes = ObjectType.#scopeTypes(this.definition.scope);
        this.parent = ObjectType.#nestedParent(this, definition);
        this.roles = Roles.of(definition);
        this.presentation = Presentable.of(definition);
        this.ordering = Orderable.of(definition);
        this.search =
            definition.search === undefined ? undefined : ObjectSearch.of(definition.search);

        // resolve the relations, add trait policies and register the policy
        const assembly = ObjectPolicy.policy(this, definition, owner);
        this.permissions = assembly.permissions;
        this.policy = assembly.policy;

        // take the event log's kind and list the tables
        this.events = definition.events;
        this.tables = this.#databaseTables();
    }

    /** The definition as written, its methods built, which descriptions encode. */
    get source(): ObjectDefinition {
        return this.definition.source;
    }

    /** The table with one record per object. */
    get table(): Configuration["table"] {
        return this.definition.table;
    }

    /** The singular name. */
    get name(): Configuration["name"] {
        return this.definition.name;
    }

    /** The prefix of the objects' identifiers. */
    get identity(): Configuration["identity"] {
        return this.definition.identity;
    }

    /** The plural name. */
    get plural(): Configuration["plural"] {
        return this.definition.plural;
    }

    /** The scope types the objects live in, none for any scope. */
    get scope(): ObjectScope | readonly ObjectScope[] {
        return this.definition.scope;
    }

    /** Whether the objects live in any scope, their definition declaring none. */
    get isAnyScope(): boolean {
        return [this.scope].flat().length === 0;
    }

    /** Whether the objects live in the universe: listed there, or living in any scope. */
    get isInUniverse(): boolean {
        return this.isAnyScope || [this.scope].flat().includes(Scope.universe.id);
    }

    /** Whether the objects live only in the universe, being no scopes themselves. */
    get isUniverseOnly(): boolean {
        return ObjectType.isUniverseOnly(this.definition);
    }

    /** Report whether a definition's objects live only in the universe, being no scopes. */
    static isUniverseOnly(
        definition: Pick<ObjectDefinition, "scope" | "isScope" | "represents">,
    ): boolean {
        return (
            definition.scope === Scope.universe.id &&
            definition.isScope !== true &&
            definition.represents?.definition.scope !== true
        );
    }

    /** The reference relation to each object's owner, absent if the scope is the owner. */
    get owner(): string | undefined {
        return this.definition.owner;
    }

    /** Whether each object is a partition: by its definition, or as the principal it stands for. */
    get isPartition(): boolean {
        const { partition, represents } = this.definition;

        return partition === true || represents?.definition.partition === true;
    }

    /** The principal kind each object stands for, absent for objects standing for none. */
    get represents(): Policy | undefined {
        return this.definition.represents;
    }

    /** Where the objects live. */
    get storage(): Configuration["storage"] {
        return ObjectType.storageKind(this.definition.storage);
    }

    /** The fields each object has, for objects declared by fields. */
    get fields(): Configuration["fields"] {
        return this.definition.fields;
    }

    /** The fields needing a separate read permission. */
    get guarded(): readonly string[] {
        return this.definition.guarded;
    }

    /** Report whether rows show a reader a guarded field with personal values, unconcealed. */
    reveals(rows: readonly Row[], hidden: readonly (readonly string[])[]): boolean {
        const personal = this.guarded.filter((name) => this.table[TABLE].personal.has(name));

        return zip(rows, hidden).some(([row, fields]) =>
            personal.some((name) => row[name] !== undefined && !fields.includes(name)),
        );
    }

    /** The fields callers write. */
    get written(): readonly string[] {
        return this.definition.written;
    }

    /** The fields callers write after creation, every written field but the immutable ones. */
    get updated(): readonly string[] {
        return this.definition.updated;
    }

    /** The columns with sensitive values. */
    get sensitive(): readonly string[] {
        return this.definition.sensitive;
    }

    /** The text fields, stored in chunks. */
    get text(): readonly string[] {
        return this.definition.text;
    }

    /** The aggregates of these objects their holder maintains, by field name. */
    get aggregates(): Readonly<Record<string, ObjectAggregate>> {
        return this.definition.aggregates;
    }

    /** The attachments the objects take. */
    get attachments(): readonly Attachment[] {
        return this.definition.attachments;
    }

    /** How often each method runs per key, by method name. */
    get limits(): Readonly<Record<string, MethodLimits>> {
        return this.definition.limits ?? {};
    }

    /** The unique indexes the key index has across databases, by name. */
    get indexes(): Readonly<Record<string, ObjectIndex>> {
        return this.definition.indexes;
    }

    /** The rows every database has identical, which no call writes. */
    get predefined(): readonly Row[] {
        return this.table[TABLE].predefined;
    }

    /** Refuse writing one of the type's predefined rows. */
    refusePredefined(id: string): void {
        if (Key.isPredefined(this.table, { id })) {
            throw ObjectError.of(
                "PREDEFINED",
                `${this.name} ${id} is predefined and cannot be written`,
            );
        }
    }

    /** The objects' natural key, absent for objects with generated identifiers. */
    get keyed(): schema.Schema<string> | undefined {
        return this.definition.key;
    }

    /** The rows the scopes inside each object's scope replica, for inherited objects. */
    get inherited(): { readonly where?: Condition } | undefined {
        return this.definition.inherited;
    }

    /** The kind whose resources the objects are, absent for other objects. */
    get provisioned(): ProvisionedDefinition | undefined {
        return this.definition.provisioned;
    }

    /** Whether reads of the objects are audited. */
    get isReadAudited(): boolean {
        return this.definition.audited?.reads === true;
    }

    /** The traits the objects take, with their options. */
    get traits(): readonly TraitInstance[] {
        const traits = this.derived.traits ?? ObjectType.#traitsOf(this.definition);
        this.derived.traits = traits;

        return traits;
    }

    /** How the objects are declared, deleted, expired, versioned, tracked and converted. */
    get lifecycle(): ObjectLifecycle<Configuration["permissions"]> {
        const { definition } = this;

        return {
            declarationSchema: this.declarationSchema(),
            recoverable: definition.recoverable,
            expiring: definition.expiring,
            versioned: definition.versioned,
            tracked: definition.tracked,
            moved: definition.moved?.fields ?? {},
            convert: definition.convert ?? {},
        };
    }

    /** The ancestor index of a tree of this object type. */
    get tree(): Tree | undefined {
        return this.table[TABLE].tree;
    }

    /** The name the type is served, routed and audited under: its name in camel case. */
    get key(): string {
        return Casing.camel(this.name);
    }

    /** Reference one object in its scope. */
    reference(scope: string, id: string): ObjectReference {
        return this.policy.reference(scope, id);
    }

    /** Read one of the type's declared event log readers by its slot name. */
    reader(name: string): EventReader {
        const declared = this.definition.readers?.[name];
        if (declared === undefined) {
            throw new TypeError(`object ${this.name} has no reader ${name}`);
        }

        return new EventReader(name, Duration.milliseconds(declared.hold));
    }

    /** Consume one of the type's readers by scope, handing each claimed page to a handler. */
    consume(reader: string, handler: ReaderHandler): (store: ObjectStore) => ReconcileController {
        return (store) => new Consumer(store, this, reader, handler);
    }

    /** Update the type's reader slots with a row the transaction inserts or removes. */
    updateReaders(
        transaction: Transaction,
        change: { readonly before: Row | null; readonly after: Row | null },
        now: number,
    ): void {
        for (const [name, declared] of Object.entries(this.definition.readers ?? {})) {
            // start or place an inserted row's slot as the transaction commits
            const { before, after } = change;
            if (before === null && after !== null) {
                const slot = this.reader(name).at(schema.string().parse(after[declared.by]));
                transaction.record((database) =>
                    declared.start === "latest"
                        ? slot.start(now, database)
                        : slot.place(now, database),
                );
            }
            // release a removed row's slot
            else if (before !== null && after === null && declared.by === "id") {
                const slot = this.reader(name).at(schema.string().parse(before[declared.by]));
                transaction.record((database) => slot.release(database));
            }
        }
    }

    /** Decide whether a value is an object type. */
    static is(value: unknown): value is ObjectType {
        return value instanceof ObjectType;
    }

    /** The package-qualified type of the objects, as access refers to it. */
    get typeReference(): ObjectTypeReference {
        return this.policy.typeReference;
    }

    /** Whether an identifier identifies one of the type's objects. */
    identifies(id: string): boolean {
        return this.idSchema.safeParse(id).success;
    }

    /** Generate an object's identifier from an environment, refusing a naturally keyed type. */
    generateId(environment: UuidEnvironment): Identifier<Configuration["identity"]> {
        this.requireGenerated();

        return Identifier.create(this.identity, environment);
    }

    /** Refuse an identifier for a type with a natural key. */
    private requireGenerated(): void {
        if (this.keyed !== undefined) {
            throw ObjectError.of("KEY_MISSING", `creating ${this.name} requires a key`);
        }
    }

    /** Derive a request object's identifier at a position, stable on repeat. */
    deriveId(requestId: string, position: number): Identifier<string> {
        this.requireGenerated();

        return Identifier.derive(this.identity, requestId, position);
    }

    /** Read an identifier of the type's identity, refusing a type with a natural key. */
    identifier(id: string): Identifier<Configuration["identity"]> {
        this.requireGenerated();

        return schema.identifier(this.identity).parse(id);
    }

    /** Decide whether another type shares this one's table. */
    same(other: ObjectType | "any" | undefined): boolean {
        return other instanceof ObjectType && other.table === this.table;
    }

    /** Decide whether the type copies a declared one, sharing its table, methods and policy. */
    isCopyOf<Type extends ObjectType>(declared: Type): this is Type {
        return this.table === declared.table;
    }

    /** A method's input conversions: renamed fields, the object's and the method's, in order. */
    conversions(name: string): Readonly<Record<Version, Readonly<Record<string, Expression>>>> {
        // rename previous field names in calls of releases before this one
        const renames = Object.fromEntries(
            Object.entries(this.lifecycle.moved).map(([field, previous]) => [
                field,
                Expression.column(previous),
            ]),
        );
        const releases: Record<string, Record<string, Expression>> = Object.keys(renames).length ===
        0
            ? {}
            : { [this.package.version]: renames };

        // merge the object's and the method's assignments by release
        const declared = ObjectType.#methodsOf(this)[name];
        for (const conversions of [this.lifecycle.convert, declared?.convert ?? {}]) {
            for (const [release, assignments] of Object.entries(conversions)) {
                releases[release] = { ...releases[release], ...assignments };
            }
        }

        return releases;
    }

    /** Read a declared method by name, refusing a name the type does not declare. */
    method(name: string): Method {
        const declared = Object.hasOwn(this.methods, name)
            ? ObjectType.#methodsOf(this)[name]
            : undefined;
        if (declared === undefined) {
            throw new TypeError(`object ${this.name} has no method ${name}`);
        }

        return declared;
    }

    /** Reference one of the object's declared permissions. */
    permission<Self extends ObjectType>(this: Self, name: PermissionOf<Self>): Permission {
        if (!this.permissions.includes(name)) {
            throw new TypeError(`object ${this.name} has no permission ${name}`);
        }

        return this.policy.permission(name);
    }

    /** Where the objects' routes place their scope. */
    get route(): ScopeRoute {
        return scopeRoute(this);
    }

    /** Read the scope a call acts in: its scope field's, else the universe. */
    readScope(name: string, input: JsonObject): string {
        // refuse a method the type lacks, which a call from elsewhere may ask for
        if (!Object.hasOwn(this.methods, name)) {
            throw ObjectError.of("METHOD_UNKNOWN", `object ${this.name} has no method ${name}`);
        }

        // read the scope field, or the universe
        const field = this.route.field;
        const scope = field === undefined ? Scope.universe.id : input[field];
        if (typeof scope !== "string") {
            throw ObjectError.of("FIELD_MISSING", `call has no ${field}`);
        }

        return scope;
    }

    /** The schemas of the objects' rows and of the inputs their methods take. */
    get schema(): ObjectSchema {
        this.derived.schema ??= objectSchema(this);

        return this.derived.schema;
    }

    /** Build a method's procedure: its route, input and output. */
    procedure(name: string): MethodProcedure {
        // build each procedure once
        const built =
            this.derived.procedure.get(name) ?? this.method(name).procedure(name, this.schema);
        this.derived.procedure.set(name, built);

        return built;
    }

    /** Read a method's input schema as clients call it, without a mutation's replay field. */
    input(name: string): InputSchema {
        // build each input once, which methods of traits share
        const built = this.derived.input.get(name);
        if (built !== undefined) {
            return built;
        }

        // leave out the field a pushed call replays under, except where a written field of its name replaces it
        const procedure = this.procedure(name).input;
        const replay = this.schema.replay;
        const input = this.method(name).mutates
            ? InputSchema.map(procedure, (fields) =>
                  schema.object(
                      Object.fromEntries(
                          Object.entries(fields.shape).filter(
                              ([field, value]) => replay[field] !== value,
                          ),
                      ),
                  ),
              )
            : procedure;
        this.derived.input.set(name, input);

        return input;
    }

    /** Describe the methods callers call, by name: kind, permission, route, input and output. */
    describeMethods(): Record<string, MethodDescription> {
        // describe each callable method by its procedure
        const described: Record<string, MethodDescription> = {};
        for (const [name, procedure] of Object.entries(callableProcedures(this))) {
            // read the method and the route and output of its procedure
            const declared = this.method(name);
            const { route, outputSchema } = procedure["~orpc"];
            const output = schema.instanceof(schema.Schema).parse(outputSchema);

            // describe its input as callers write it, and its output
            described[name] = {
                kind: declared.kind,
                permission: declared.permission,
                mutates: declared.mutates,
                route: {
                    method: present(route.method, `the route method of ${name}`),
                    path: present(route.path, `the route path of ${name}`),
                },
                input: JsonValue.object(toJsonSchema(this.input(name))),
                output: JsonValue.object(toJsonSchema(output)),
            };
        }

        return described;
    }

    /** Read the event log's kind of an events type. */
    eventKind(): NonNullable<Configuration["events"]> {
        return present<NonNullable<Configuration["events"]>>(
            this.events,
            `the event kind of ${this.name}, which is no events type`,
        );
    }

    /** The procedures of the object's methods. */
    get procedures(): ObjectProcedures<ObjectType<Configuration>> {
        return objectProcedures<ObjectType<Configuration>>(this);
    }

    /** The procedures every object type shares. */
    get shared(): { readonly store: StoreProcedures } {
        return { store: storeProcedures };
    }

    /** Derive the audit action `Noun.method` recording one method. */
    audit(name: string): AuditAction {
        // read the method's audit details
        const declared = ObjectType.#methodsOf(this)[name]?.audit;

        return {
            package: this.package,
            name: `${this.key}.${name}`,
            target: AuditTarget,
            details: declared === undefined ? schema.object({}) : declared.details.partial(),
        };
    }

    /** Build a call's audit action and values: its object if targeted, else the collection. */
    auditCall(name: string, input: JsonObject, scope: string) {
        // target one object
        const declared = ObjectType.#methodsOf(this)[name];
        if (declared !== undefined && declared.target !== "scope") {
            const id = schema.string().parse(input["id"]);
            const target = { type: this.name, id };

            return { action: this.audit(name), values: { target, details: {} } };
        }

        // target the scope's collection
        const target = { type: this.plural, id: scope };

        return { action: this.audit(name), values: { target, details: {} } };
    }

    /** Call one of the type's methods as a value, which a store or a call runs as its caller. */
    call<
        Self extends ObjectType,
        Name extends MethodName<Self>,
        const Input extends CallInput<Self, Name>,
    >(
        this: Self,
        name: Name,
        input: Input & ExtraFields<Input, CallInput<Self, Name>>,
    ): ObjectCall<Self, MethodOutput<Self, Name>> {
        return ObjectType.#record(this, name, input);
    }

    /** Convert a recorded call's input to the object's release, dropping undeclared fields. */
    upgrade(name: string, call: Call, shape?: object): JsonObject {
        // refuse a call of a later release
        const served = this.package.version;
        if (Version.compare(call.release, served) > 0) {
            throw ObjectError.of(
                "RELEASE_MISMATCH",
                `${Call.label(call)} targets release ${call.release} of ${this.name}, but ${this.name} is at release ${served}`,
            );
        }

        // take a call of this release as it is
        const conversions = this.conversions(name);
        if (Version.between(conversions, call.release, served).length === 0) {
            return call.input;
        }

        // assign each later release's fields and drop the fields this release no longer declares
        const converted = Expression.upgrade(conversions, call.input, call.release, served);

        return shape === undefined
            ? converted
            : Object.fromEntries(Object.entries(converted).filter(([field]) => field in shape));
    }

    /** Read a method's answer as its result, parsed by the method's output schema. */
    result(name: string, answer: unknown): unknown {
        // validate the answer synchronously by the method's output schema
        const label = `${this.name}.${name}`;
        const output = schema.standard().parse(this.procedure(name).output);
        const validated = output["~standard"].validate(answer);
        if (validated instanceof Promise) {
            throw new TypeError(`the output schema of ${label} is asynchronous`);
        } else if (validated.issues !== undefined) {
            throw ObjectError.of("OUTPUT_INVALID", `invalid output of ${label}`, {
                data: Issues.of(validated.issues),
            });
        }

        return validated.value;
    }

    /** Decide whether a call is of one of this type's methods. */
    isTypeOf(call: Pick<Call, "type">): boolean {
        return (
            this.typeReference.packageId === call.type.packageId &&
            this.typeReference.type === call.type.type
        );
    }

    /** Point a call at an object: its scope as the type's scope field, and its identifier. */
    target<Self extends ObjectType>(
        this: Self,
        row: { readonly scope: string; readonly id: string },
    ): TargetInput<Self> {
        return mapValues<Readonly<Record<string, string>>, TargetInput<Self>>(
            { ...this.in(row.scope), id: row.id },
            (value) => value,
        );
    }

    /** Place a call in a scope, as the type's scope field, for calls creating objects there. */
    in<Self extends ObjectType>(this: Self, scope: string): ScopeInput<Self> {
        const field = this.route.field;

        return mapValues<Readonly<Record<string, string>>, ScopeInput<Self>>(
            field === undefined ? {} : { [field]: scope },
            (value) => value,
        );
    }

    /** Write a query of the objects as data, as clients and shapes send it. */
    query<Self extends ObjectType>(
        this: Self,
        options: Omit<ObjectQuery, "object" | "package" | "permission"> & {
            readonly permission?: PermissionOf<Self>;
        } = {},
    ): ObjectQuery {
        return { object: this.name, package: this.package.id, ...options };
    }

    /** Follow a shape at one key per row of these objects in the follower's database. */
    follow<Self extends ObjectType, Parameters extends object>(
        this: Self,
        shape: Shape<Parameters>,
        at: Omit<RowFollow<Parameters, Self>, "shape" | "from">,
    ): RowFollow<Parameters, Self> {
        return { ...at, shape, from: this };
    }

    /** Compile a query or include of the objects. */
    compile(
        options: Omit<ObjectQuery, "object" | "package">,
        objects: readonly ObjectType[],
    ): Omit<Query, "scopes"> {
        return compile(this, options, objects);
    }

    /** Compile a scope's named object queries. */
    static queries(
        objects: readonly ObjectType[],
        queries: Readonly<Record<string, ObjectQuery>> | undefined,
        chain: readonly string[],
    ): Record<string, Query> {
        return compileQueries(objects, queries, chain);
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

    /** Require a matching aggregate field with readers that may list every measured row. */
    requireAggregate(measured: ObjectType, name: string, aggregate: ObjectAggregate): void {
        // require this type's field
        if (this.fields[name]?.aggregate !== aggregate.function) {
            throw new TypeError(
                `aggregate ${name} of ${measured.name} needs a ${aggregate.function} field on ${this.name}`,
            );
        }

        // require its readers to list every measured row: the field's guard, or the read methods' permissions
        const guard = this.fields[name].access?.read;
        const reads = Object.values(ObjectType.#methodsOf(this))
            .filter((declared) => declared.kind === "get" || declared.kind === "list")
            .map((declared) => declared.permission);
        const readers = new Set(guard === undefined ? reads : [guard]);
        const listing = measured.listing?.name;
        const link = aggregate.via ?? "parent";
        const permissions = measured.policy.definition.permissions;
        const uncovered = [...readers].filter(
            (reader) =>
                listing === undefined ||
                !ObjectType.#covers(permissions, listing, link, reader, new Set()),
        );
        if (uncovered.length > 0) {
            throw new TypeError(
                `aggregate ${name} of ${measured.name} counts rows that readers of ${this.name} may not list: list them through ${link} ${uncovered.join(", ")}`,
            );
        }
    }

    /** Match the objects living in one scope, as SQL. */
    inScope(scope: string): SQL {
        return eq(this.table[TABLE].column("scope"), scope);
    }

    /** Match the objects in some scopes, skipping scopes the type never lives in, as SQL. */
    inScopes(scopes: readonly string[]): SQL {
        const column = this.table[TABLE].column("scope");
        const possible = scopes.filter(
            (scope) => column.definition.schema.safeParse(scope).success,
        );

        return possible.length === 0 ? sql`false` : inArray(column, possible);
    }

    /** Match the objects with some identifiers, as SQL. */
    withIds(ids: readonly string[]): SQL {
        return inArray(this.table[TABLE].column("id"), [...ids]);
    }

    /** Take these objects as attachments of a host. */
    attach(options: {
        readonly by: string;
    }): Attachment<ObjectType & { readonly identity: Configuration["identity"] }> {
        if (this.parent?.object !== "any") {
            throw new TypeError(`object ${this.name} is not nested in any parent`);
        }

        return { object: this, by: options.by };
    }

    /** The object types of plain reference relations, read by the type's permissions. */
    get related(): readonly ObjectType[] {
        return Object.entries(this.fields).flatMap(([property, field]) => {
            const name = Casing.kebab(field.relationAt(property));
            const isRelation = Object.hasOwn(this.policy.definition.relations, name);

            return isRelation && field.type === "reference" && field.target ? [field.target()] : [];
        });
    }

    /** The object types the type refers to: reference targets and relation subject types. */
    get referenced(): readonly ObjectType[] {
        // take the targets of reference fields and the types polymorphic fields accept
        const fields = Object.values(this.fields).flatMap((field) => {
            const { source, target } = field;
            const accepted =
                source.builder === "object" || source.builder === "subject" ? source.arguments : [];

            return [
                ...(target === undefined ? [] : [target()]),
                ...accepted.filter((type): type is ObjectType => type instanceof ObjectType),
            ];
        });

        // add the types the declared relations accept as subjects
        const relations = Object.values(this.definition.relations ?? {}).flatMap((relation) =>
            relation.subjects.filter((type): type is ObjectType => type instanceof ObjectType),
        );

        return [...fields, ...relations].filter((type) => !type.same(this));
    }

    /** The scope types enclosing each object, nearest first, for objects in one scope type. */
    get ancestors(): readonly ObjectType[] {
        const ancestors: ObjectType[] = [];
        for (
            let scope: ObjectScope | readonly ObjectScope[] = this.scope;
            scope instanceof ObjectType;
            scope = scope.scope
        ) {
            ancestors.push(scope);
        }

        return ancestors;
    }

    /** Read the scopes a query reads from a scope chain, nearest first. */
    readScopes(chain: readonly string[]): string[] {
        const held = this.table[TABLE].column("scope").definition.schema;

        return this.inherited === undefined
            ? [nth(chain, 0)]
            : chain.filter((scope) => held.safeParse(scope).success);
    }

    /** Decide whether the objects live in a scope. */
    livesIn(scope: ObjectReference): boolean {
        // read the policies of the scope types and principals the objects live in
        const policies = ObjectScope.policies(this.scope);

        return (
            this.scope !== Scope.universe.id &&
            (policies.length === 0 || policies.some((policy) => policy.is(scope)))
        );
    }

    /** Decide whether objects of another type attach to these objects. */
    attaches(child: ObjectType): boolean {
        return (
            this.attachments.some((attachment) => child.same(attachment.object)) ||
            (child.table === chunk && this.text.length > 0)
        );
    }

    /** Read a unique key duplicate as a conflict listing its fields, else pass on the failure. */
    conflict(error: unknown): unknown {
        // read the violated key of the type's table
        const definition = this.table[TABLE];
        const key = DatabaseError.is(error, "DUPLICATE") ? error.data : null;
        if (key === null || key.table !== definition.name) {
            return error;
        }

        // map the columns of each relation and reference to its name
        const { relations, references = {} } = this.mapping;
        const named = new Map<string, string>();
        for (const [name, relation] of Object.entries(relations)) {
            for (const column of [
                relation.column,
                relation.subject?.packageId,
                relation.subject?.type,
            ]) {
                if (column !== undefined) {
                    named.set(column, name);
                }
            }
        }
        for (const [name, reference] of Object.entries(references)) {
            for (const column of [reference.packageId, reference.type, reference.id]) {
                named.set(column, name);
            }
        }
        for (const declared of Object.values(this.fields)) {
            if (declared.reference !== "none" && declared.relation !== undefined) {
                const { packageId, type, scope, id } = Field.columnsOf(declared.relation);
                for (const column of [packageId, type, scope, id]) {
                    named.set(column, declared.relation);
                }
            }
        }

        // list each key column once, by its relation or its field
        const names = key.columns.map((field) => named.get(field) ?? field);
        const listed = new Intl.ListFormat("en").format([...new Set(names)]);

        return ObjectError.of("DUPLICATE", `another ${this.name} has the same ${listed}`, {
            cause: error,
        });
    }

    /** The table mapping access reads the objects through, built on its first read. */
    get mapping(): TableMapping {
        const mapping = this.derived.mapping ?? ObjectPolicy.mapping(this);
        this.derived.mapping = mapping;

        return mapping;
    }

    /** The create input fields a stack supplies each declaration. */
    get supplied(): readonly string[] {
        const { field } = this.route;
        const { ownerColumn } = this;

        return [
            ...(field === undefined ? [] : [field]),
            "id",
            ...(ownerColumn === undefined ? [] : [ownerColumn]),
            ...(this.definition.supplied ?? []),
        ];
    }

    /** Find the type of a call's method among some types. */
    static find(types: readonly ObjectType[], call: Pick<Call, "type">): ObjectType | undefined {
        return types.find((type) => type.isTypeOf(call));
    }

    /** Resolve a recorded call against the served types, converting input to the served release. */
    static resolve(
        objects: readonly ObjectType[],
        entry: Call,
        mutates: boolean,
        site: CallSite,
    ): {
        readonly object: ObjectType;
        readonly name: string;
        readonly input: JsonObject;
    } {
        // find the object type and a method the calling code may call
        const object = ObjectType.find(objects, entry);
        const name = entry.method;
        const method = object?.methods?.[name];
        if (
            object === undefined ||
            method === undefined ||
            !CallSite.covers(site, object, method) ||
            (mutates && !method.mutates)
        ) {
            throw ObjectError.of(
                "METHOD_UNKNOWN",
                `unknown ${mutates ? "mutating " : ""}method ${Call.label(entry)}`,
            );
        } else if (!mutates && method.mutates) {
            throw ObjectError.of(
                "PUSH_REQUIRED",
                `${Call.label(entry)} changes objects: push it from a client`,
            );
        }

        // convert the input to the served release
        const fields = InputSchema.fields(object.input(name)).shape;

        return { object, name, input: object.upgrade(name, entry, fields) };
    }

    /** Write a call of a method as data, against the release of the type's package. */
    static #record<Result>(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): Call<Result> {
        return {
            type: object.typeReference,
            method: name,
            input: schema.jsonObject().parse(input),
            release: object.package.version,
        };
    }

    /** Read how stacks declare a type: a durable type with a create method, by its create input. */
    static #declarationOf(
        definition: ObjectDefinition<string, Readonly<Record<string, Method>>>,
    ): DeclarationHandler | undefined {
        return ObjectType.declares(definition) ? {} : undefined;
    }

    /** Report whether stacks can declare a definition's objects: durable ones with create. */
    static declares(
        definition: ObjectDefinition<string, Readonly<Record<string, Method>>>,
    ): boolean {
        return (
            definition.methods?.["create"] !== undefined &&
            ObjectType.storageKind(definition.storage) === "durable"
        );
    }

    /** Read the schema of one declaration: the create input with its references by name. */
    private declarationSchema(): schema.Schema | undefined {
        // declare nothing of a type without a declaration
        if (this.declaration === undefined) {
            return undefined;
        }

        // derive the declaration from each create input variant once, without the name and the supplied fields
        const { supplied, declaredReferences: references } = this;
        const derived =
            this.derived.declaration ??
            InputSchema.map(this.input("create"), (created) =>
                schema.object(
                    Object.fromEntries(
                        Object.entries(created.shape)
                            .filter(([field]) => !supplied.includes(field))
                            .map(([field, written]): [string, schema.Schema<JsonValue>] => {
                                const reference = references[field];

                                // let the declaration's name fill a name it leaves out
                                return field === "name"
                                    ? [field, written.exactOptional()]
                                    : reference === undefined
                                      ? [field, written]
                                      : [
                                            ObjectType.declaredField(field),
                                            ObjectType.#declaredReference(reference, written),
                                        ];
                            }),
                    ),
                ),
            );
        this.derived.declaration = derived;

        return derived;
    }

    /** Name a reference field as a declaration writes it, without `Id`: `tagId` as `tag`. */
    static declaredField(field: string): string {
        return field.length > 2 && field.endsWith("Id") ? field.slice(0, -2) : field;
    }

    /** Read a declared reference field's schema, optional for objects and optional fields. */
    static #declaredReference(
        reference: DeclaredReference,
        written: schema.Schema,
    ): schema.Schema<JsonValue> {
        // write one type's records by name or identifier, any object as declared, and a subject by its forms
        const declared =
            reference === "object"
                ? StackObjectReference
                : reference === "subject"
                  ? StackSubject
                  : StackReference.of(reference.identity);
        const isOptional = reference === "object" || schema.isOptional(written);

        return isOptional ? declared.exactOptional() : declared;
    }

    /** The create fields a declaration refers to other records by: reference and listed fields. */
    get declaredReferences(): Readonly<Record<string, DeclaredReference>> {
        // derive them once
        if (this.derived.declaredReferences !== undefined) {
            return this.derived.declaredReferences;
        }

        // refer by each object reference and subject field a declaration gives, keyed as created
        const created = InputSchema.fields(this.input("create")).shape;
        const supplied = new Set(this.supplied);
        const fields = Object.entries(this.definition.fields).flatMap(
            ([name, field]): [string, DeclaredReference][] => {
                const target = field.target?.();
                if (!(name in created) || supplied.has(name)) {
                    return [];
                } else if (target !== undefined) {
                    return [[name, target]];
                } else if (field.principals !== undefined) {
                    return [[name, "subject"]];
                }

                return [];
            },
        );

        // refer to the parent, and by the listed fields
        const parent = this.parent?.object;
        const parentId: [string, DeclaredReference][] =
            parent === undefined || parent === "any" ? [] : [["parentId", parent]];

        // cache them for the type's later reads
        const derived = Object.fromEntries([
            ...fields,
            ...parentId,
            ...Object.entries(this.definition.references ?? {}),
        ]);
        this.derived.declaredReferences = derived;

        return derived;
    }

    /** The column of the reference to the objects' owner, absent for objects without an owner. */
    get ownerColumn(): string | undefined {
        const { owner } = this;
        if (owner === undefined) {
            return undefined;
        }

        return present(this.mapping.relations[owner], `the owner of ${this.name}`).column;
    }

    /** The permission listing the objects needs, absent without a list method. */
    get listing(): Permission | undefined {
        return ObjectType.#methodPermission(this, "list");
    }

    /** The permissions roles may grant: every one but the reserved ones. */
    get grantable(): readonly string[] {
        const reserved = this.policy.definition.reserved ?? [];

        return this.permissions.filter((name) => !reserved.includes(name));
    }

    /** The permission reading one object needs, the listing's without a get method, or none. */
    get reading(): Permission | undefined {
        return ObjectType.#methodPermission(this, "get") ?? this.listing;
    }

    /** Whether a controller reconciles the objects to their desired generation. */
    get isControlled(): boolean {
        return this.traits.some((applied) => applied.trait === controlled);
    }

    /** Decide whether the type stores records: its record trait, outside the event log. */
    isRecord(): this is ObjectOf<{ table: ObjectTable }> {
        return this.storage !== "events" && this.traits.some((applied) => applied.trait === record);
    }

    /** Decide whether the type deletes to a trash, by its recoverable or controlled trait. */
    isTrashable(): this is ObjectOf<{
        table: TraitTable<{ readonly recoverable: RecoverableDefinition }>;
    }> {
        return this.lifecycle.recoverable !== undefined || this.isControlled;
    }

    /** Decide whether the type retains purged records, as its recoverable trait declares. */
    isRetained(): this is ObjectOf<{
        table: TraitTable<{
            readonly recoverable: RecoverableDefinition & { readonly retain: "record" };
        }>;
    }> {
        return this.lifecycle.recoverable?.retain === "record";
    }

    /** List the fields a declaration sets, its create input's, which its manager owns. */
    declaredFields(): ReadonlySet<string> {
        this.derived.declaredFields ??= new Set(
            Object.keys(InputSchema.fields(this.input("create")).shape).filter(
                (field) => !this.supplied.includes(field),
            ),
        );

        return this.derived.declaredFields;
    }

    /** The collection declaring the type's records: its declaration's, else its plural. */
    get collection(): string {
        return this.declaration?.collection ?? this.plural;
    }

    /** Report whether stacks can declare the type's records, which then have manager columns. */
    isDeclarable(): this is ObjectOf<{ table: ManagedTable }> {
        return (
            this.declaration !== undefined && "managerInstallationId" in this.table[TABLE].columns
        );
    }

    /** Implement the type: attach method handlers, its controller and its stack handler. */
    implement<Self extends ObjectType>(
        this: Self,
        implementation: ObjectImplementation<Self> &
            (Self["table"] extends ManagedTable ? unknown : { readonly declaration?: never }),
    ): Self;
    /**
     * Implement the type, the replica sharing its table, methods and policy, adding handlers.
     *
     * NOTE #Architecture: the one stated type of handler maps, as oRPC's router implementation.
     */
    implement(this: ObjectType, implementation: ObjectImplementation): ObjectType {
        // wrap each given method's handler
        const methods: Record<string, Method> = { ...this.methods };
        for (const [name, handler] of Object.entries(implementation.methods ?? {})) {
            const declared = methods[name];
            if (declared === undefined) {
                throw new TypeError(`object ${this.name} has no method ${name}`);
            } else if (handler !== undefined) {
                methods[name] = Method.handled(declared, handler);
            }
        }

        // leave ephemeral and external objects off the controllers
        const { controller, declaration } = implementation;
        if (controller !== undefined && this.storage !== "durable") {
            throw new TypeError(`${this.storage} object ${this.name} cannot have a controller`);
        }

        // replicate the type with its handlers, sharing its procedures
        return ObjectType.#copy(
            this,
            {
                methods,
                ...(controller === undefined ? {} : { controller }),
                ...(declaration === undefined ? {} : { declaration }),
            },
            this.derived,
        );
    }

    /** The phases implementations attach to the type's methods, by method name. */
    get hooks(): Readonly<Record<string, readonly Phase[]>> {
        return Object.fromEntries(
            Object.entries(this.methods).flatMap(([name, declared]) => {
                const phases = Method.phases(declared);

                return phases.length === 0 ? [] : [[name, phases]];
            }),
        );
    }

    /** Require valid members and traits once, as describing or serving the type does. */
    requireValid(): void {
        if (!this.isValid) {
            this.#validate(this.definition);
            this.isValid = true;
        }
    }

    /** Replicate the object type with some members changed, sharing its table. */
    with<Self extends ObjectType>(
        this: Self,
        changes: Partial<Pick<Self, "methods" | "controller" | "declaration" | "definition">>,
    ): Self {
        // replicate the members, building the changed type's schemas and procedures afresh
        return ObjectType.#copy(this, changes, ObjectType.#underived());
    }

    /** Copy an object type with some members changed and the given derived values. */
    static #copy<Type extends ObjectType>(
        type: Type,
        changes: Partial<Pick<Type, "methods" | "controller" | "declaration" | "definition">>,
        derived: Derived,
    ): Type {
        // validate the authored type before a copy changes its definition
        type.requireValid();

        return Object.assign(inherit(readPrototype(type)), type, changes, { derived });
    }

    /** Assemble an object type from its definition. */
    static assemble(
        owner: Package,
        definition: ObjectDefinition & {
            readonly table: Table;
            readonly methods: Readonly<Record<string, Method>>;
            readonly source: ObjectDefinition;
        },
    ): ObjectType {
        // list the fields by what callers may do with them
        const fields = Object.entries(definition.fields ?? {});
        const written = fields
            .filter(([, declared]) => declared.written !== false)
            .map(([name]) => name);
        const updated = fields
            .filter(([, declared]) => declared.written === true)
            .map(([name]) => name);

        // refuse an update of a field written only at creation
        const changing = Object.entries(definition.methods).find(
            ([, declared]) =>
                (declared.kind === "update" || declared.kind === "updateMany") &&
                (declared.fields ?? []).some(
                    (name) => definition.fields?.[name]?.written === "create",
                ),
        );
        if (changing !== undefined) {
            throw new TypeError(
                `object ${definition.name} updates an immutable field in method ${changing[0]}`,
            );
        }
        const guarded = fields
            .filter(([, declared]) => declared.access?.read !== undefined)
            .map(([name]) => name);
        const texts = fields
            .filter(([, declared]) => declared.type === "text")
            .map(([name]) => name);

        // list the sensitive columns
        const sensitive = Object.entries(definition.table[TABLE].columns)
            .filter(([, column]) => column.definition.classification === "sensitive")
            .map(([name]) => name);

        return new ObjectType(owner, {
            ...definition,
            identity: definition.identity ?? definition.name,
            written,
            updated,
            guarded,
            sensitive,
            text: texts,
            events: ObjectType.#eventKind(owner, definition),
        });
    }

    /** List object types with the chunk type their text fields need. */
    static served(objects: readonly ObjectType[]): ObjectType[] {
        const owners = objects.filter((object) => object.text.length > 0);

        return owners.length === 0 ? [...objects] : [...objects, Chunk.object(owners)];
    }

    /** List the types some types refer to or live in, transitively, leaving out the given ones. */
    static referenced(objects: readonly ObjectType[]): ObjectType[] {
        // walk the references and enclosing scope types of each type found
        const found: ObjectType[] = [];
        const isKnown = (type: ObjectType) =>
            objects.some((each) => each.same(type)) || found.some((each) => each.same(type));
        const pending = objects.flatMap((type) => [...type.referenced, ...type.ancestors]);
        for (let type = pending.shift(); type !== undefined; type = pending.shift()) {
            if (!isKnown(type)) {
                found.push(type);
                pending.push(...type.referenced, ...type.ancestors);
            }
        }

        return found;
    }

    /** Start a type's derived values empty. */
    static #underived(): Derived {
        return { procedure: new Map(), input: new Map() };
    }

    /** Require an object's table to be logged. */
    static #requireLogged(table: Table): void {
        if (table[TABLE].retention === "none") {
            throw new TypeError(`object table is not logged: ${table[TABLE].name}`);
        }
    }

    /** List the object types among a definition's scopes, leaving out the universe and policies. */
    static #scopeTypes(scope: ObjectScope | readonly ObjectScope[]): ObjectType[] {
        const scopes: readonly ObjectScope[] = [scope].flat();

        return scopes.filter((type) => type instanceof ObjectType);
    }

    /** List the tables the objects need: the store's or the event log's, and the type's. */
    #databaseTables(): readonly Table[] {
        // store events in the event log's tables, and ephemeral and external rows only in their table
        if (this.events !== undefined) {
            return eventTables([this.events]);
        } else if (this.storage !== "durable") {
            return [this.table];
        }

        return [this.table, ...SYSTEM_TABLES, ...(this.text.length === 0 ? [] : [chunk, chunkRun])];
    }

    /** Require valid methods, fields, aggregates, scopes, indexes and traits. */
    #validate(definition: ObjectTypeDefinition<Configuration>): void {
        // require the type's members
        this.#requireMethods();
        this.#requireAggregates();
        this.#requireScopes();
        this.#requireIndexes();
        this.#requireOwner();
        this.#requireStorage(definition, this.traits);
        this.#requireReaders();
        this.#requireRowSharing();
        this.#requireRoutes();

        // let each trait validate its options
        for (const { trait, options } of this.traits) {
            trait.validate?.(options, this, definition);
        }
    }

    /** Refuse two public methods answering one route, which a router would send to one of them. */
    #requireRoutes(): void {
        const routes = new Map<string, string>();
        for (const [name, declared] of Object.entries(this.methods)) {
            // leave off the methods clients cannot call, which no route answers
            if (!Method.isRouted(declared)) {
                continue;
            }

            // refuse a method and path another method answers already
            const { route } = this.procedure(name);
            const key = `${route.method} ${route.path}`;
            const earlier = routes.get(key);
            if (earlier !== undefined) {
                throw new TypeError(
                    `object ${this.name} methods ${earlier} and ${name} both handle ${key}`,
                );
            }
            routes.set(key, name);
        }
    }

    /** Refuse sharing methods on a partitioned scope, whose access lives in its partition. */
    #requireRowSharing(): void {
        const isSharing = Object.values(this.methods).some((declared) =>
            SHARING_KINDS.has(declared.kind),
        );
        if (this.isPartition && isSharing) {
            throw new TypeError(
                `object ${this.name} must share a partitioned scope through its relationships`,
            );
        }
    }

    /** Require readers by scope to start at the log's start, and only them to batch. */
    #requireReaders(): void {
        // refuse a reader per scope starting at the latest commit, and a batch of a reader per row
        const readers = Object.entries(this.definition.readers ?? {});
        const restarting = readers.find(
            ([, declared]) => declared.by === "scope" && declared.start === "latest",
        );
        const batching = readers.find(
            ([, declared]) => declared.by === "id" && declared.batch !== undefined,
        );
        if (restarting !== undefined) {
            throw new TypeError(
                `reader ${restarting[0]} of ${this.name} starts each scope once: start it at the earliest`,
            );
        } else if (batching !== undefined) {
            throw new TypeError(
                `reader ${batching[0]} of ${this.name} reads each row, so it cannot batch pages: only readers by scope can`,
            );
        }
    }

    /** Require each conversion up to the release, and moved fields clear of current fields. */
    static #requireConversions(
        definition: ObjectDefinition<string, Readonly<Record<string, Method>>>,
        owner: Package,
    ): void {
        // require each method's conversions up to the release
        for (const [name, declared] of Object.entries(definition.methods ?? {})) {
            Version.requireUpTo(
                declared.convert ?? {},
                owner.version,
                `${definition.name}.${name}`,
            );
        }

        // require moved fields to refer to previous fields only
        for (const [field, previous] of Object.entries(definition.moved?.fields ?? {})) {
            if (previous in (definition.fields ?? {})) {
                throw new TypeError(
                    `field ${definition.name}.${field} cannot move from ${previous}: it is a current field`,
                );
            }
        }
        Version.requireUpTo(definition.convert ?? {}, owner.version, definition.name);
    }

    /** Require client method names functions lack, and declared permissions. */
    #requireMethods(): void {
        // refuse client method names functions have
        const shadowed = Object.entries(ObjectType.#methodsOf(this)).find(
            ([name, declared]) =>
                Method.isRouted(declared) &&
                Object.getOwnPropertyNames(Function.prototype).includes(name),
        )?.[0];
        if (shadowed !== undefined) {
            throw new TypeError(
                `object ${this.name} has method ${shadowed}, which is already a function property`,
            );
        }

        // require a method for each limit
        const methods = ObjectType.#methodsOf(this);
        for (const limited of Object.keys(this.limits)) {
            if (!Object.hasOwn(methods, limited)) {
                throw new TypeError(
                    `object ${this.name} limits method ${limited}, which it does not have`,
                );
            }
        }

        // require declared permissions and valid methods
        for (const declared of Object.values(methods)) {
            if (!this.permissions.includes(declared.permission)) {
                throw new TypeError(`object ${this.name} has no permission ${declared.permission}`);
            }
            declared.validate?.(this);
        }

        // require the permissions guarding fields
        for (const [name, declared] of Object.entries(this.fields)) {
            for (const permission of [declared.access?.read, declared.access?.write]) {
                if (permission !== undefined && !this.permissions.includes(permission)) {
                    throw new TypeError(
                        `object ${this.name} field ${name} has no permission ${permission}`,
                    );
                }
            }
        }
    }

    /** Require each aggregate's holder field, and method inputs without aggregate fields. */
    #requireAggregates(): void {
        // require each aggregate's holder field
        for (const [name, aggregate] of Object.entries(this.aggregates)) {
            const holder =
                aggregate.via === undefined
                    ? this.parent?.object
                    : this.fields[aggregate.via]?.target?.();
            if (holder === undefined) {
                throw new TypeError(
                    `aggregate ${name} of ${this.name} needs a ${aggregate.function} field on its holder`,
                );
            } else if (holder !== "any") {
                holder.requireAggregate(this, name, aggregate);
            }
        }

        // refuse aggregate fields in method inputs
        for (const [name, declared] of Object.entries(ObjectType.#methodsOf(this))) {
            const { input } = declared;
            const variants = input instanceof schema.DiscriminatedUnion ? input.options : [input];
            const written = variants.flatMap((variant) =>
                variant instanceof schema.Object ? Object.keys(variant.shape) : [],
            );
            const aggregated = written.find((field) => this.fields[field]?.aggregate !== undefined);
            if (aggregated !== undefined) {
                throw new TypeError(
                    `method ${name} of ${this.name} cannot write aggregate field ${aggregated}`,
                );
            }
        }
    }

    /** Require readable scope types and valid local object scopes. */
    #requireScopes(): void {
        // refuse a containment grant on a type that lives only in the universe, without principals
        const contained = Object.entries(this.policy.definition.permissions).find(
            ([, expression]) => ObjectType.#grantsContainment(expression),
        );
        if (this.isUniverseOnly && contained !== undefined) {
            throw new TypeError(
                `object ${this.name} is in the universe, which has no principals: contained() in its ${contained[0]} permission allows no one`,
            );
        }

        // require each scope type to be a readable scope
        for (const scope of this.storage === "local" ? [] : this.scopes) {
            // require a scope type
            if (scope.policy.definition.scope !== true) {
                throw new TypeError(
                    `object ${this.name} is in ${scope.name}, which is not a scope`,
                );
            }
            // refuse a scope without the read permission
            else if (!scope.permissions.includes(SCOPE_READ)) {
                throw new TypeError(
                    `object ${this.name} is in ${scope.name}, which has no ${SCOPE_READ} permission`,
                );
            }
        }
    }

    /** Require each index over logged fields, unique within an enclosing scope or the universe. */
    #requireIndexes(): void {
        const { logged } = this.table[TABLE];
        for (const [name, declared] of Object.entries(this.indexes)) {
            // find a missing or unlogged field beside the identifier, and the scope the keys are unique within
            const across = declared.across();
            const missing = declared.on.find(
                (field) => field !== "id" && !Object.hasOwn(this.fields, field),
            );
            const unlogged = declared.on.find((field) => !Object.hasOwn(logged, field));
            const isEnclosing =
                across instanceof ObjectType
                    ? this.ancestors.some((ancestor) => ancestor.same(across))
                    : across === universe;

            // refuse each defect
            if (missing !== undefined) {
                throw new TypeError(`index ${name} of ${this.name} has no field ${missing}`);
            } else if (unlogged !== undefined) {
                throw new TypeError(`index ${name} of ${this.name} has unlogged field ${unlogged}`);
            } else if (!isEnclosing) {
                throw new TypeError(
                    `index ${name} of ${this.name} must be unique within the universe or a scope type enclosing its objects`,
                );
            }
        }
    }

    /** Require an owner to be the relation of a plain reference field. */
    #requireOwner(): void {
        const { owner } = this;
        const field = Object.entries(this.fields).find(
            ([property, each]) => Casing.kebab(each.relationAt(property)) === owner,
        )?.[1];
        if (owner !== undefined && (field?.type !== "reference" || field.target === undefined)) {
            throw new TypeError(
                `object ${this.name} takes its owner from ${owner}, but no reference relates it`,
            );
        }
    }

    /** Read the kind of a written storage, durable when absent. */
    static storageKind(written: ObjectDefinition["storage"]): ObjectStorage {
        return typeof written === "object" ? written.kind : (written ?? "durable");
    }

    /** Build the event log's kind of an events type, none for another storage. */
    static #eventKind(
        owner: Package,
        definition: Pick<ObjectDefinition, "name" | "storage" | "fields"> & {
            readonly table: Table;
        },
    ): EventKind | undefined {
        // derive no kind for another storage, and require the event columns
        const { name, table, fields = {} } = definition;
        const written = definition.storage;
        if (typeof written !== "object") {
            return undefined;
        } else if (!EventTable.is(table)) {
            throw new TypeError(`events object ${name} is missing the event columns`);
        }

        // read the keys from the key columns' JSON forms
        const keys = Object.fromEntries(
            Object.entries(table[TABLE].columns)
                .filter(([column]) => !EVENT_COLUMNS.has(column))
                .map(([column, builder]) => [
                    column,
                    builder.definition.nullable
                        ? schema.nullable(builder.definition.json)
                        : builder.definition.json,
                ]),
        );

        return new EventKind({
            package: owner,
            name,
            table,
            keys: schema.object(keys),
            data: table.data.definition.json,
            delivery: written.delivery,
            policy: {
                flush: {
                    maxAge: Duration.milliseconds(written.flush.maxAge),
                    maxRows: written.flush.maxRows,
                },
                retention: ObjectType.#retention(written.retention),
            },
            ...(written.isLocked === undefined ? {} : { isLocked: written.isLocked }),
            ...(written.subject === undefined
                ? {}
                : { subject: ObjectType.#subjectKeys(name, fields, written.subject) }),
            ...(written.distribution === undefined ? {} : { distribution: written.distribution }),
        });
    }

    /** Write a declared retention as an event kind's: milliseconds, or the setting it reads. */
    static #retention(retention: EventStorage["retention"]): EventRetention {
        return "setting" in retention ? retention : Duration.milliseconds(retention);
    }

    /** List the key columns of an event's person: a reference's columns, else a text field. */
    static #subjectKeys(
        name: string,
        fields: Readonly<Record<string, Field>>,
        property: string,
    ): readonly string[] {
        // take an object or principal reference's columns, an identifier reference's, or a text field
        const field = fields[`${property}Id`] ?? fields[property];
        if (field?.reference === "object") {
            const { packageId, type, scope, id } = Field.columnsOf(property);

            return [packageId, type, scope, id];
        } else if (field?.reference === "identifier") {
            return [`${property}Id`];
        } else if (field?.type === "string") {
            return [property];
        }

        throw new TypeError(`events object ${name} has no person field ${property}`);
    }

    /** Refuse durable-only declarations, writes and work on ephemeral, external, local objects. */
    #requireStorage(definition: ObjectDefinition, traits: readonly TraitInstance[]): void {
        const refused = ObjectType.#refusedDeclarations(definition, traits, this.storage);

        // refuse the event log without its settings
        if (this.storage === "events" && this.events === undefined) {
            throw new TypeError(`events object ${this.name} has no storage settings`);
        }

        // refuse declarations the storage takes none of, and writes but the package's own and an events type's appends
        if (this.storage === "external" || this.storage === "events") {
            const isAppend = (kind: MethodKind) =>
                this.storage === "events" && (kind === "create" || kind === "createMany");
            const writing = Object.entries(ObjectType.#methodsOf(this)).find(
                ([, declared]) =>
                    declared.mutates &&
                    declared.visibility !== "package" &&
                    !isAppend(declared.kind),
            );
            if (refused.length > 0) {
                throw new TypeError(
                    `${this.storage} object ${this.name} cannot have ${refused[0]}`,
                );
            } else if (writing !== undefined) {
                throw new TypeError(
                    `${this.storage} object ${this.name} cannot write in method ${writing[0]}`,
                );
            }
        }
        // refuse durable declarations and external work on ephemeral objects
        else if (this.storage === "ephemeral") {
            const external = Object.entries(ObjectType.#methodsOf(this)).find(
                ([, declared]) => declared.prepare !== undefined || Method.settles(declared),
            );
            if (refused.length > 0) {
                throw new TypeError(`ephemeral object ${this.name} cannot have ${refused[0]}`);
            } else if (external !== undefined) {
                throw new TypeError(
                    `ephemeral object ${this.name} cannot do external work in method ${external[0]}`,
                );
            }
        }
        // refuse durable declarations, work beyond the client and several scopes on local objects
        else if (this.storage === "local") {
            this.#requireLocal(refused);
        }
    }

    /** Refuse declarations, off-client methods and several scope types on local objects. */
    #requireLocal(refused: readonly string[]): void {
        // refuse prepared and settled work, and a record method declared to run only on the server
        const remote = Object.entries(ObjectType.#methodsOf(this)).find(
            ([, declared]) =>
                (!declared.isPredicted && declared.kind !== "custom") ||
                declared.prepare !== undefined ||
                Method.settles(declared),
        );
        if (refused.length > 0) {
            throw new TypeError(`local object ${this.name} cannot have ${refused[0]}`);
        } else if (remote !== undefined) {
            throw new TypeError(
                `local object ${this.name} cannot run method ${remote[0]} outside the client`,
            );
        } else if (!(this.scope instanceof ObjectType)) {
            throw new TypeError(`local object ${this.name} must belong to one scope type`);
        }
    }

    /** Declare an object type from a definition, deriving its table, traits and methods. */
    static declare(input: ObjectInput, module: ModuleMetadata | undefined): ObjectType {
        // collect the traits, adding what the roles define, and take the built methods
        const owner = ModuleMetadata.require(module, "defineObject").package;
        const { methods: listed, ...written } = input;
        const built =
            listed === undefined
                ? undefined
                : Object.fromEntries(
                      Object.entries(listed).filter(
                          (entry): entry is [string, Method] => entry[1] !== false,
                      ),
                  );
        const expanded = Orderable.expand(
            Presentable.expand(
                Roles.expand(
                    Provisioned.expand({
                        ...written,
                        ...(built === undefined ? {} : { methods: built }),
                    }),
                ),
            ),
        );

        // key each reference under its relation's columns, constraining object references
        const fields = expanded.fields ?? {};
        const definition = {
            ...expanded,
            fields: ObjectType.#keyFields(fields),
            constraints: {
                ...Object.fromEntries(
                    Object.entries(fields).flatMap(([name, field]) =>
                        Object.entries(field.constraints(name)),
                    ),
                ),
                ...expanded.constraints,
            },
        };
        const foreign = definition.mapping?.table;
        const traits = ObjectType.#traitsOf(definition);

        // refuse options a foreign table takes none of
        const refused = FOREIGN_REFUSED.find((key) => written[key] !== undefined);
        if (foreign !== undefined && refused !== undefined) {
            throw new TypeError(
                `object ${definition.name} over a foreign table cannot have ${refused}`,
            );
        }

        // nest objects over a foreign table only through a parent relation on the parent identifier column
        const isUnparented = definition.mapping?.relations["parent"]?.column !== "parentId";
        if (foreign !== undefined && written.nested !== undefined && isUnparented) {
            throw new TypeError(
                `object ${definition.name} over a foreign table has no parent relation to nest through`,
            );
        }

        // add trait methods, but those the definition declares or removes
        const declared: Readonly<Record<string, Method>> = definition.methods ?? {};
        const derived = traits.map(({ trait, options }) =>
            trait.methods(options, { ...listed, ...declared }),
        );
        const traitMethods = Object.fromEntries(
            derived
                .flatMap((each) => Object.entries(each))
                .map(([name, traitMethod]): [string, Method] => [
                    name,
                    Method.declared(traitMethod),
                ]),
        );

        // let a declarable type's managers detach its records from their declarations and attach them again
        const isDeclarable = ObjectType.declares({
            ...definition,
            methods: { ...declared, ...traitMethods },
        });
        const managing = isDeclarable
            ? { detach: Method.declared(detach), attach: Method.declared(attach) }
            : {};
        const clash = [...Object.keys(traitMethods), ...Object.keys(managing)].find((name) =>
            Object.hasOwn(listed ?? {}, name),
        );
        if (clash !== undefined) {
            throw new TypeError(
                `object ${definition.name} declares or removes method ${clash}, but a trait derives it`,
            );
        }
        const methods = { ...declared, ...traitMethods, ...managing };

        // derive the table
        const packageId = (definition.represents?.package ?? owner).id;
        const table =
            foreign ?? ObjectTable.derive({ ...definition, methods }, traits, packageId, module);

        return ObjectType.assemble(owner, {
            ...definition,
            table,
            methods,
            source: { ...written, ...(built === undefined ? {} : { methods: built }) },
        });
    }

    /** List the traits in application order, read at use as traits import this module. */
    static #traits(): readonly Trait<unknown>[] {
        return [
            record,
            readable,
            writable,
            nested,
            transitions,
            recoverable,
            expiring,
            versioned,
            controlled,
            shareable,
            suspendable,
            fencing,
            bindable,
            attachments,
            tracked,
            text,
            search,
            orderable,
            series,
        ];
    }

    /** List a definition's traits with their options, readable alone over a foreign table. */
    static #traitsOf(definition: ObjectDefinition): TraitInstance[] {
        return definition.mapping === undefined
            ? ObjectType.#traits().flatMap((trait) => {
                  const options = trait.options(definition);

                  return options === undefined ? [] : [{ trait, options }];
              })
            : [{ trait: readable, options: {} }];
    }

    /** Key each reference under its relation's columns, declaring its relation. */
    static #keyFields(fields: Readonly<Record<string, Field>>): Readonly<Record<string, Field>> {
        return Object.fromEntries(
            Object.entries(fields).flatMap(([name, field]) => Object.entries(field.expand(name))),
        );
    }

    /** Decide whether a permission allows the holders of a related object's permission. */
    static #covers(
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

        // search the permission's expression
        const expression = permissions[name];

        return (
            expression !== undefined &&
            ObjectType.#includes(permissions, expression, relation, permission, seen)
        );
    }

    /** Decide whether an expression allows a related permission's holders, via unions and names. */
    static #includes(
        permissions: Readonly<Record<string, AccessExpression>>,
        expression: AccessExpression,
        relation: string,
        permission: string,
        seen: Set<string>,
    ): boolean {
        // search each branch of a union
        if (expression.kind === "union") {
            return expression.expressions.some((branch) =>
                ObjectType.#includes(permissions, branch, relation, permission, seen),
            );
        }
        // follow a named permission
        else if (expression.kind === "permission") {
            return ObjectType.#covers(permissions, expression.name, relation, permission, seen);
        }
        // match the related permission itself
        else {
            return (
                expression.kind === "through" &&
                expression.relation === relation &&
                expression.permission === permission
            );
        }
    }

    /** Read an object type's methods by name, as code reading them by a runtime name sees them. */
    static #methodsOf(object: ObjectType): Readonly<Record<string, Method>> {
        return object.methods;
    }

    /** Report whether an expression grants a scope's containment set anywhere in it. */
    static #grantsContainment(expression: AccessExpression): boolean {
        switch (expression.kind) {
            case "contained":
                return true;
            case "union":
            case "intersection":
                return expression.expressions.some((each) => ObjectType.#grantsContainment(each));
            case "exclusion":
                return (
                    ObjectType.#grantsContainment(expression.include) ||
                    ObjectType.#grantsContainment(expression.exclude)
                );
            case "none":
            case "relation":
            case "permission":
            case "resource":
            case "context":
            case "through":
            case "granters":
            case "readers":
            case "subjectSet":
            case "self":
                return false;
        }
    }

    /** List the durable-only declarations of a definition: its durable traits and keys. */
    static #refusedDeclarations(
        definition: ObjectDefinition,
        traits: readonly TraitInstance[],
        storage: ObjectStorage,
    ): string[] {
        const keys = [
            "audited",
            "inherited",
            "isScope",
            "partition",
            "aggregates",
            "indexes",
        ] as const;

        return [
            ...traits
                .filter(
                    ({ trait }) =>
                        trait.storages !== undefined && !trait.storages.includes(storage),
                )
                .map(({ trait }) => present(trait.key, "the key of a trait some storages refuse")),
            ...(storage === "durable"
                ? []
                : keys.filter((key) => {
                      // count a declared key, a defaulted map only with entries
                      const declared = definition[key];
                      const isDefaulted = key === "aggregates" || key === "indexes";

                      return isDefaulted
                          ? Object.keys(declared ?? {}).length > 0
                          : declared !== undefined && declared !== false;
                  })),
        ];
    }

    /** Read the parent of a nested type, absent for a type that is not nested. */
    static #nestedParent(
        type: ObjectType,
        definition: ObjectDefinition<string, Readonly<Record<string, Method>>>,
    ): ObjectType["parent"] {
        return (
            definition.nested && {
                object: definition.nested.in === "self" ? type : definition.nested.in,
                optional: definition.nested.optional ?? false,
                receive: definition.nested.receive,
                delete: definition.nested.delete ?? "cascade",
            }
        );
    }

    /** Read the permission a type's standard method of a kind needs, absent without one. */
    static #methodPermission(object: ObjectType, kind: MethodKind): Permission | undefined {
        const standard = Object.values(object.methods).find((declared) => declared.kind === kind);

        return standard === undefined ? undefined : object.permission(standard.permission);
    }
}

/** An aggregate of objects their holder maintains in one field. */
export type ObjectAggregate = {
    /** The values of the aggregated objects' fields. */
    readonly where?: Readonly<Record<string, string | number | boolean | null>>;
    /** The reference field pointing at the holder object, the parent when absent. */
    readonly via?: string;
} & (
    | {
          /** Count the objects. */
          readonly function: "count";
      }
    | {
          /** Sum a field, or take its least or greatest. */
          readonly function: "sum" | "min" | "max";
          /** The aggregated field. */
          readonly value: string;
      }
);

/** A type of attachments a host takes. */
export interface Attachment<Object extends ObjectType = ObjectType> {
    /** The attachment type, nested in any parent. */
    readonly object: Object;
    /** The host permission whose holders attach. */
    readonly by: string;
}

/** The builder of a definition's declared methods, typed by its table, fields and permissions. */
export type MethodsOf<Definition extends ObjectDefinition> = Definition extends {
    readonly mapping: { readonly table: Table };
}
    ? MethodBuilder<ForeignTable<Definition>, string, Listed<DeclaredPermissions<Definition>>>
    : MethodBuilder<
          DefinedTable<Definition>,
          WrittenField<DefinedFields<Definition>>,
          Listed<DefinedPermissions<Definition>>
      >;

/** The object type over another package's table, with its declared methods. */
export type ForeignType<Definition extends ObjectDefinition, Methods> = ObjectType<{
    readonly name: Definition["name"];
    readonly plural: Definition["plural"];
    readonly identity: Definition["name"];
    readonly table: ForeignTable<Definition>;
    readonly fields: {};
    readonly methods: DeclaredMethods<Methods> &
        ReadableMethodMap<Methods, ForeignTable<Definition>>;
    readonly permissions: DeclaredPermissions<Definition>;
    readonly scope: ScopeIdentityOf<Option<Definition, "scope", ObjectScope>>;
    readonly parent: undefined;
    readonly owner: Option<Definition, "owner">;
    readonly attachments: never;
    readonly storage: "durable";
    readonly events: undefined;
    readonly references: Option<Definition, "references", {}>;
    readonly supplied: Option<Definition, "supplied", readonly never[]>[number];
}>;

/** Read a definition's option, or a default where it sets none. */
type Option<Definition, Key extends PropertyKey, Default = undefined> = Key extends keyof Definition
    ? Exclude<Definition[Key], undefined>
    : Default;

/** Write a union of names out by its members, as messages list them. */
type Listed<Union extends string> = `${Union}`;

/** The permission names a definition declares. */
type DeclaredPermissions<Definition> = Definition extends {
    readonly permissions: infer Permissions;
}
    ? Permissions extends readonly (infer Name extends string)[]
        ? Name
        : keyof Permissions & string
    : never;

/** The options that reference permissions, typed by the permissions a definition grants. */
type Checked<Definition extends ObjectDefinition> = ObjectTraits<
    DefinedPermissions<Definition> | RolePermission
> & {
    /** Permissions only their expressions grant, never roles. */
    readonly reserved?: readonly DefinedPermissions<Definition>[];
    /** Sensitive permissions that apply only after the authentication each requires. */
    readonly elevated?: Readonly<Partial<Record<DefinedPermissions<Definition>, Elevation>>>;
    /** Permissions available while the object's scope is suspended. */
    readonly administration?: readonly DefinedPermissions<Definition>[];
    /** Permissions available while the object's scope moves to another machine. */
    readonly moving?: readonly DefinedPermissions<Definition>[];
    /** Checks, indexes, unique constraints and foreign keys over the columns, by name. */
    readonly constraints?: DefinedConstraints<Definition>;
    /** Where the objects live: a kind, or the event log with its settings, never the bare log. */
    readonly storage?: Exclude<ObjectStorage, "events"> | EventStorage;
} & {
    /** Keys no definition takes, refused. */
    readonly [Key in Exclude<keyof Definition, Exclude<keyof ObjectDefinition, "methods">>]: never;
};

/** The constraints a definition declares, with its columns and those of the referenced types. */
type DefinedConstraints<Definition extends ObjectDefinition> = {
    readonly [Name in keyof Option<Definition, "constraints", {}>]: ObjectConstraint<
        DefinedColumn<Definition>,
        ReferencedColumn<Option<Definition, "constraints", {}>[Name], DefinedColumn<Definition>>
    >;
};

/** The columns a foreign key references: its table's, a typed object type's, or any type's. */
type ReferencedColumn<Constraint, Column extends string> = Constraint extends {
    readonly references: { readonly object: infer Referenced };
}
    ? Referenced extends "self"
        ? Column
        : Referenced extends () => ObjectType<infer Configuration>
          ? string extends Configuration["name"]
              ? string
              : keyof Configuration["table"][typeof TABLE]["columns"] & string
          : string
    : string;

/** The foreign table of an object type over another package's table. */
type ForeignTable<Definition extends ObjectDefinition> = NonNullable<
    Definition["mapping"]
>["table"];

/** The traits a definition sets that type its table, methods or declarations. */
type DefinitionTraits<Definition> = TraitOptions<{
    readonly key: Option<Definition, "key">;
    readonly nested: Option<Definition, "nested">;
    readonly recoverable: Option<Definition, "recoverable">;
    readonly versioned: Option<Definition, "versioned">;
    readonly controlled: Option<Definition, "controlled">;
    readonly suspendable: Option<Definition, "suspendable">;
    readonly fencing: Option<Definition, "fencing">;
    readonly bindable: Option<Definition, "bindable">;
    readonly tracked: Option<Definition, "tracked">;
}>;

/** The trait options a definition sets, without the absent ones. */
type TraitOptions<Traits> = {
    readonly [Key in keyof Traits as [Traits[Key]] extends [undefined] ? never : Key]: Traits[Key];
};

/** The traits a definition's objects take: its traits, and those its kind adds. */
type DefinedTraits<Definition> = DefinitionTraits<Definition> &
    ProvisionedTraitsOf<Option<Definition, "provisioned">>;

/** The fields a definition's objects have: its fields, and those of its sharing, kind, traits. */
type DefinedFields<Definition extends ObjectDefinition> = FieldsByColumn<
    NonNullable<Definition["fields"]> &
        RoleFieldsOf<Option<Definition, "shareable">, Option<Definition, "isScope", false>> &
        ProvisionedFieldsOf<Option<Definition, "provisioned">> &
        PresentationFieldsOf<Option<Definition, "presentable">> &
        OrderedFieldsOf<Option<Definition, "orderable">>
>;

/** The permissions a definition's objects have. */
type DefinedPermissions<Definition> =
    | DeclaredPermissions<Definition>
    | RolePermissionsOf<Option<Definition, "shareable">>
    | ProvisionedPermissionOf<Option<Definition, "provisioned">>
    | TraitPermissionsOf<DefinedTraits<Definition>>;

/** The scope identity of a definition's objects. */
type DefinedScope<Definition> = ScopeIdentityOf<Option<Definition, "scope", ObjectScope>>;

/** The prefix of a definition's identifiers. */
type DefinedIdentity<Definition extends ObjectDefinition> = Option<
    Definition,
    "identity",
    Definition["name"]
>;

/** The table a definition derives. */
type DefinedTable<Definition extends ObjectDefinition> = ObjectTable<
    Definition["name"],
    DefinedScope<Definition>,
    DefinedFields<Definition>,
    TableTraitsOf<DefinedTraits<Definition>>,
    DefinedIdentity<Definition>,
    StorageKind<Option<Definition, "storage", "durable">>
>;

/** The columns a definition's constraints name. */
type DefinedColumn<Definition extends ObjectDefinition> = ConstraintColumn<
    FieldsByColumn<NonNullable<Definition["fields"]>>,
    StorageKind<Option<Definition, "storage", "durable">>
>;

/** The query keys of an events type's table: a schema per column beside the event columns. */
type EventShapeOf<Definition extends Table> = {
    readonly [Key in Exclude<keyof Select<Definition> & string, EventColumn>]: schema.Schema<
        Select<Definition>[Key]
    >;
};

/** The data of an events type's table, any JSON for a table without a typed data column. */
type EventDataOf<Definition extends Table> =
    Select<Definition> extends { readonly data: infer Data extends JsonValue } ? Data : JsonValue;

/** Refuse the fields of an input beyond those a call takes, which spreads would otherwise pass. */
type ExtraFields<Input, Taken> =
    string extends TakenKey<Taken>
        ? {}
        : { readonly [Key in Exclude<keyof Input & string, TakenKey<Taken>>]: never };

/** The fields any variant of a call's input takes. */
type TakenKey<Taken> = Taken extends unknown ? keyof Taken & string : never;

/** The methods a definition declares, without those it removes. */
type DeclaredMethods<Methods> = {
    readonly [Name in keyof Methods as Methods[Name] extends false ? never : Name]: Methods[Name];
};

/** The object type a definition declares with its methods. */
export type DefinedType<Definition extends ObjectDefinition, Methods> = ObjectType<{
    readonly name: Definition["name"];
    readonly plural: Definition["plural"];
    readonly identity: DefinedIdentity<Definition>;
    readonly table: DefinedTable<Definition>;
    readonly fields: DefinedFields<Definition>;
    readonly methods: DeclaredMethods<Methods> &
        ReadableMethodMap<Methods, DefinedTable<Definition>> &
        WritableMethodMap<
            Methods & ProvisionedMethodsOf<Option<Definition, "provisioned">>,
            DefinedPermissions<Definition>,
            DefinedTable<Definition>,
            WrittenField<DefinedFields<Definition>>,
            Option<Definition, "versioned"> extends undefined ? true : false,
            Option<Definition, "recoverable"> extends undefined ? true : false
        > &
        ProvisionedMethodsOf<Option<Definition, "provisioned">> &
        SeriesMethodMap<StorageKind<Option<Definition, "storage", "durable">>> &
        ManagerMethodMap<Definition, Methods> &
        TraitMethods<
            FieldsByColumn<NonNullable<Definition["fields"]>>,
            DefinedTraits<Definition> & {
                readonly shareable: Option<Definition, "partition"> extends true
                    ? undefined
                    : Option<Definition, "represents"> extends Policy<string, string, true>
                      ? undefined
                      : ProvisionedSharingOf<
                            Option<Definition, "provisioned">,
                            Option<Definition, "shareable">
                        >;
            }
        >;
    readonly permissions: DefinedPermissions<Definition> | ManagerPermissionOf<Definition, Methods>;
    readonly scope: DefinedScope<Definition>;
    readonly parent: Option<Definition, "nested"> extends { readonly in: infer Parent }
        ? Parent extends ObjectType
            ? IdentityOf<Parent>
            : Parent extends "self" | "any"
              ? Parent
              : undefined
        : undefined;
    readonly owner: Option<Definition, "owner">;
    readonly attachments: IdentityOf<Option<Definition, "attachments", []>[number]["object"]>;
    readonly storage: StorageKind<Option<Definition, "storage", "durable">>;
    readonly events: StorageKind<Option<Definition, "storage", "durable">> extends "events"
        ? EventKind<EventShapeOf<DefinedTable<Definition>>, EventDataOf<DefinedTable<Definition>>>
        : undefined;
    readonly references: Option<Definition, "references", {}>;
    readonly supplied: Option<Definition, "supplied", readonly never[]>[number];
}>;

/** The traits as a derived table reads them, a nested parent by its identity and its key. */
export type TableTraitsOf<Traits> = {
    readonly [Key in keyof Traits]: Key extends "nested"
        ? Traits[Key] extends { readonly in: infer Parent }
            ? Omit<Traits[Key], "in"> &
                  (Parent extends ObjectType
                      ? { readonly in: IdentityOf<Parent>; readonly id: ReferenceOf<Parent> }
                      : { readonly in: Parent })
            : Traits[Key]
        : Traits[Key];
};

/** The permissions traits derive for their methods, granted only by roles unless declared. */
export type TraitPermissionsOf<Traits> =
    | ([TraitOf<Traits, "controlled">] extends [undefined] ? never : "control")
    | (TraitOf<Traits, "controlled"> extends { readonly approval: true } ? "approve" : never)
    | ([TraitOf<Traits, "expiring">] extends [undefined] ? never : "expire")
    | ([TraitOf<Traits, "bindable">] extends [undefined] ? never : "bind");

/** The permission a durable type declaring create derives for its records' managers. */
type ManagerPermissionOf<Definition, Methods> =
    IsDeclarable<Definition, Methods> extends true ? "manage" : never;

/** Whether stacks can declare the objects: durable ones with a declared or written create. */
type IsDeclarable<Definition, Methods> =
    StorageKind<Option<Definition, "storage", "durable">> extends "durable"
        ? Methods extends { readonly create: Method }
            ? true
            : ["write"] extends [DeclaredPermissions<Definition>]
              ? true
              : false
        : false;

/** The methods a durable type declaring create gives its records' managers. */
export type ManagerMethodMap<Definition, Methods> =
    IsDeclarable<Definition, Methods> extends true
        ? { readonly detach: typeof detach; readonly attach: typeof attach }
        : {};

/** The methods an object's traits derive beside its declared ones. */
export type TraitMethods<Fields, Traits> = TransitionMethodMap<Fields> &
    RecoverableMethodMap<TraitOf<Traits, "recoverable">> &
    ControlledMethodMap<TraitOf<Traits, "controlled">> &
    NestedMethodMap<TraitOf<Traits, "nested">> &
    ShareableMethodMap<SharingGateOf<TraitOf<Traits, "shareable">>> &
    SuspendableMethodMap<GateOf<TraitOf<Traits, "suspendable">>> &
    FencingMethodMap<GateOf<TraitOf<Traits, "fencing">>> &
    BindableMethodMap<TraitOf<Traits, "bindable">> &
    TrackedMethodMap<TraitOf<Traits, "tracked">> &
    TextMethodMap<Fields>;

/** Declare an object type from its data, with the methods it declares beside the derived ones. */
export function defineObject<
    const Definition extends ObjectDefinition & Checked<Definition>,
    const Methods extends Readonly<Record<string, Method | false>> = {},
>(
    definition: Definition,
    methods?: (method: MethodsOf<Definition>) => Methods,
    module?: ModuleMetadata,
): Definition extends { readonly mapping: { readonly table: Table } }
    ? ForeignType<Definition, Methods>
    : DefinedType<Definition, Methods>;
/**
 * Declare an object type, its type computed from its definition by the signature above.
 *
 * NOTE #Architecture: the one stated type of object, as zod's `z.object` and Drizzle's `pgTable`.
 */
export function defineObject(
    definition: ObjectInput,
    methods?: (method: MethodBuilder) => Readonly<Record<string, Method | false>>,
    module?: ModuleMetadata,
): unknown {
    return ObjectType.declare(
        methods === undefined
            ? definition
            : { ...definition, methods: methods(new MethodBuilder()) },
        module,
    );
}

/** A definition with the methods its builder function declared. */
type ObjectInput = Omit<ObjectDefinition, "methods"> & {
    /** The operations callers may execute, keyed by method name, false removing a derived one. */
    readonly methods?: Readonly<Record<string, Method | false>>;
};

/** The name of a field whose values need a separate read permission. */
export type GuardedField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly guarded: true } ? Name : never;
}[keyof Fields & string];

/** The name of a text field. */
export type TextFieldName<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends TextField ? Name : never;
}[keyof Fields & string];

/** The name of a field with sensitive values. */
export type SensitiveField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly isSensitive: true }
        ? Name
        : never;
}[keyof Fields & string];

/** The field a creation fills with the caller, set directly only by server-only creations. */
export type CallerField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly isCallerFilled: true }
        ? Name
        : never;
}[keyof Fields & string];

/** The name of a field callers write after creation. */
export type UpdatedField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly written: true } ? Name : never;
}[keyof Fields & string];

/** The name of a field callers write. */
export type WrittenField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly written: false }
        ? never
        : Name;
}[keyof Fields & string];
