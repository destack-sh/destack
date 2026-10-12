import { AuditTarget } from "@destack/audit";
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
    Replica,
    Scope,
    type ObjectReference,
    type ObjectTypeReference,
    type Query,
    type Subject,
} from "@destack/sync";
import type * as sync from "@destack/sync";
import {
    type ColumnMap,
    Snapshot,
    Expression,
    Condition,
    eq,
    inArray,
    DatabaseError,
    type RowImage,
    type SQL,
    TABLE,
    type Table,
    type TableConstraint,
    type Tree,
} from "@destack/db";
import {
    aligned,
    Duration,
    Identifier,
    type JsonObject,
    present,
    schema,
    Version,
} from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { AuditAction } from "@destack/audit";
import { ModuleMetadata, type Package } from "@destack/package";
import type { Field, ObjectFields, TextField } from "../field/field.ts";
import {
    Call,
    type Bivariant,
    type Handler,
    type TypesOf,
    type ObjectCall,
} from "../method/call.ts";
import { Method, type MethodBuilder, type MethodKind } from "../method/method.ts";
import { method } from "../method/record.ts";
import { InputSchema } from "../method/input.ts";
import type { Client, Service } from "@destack/service";
import { createClient, type ClientOptions } from "@destack/service/client";
import {
    objectProcedures,
    objectSchema,
    scopeRoute,
    type ObjectCallInput,
    type CallOutput,
    type Calls,
    type MethodName,
    type MethodProcedure,
    type RowSchema,
    type ScopeInput,
    type TargetInput,
    type ObjectProcedures,
    type ScopeRoute,
    type ObjectSchema,
} from "../method/procedure.ts";
import {
    replicaProcedures,
    type OpenQueryOptions,
    type ObjectQuery,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { compile, compileQueries } from "../query/query.ts";
import { record } from "../trait/record.ts";
import {
    declarable,
    adoptable,
    type DeclarableDefinition,
    type DeclarableMethodMap,
    type AdoptableMethodMap,
} from "../trait/declarable.ts";
import {
    controlled,
    type ControlledDefinition,
    type ControlledMethodMap,
} from "../trait/controlled.ts";
import { nested, type NestedMethodMap, type NestedDefinition } from "../trait/nested.ts";
import { transitions, type TransitionMethodMap } from "../trait/transition.ts";
import {
    recoverable,
    type RecoverableDefinition,
    type RecoverableMethodMap,
} from "../trait/recoverable.ts";
import { expiring, type ExpiryRule } from "../trait/expiring.ts";
import { projected, type ProjectedDefinition } from "../trait/projected.ts";
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
import { INTRINSIC, type Intrinsic } from "./intrinsic.ts";
import { storeTables } from "../stack/db.ts";
import type { ObjectController } from "./controller.ts";
import {
    DeclarationIndex,
    encodeType,
    type KnownDeclaration,
    ObjectDescription,
} from "./description.ts";
import type { DeclarationOf, DeclarationHandler, ManagedTable } from "../server/stack.ts";
import { camelCase, kebabCase } from "./name.ts";
import { ObjectAccess } from "./access.ts";
import { deriveTable, type ConstraintColumns, type ObjectTable, type TraitOf } from "./table.ts";

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
}

/** The scope and identifier of an object's row, read from a row of any object table. */
export const ObjectKey = schema.looseObject({ scope: schema.string(), id: schema.string() });

/** The identifier schemas of object type identities, one each, shared by derived copies. */
const IDENTIFIERS = new Map<string, ReturnType<typeof schema.identifier>>();

/** The default ephemeral linger, in milliseconds: 10 s spans two 1–3 s reconnects. */
const LINGER_MILLISECONDS = 10_000;

/** The relation to an object's scope of any scope type, read as `through("scope", permission)`. */
export const SCOPE = "scope";

/** The permission on a scope's own object that shows the scope. */
export const SCOPE_READ = "read";

/** The permission to copy the access rows of the scopes above a scope. */
export const REPLICATE = "replicate";

/** Create an empty object inheriting from a prototype. */
const inherit: (prototype: object | null) => object = Object.create;

/** Read the prototype an object inherits from. */
const prototypeOf: (value: object) => object | null = Object.getPrototypeOf;

/** Every trait, in application order. */
const TRAITS: readonly Trait<unknown>[] = [
    record,
    nested,
    transitions,
    recoverable,
    expiring,
    projected,
    versioned,
    controlled,
    declarable,
    adoptable,
    shareable,
    suspendable,
    bindable,
    attachments,
    tracked,
    text,
    search,
    orderable,
];

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
}

/** The scope objects live in: a scope type, or the universe. */
export type ObjectScope = typeof Scope.universe.id | ObjectType;
/** The scope objects live in: a scope type, or the universe. */
export const ObjectScope = {
    /** Decide whether objects live in several scopes. */
    isList(scope: ObjectScope | readonly ObjectScope[]): scope is readonly ObjectScope[] {
        return Array.isArray(scope);
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
export interface ObjectTraits<Declared = unknown, Permissions extends string = string> {
    /** The parent object type, or "self" for a tree. */
    readonly nested?: NestedDefinition;
    /** Deleted objects stay restorable for a window. */
    readonly recoverable?: RecoverableDefinition<Permissions>;
    /** The rules after which their store removes the objects. */
    readonly expiring?: readonly ExpiryRule[];
    /** Project another type's rows into the objects, each its own object with its own state. */
    readonly projected?: ProjectedDefinition;
    /** The types projecting the objects into recipients' homes, as their `projected` refers. */
    readonly projections?: () => readonly ObjectType[];
    /** The objects' natural key, by which callers and sources identify them in place of ids. */
    readonly key?: schema.Schema<string>;
    /** The scopes inside each object's scope that copy the objects, matching any condition. */
    readonly inherited?: { readonly where?: Condition };
    /** The objects are immutable, numbered versions of their parent. */
    readonly versioned?: VersionsDefinition;
    /** A controller reconciles the objects. */
    readonly controlled?: ControlledDefinition;
    /** Installations bind to the objects, becoming their consumers. */
    readonly bindable?: true | Gated<Permissions>;
    /** The objects are a kind's resources, which its providers provision. */
    readonly provisioned?: ProvisionedDefinition;
    /** Stacks declare the objects. */
    readonly declarable?: DeclarableDefinition<Declared>;
    /** Holders of the permission detach a declared record, and adopt a direct record into one. */
    readonly adoptable?: Gated<Permissions>;
    /** Holders of the permission share the objects. */
    readonly shareable?: ShareableDefinition<Permissions>;
    /** Holders of the permission suspend and resume a scope object. */
    readonly suspendable?: Gated<Permissions>;
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
    Declared = unknown,
    Permissions extends string = string,
    Methods extends Readonly<Record<string, Method>> = {},
    Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope | readonly ObjectScope[],
    Granted extends string = Permissions,
> extends ObjectTraits<Declared, NoInfer<Granted>> {
    /** The singular name, unique within the declaring package. */
    readonly name: string;
    /** The prefix of the objects' identifiers, the name when absent. */
    readonly identity?: string;
    /** The plural name. */
    readonly plural: string;
    /** The scope types the objects live in, any scope when absent. */
    readonly scope?: Scope;
    /** The reference relation to each object's owner when it lives in another scope. */
    readonly owner?: string;
    /** Attributes permission expressions read. */
    readonly attributes?: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** Further relations kept in relationships. */
    readonly relations?: Readonly<Record<string, ObjectRelationInput>>;
    /** Permission names roles grant, or named expressions derived from relations. */
    readonly permissions?: ObjectPermissions<Permissions>;
    /** Permissions only their expressions grant, never roles. */
    readonly reserved?: readonly NoInfer<Granted>[];
    /** Sensitive permissions that apply only after the authentication each requires. */
    readonly elevated?: Readonly<Partial<Record<NoInfer<Granted>, Elevation>>>;
    /** Permissions available while the object's scope is suspended. */
    readonly administration?: readonly NoInfer<Granted>[];
    /** An access-owned type whose identity the objects take. */
    readonly represents?: Policy;
    /** Whether the objects are scopes containing other objects. */
    readonly isScope?: boolean;
    /** Whether each object is a partition, as only a scope type is. */
    readonly partition?: true;
    /** The operations callers may execute, keyed by method name. */
    readonly methods?: Methods;
    /** The fields each object has, for objects declared by fields. */
    readonly fields?: Readonly<Record<string, Field>>;
    /** The aggregates of these objects their holder keeps, by field name. */
    readonly aggregates?: Readonly<Record<string, ObjectAggregate>>;
    /** The attachments the objects take. */
    readonly attachments?: readonly Attachment[];
    /** Indexes, unique constraints and checks over the derived columns. */
    readonly constraints?: ObjectConstraints;
    /** Unique indexes the key index keeps across databases, by name. */
    readonly indexes?: Readonly<Record<string, ObjectIndex>>;
    /** Where the objects live. */
    readonly storage?: ObjectStorage;
    /** The previous names of renamed fields, by current field. */
    readonly moved?: { readonly fields?: Readonly<Record<string, string>> };
    /** The fields each release computes from stored rows and earlier inputs, by release. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** How long ephemeral objects outlive their session, 10 s by default. */
    readonly linger?: Duration;
}

/**
 * Where objects live: durable, ephemeral, external, or local.
 */
export type ObjectStorage = "durable" | "ephemeral" | "external" | "local";

/** How a type's objects are declared, deleted, expired, versioned, tracked and converted. */
export interface ObjectLifecycle<Declared = unknown, Permissions extends string = string> {
    /** The schema of one declaration in a stack, when stacks may declare the objects. */
    readonly declarationSchema: schema.Schema<Declared> | undefined;
    /** How deleted objects stay restorable, for recoverable objects. */
    readonly recoverable: RecoverableDefinition<Permissions> | undefined;
    /** When their store removes the objects, for expiring objects. */
    readonly expiring: readonly ExpiryRule[] | undefined;
    /** Whether the objects are numbered versions of their parent. */
    readonly versioned: VersionsDefinition | undefined;
    /** How tracked objects keep their history. */
    readonly tracked: TrackedDefinition<Permissions> | undefined;
    /** The previous names of renamed fields, by current field. */
    readonly moved: Readonly<Record<string, string>>;
    /** The fields each release computes from stored rows and earlier callers' inputs. */
    readonly convert: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
}

/** A definition as an object type retains it, with identity, storage and field lists resolved. */
type ObjectTypeDefinition<Configuration extends ObjectConfiguration> = ObjectDefinition<
    Configuration["declared"],
    Configuration["permissions"],
    Configuration["methods"]
> & {
    readonly name: Configuration["name"];
    readonly identity: Configuration["identity"];
    readonly plural: Configuration["plural"];
    readonly table: Configuration["table"];
    readonly methods: Configuration["methods"];
    readonly storage: Configuration["storage"];
    readonly guarded: readonly string[];
    readonly written: readonly string[];
    readonly sensitive: readonly string[];
    readonly text: readonly string[];
    readonly source: ObjectSource;
};

/** A definition as a type keeps it: expanded by traits, lists present, built members apart. */
export type ObjectAssembly<Configuration extends ObjectConfiguration = ObjectConfiguration> =
    ObjectTypeDefinition<Configuration> & {
        readonly scope: ObjectScope | readonly ObjectScope[];
        readonly fields: Configuration["fields"];
        readonly aggregates: Readonly<Record<string, ObjectAggregate>>;
        readonly attachments: readonly Attachment[];
        readonly indexes: Readonly<Record<string, ObjectIndex>>;
    };

/** An object type's definition as written, with built methods and any intrinsic foreign table. */
export type ObjectSource = ObjectDefinition & { readonly [INTRINSIC]?: Intrinsic };

/** An object type's code: method handlers by name, its controller and its stack handler. */
export interface ObjectImplementation<
    Self extends ObjectType = ObjectType,
    Collected = unknown,
    Resolved = Collected,
> {
    /** The handler of each method, by method name. */
    readonly methods?: {
        readonly [Name in keyof Self["methods"]]?: Handler<
            TypesOf<Self["methods"][Name], Self["table"]>
        >;
    };
    /** Reconcile or follow the objects with work waiting as the store's principal. */
    readonly controller?: ObjectController<Self>;
    /** How a stack's declarations of the objects become their managed records. */
    readonly declaration?: DeclarationHandler<Self, Collected, Resolved>;
}

/** A unique index the key index keeps across databases. */
export interface ObjectIndex {
    /** The indexed fields, in key order. */
    readonly on: readonly string[];
    /** Whether each key identifies at most one object. */
    readonly unique: true;
    /** The scope type of the keys' uniqueness: the universe or an enclosing type, read lazily. */
    readonly across: () => ObjectType | typeof universe;
    /** The namespace shared with other types' same-named indexes, the type's own if absent. */
    readonly namespace?: string;
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
    /** The declarations' value, for declarable types. */
    readonly declared: unknown;
    /** The permissions the type grants. */
    readonly permissions: string;
    /** The methods, by name. */
    readonly methods: Readonly<Record<string, Method>>;
    /** The objects' scope: "universe", its scope type's identity, or any string for several. */
    readonly scope: string;
    /** Where the objects live. */
    readonly storage: ObjectStorage;
    /** The fields, by name, whose signatures say which are guarded, written, sensitive or text. */
    readonly fields: Readonly<Record<string, Field>>;
    /** The parent type's identity, "self" for a tree, "any" for attachments, undefined for none. */
    readonly parent: string | undefined;
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
    /** How stack declarations become managed records: the handler, else a provisioned kind's. */
    readonly declaration: DeclarationHandler | undefined;

    /** The scope types the objects live in, none for universe objects and objects in any scope. */
    readonly scopes: readonly ObjectType[];
    /** The parent of nested objects. */
    readonly parent:
        | {
              /** The owning object type, or "any" for attachments. */
              readonly object: ObjectType | "any";
              /** Whether an object may have no parent. */
              readonly optional: boolean;
              /** The parent permission a caller needs to create or move an object under it. */
              readonly receive: string;
              /** Delete objects with their parent, or refuse deleting a parent with any. */
              readonly delete: "cascade" | "restrict";
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
    /** How long ephemeral objects outlive their session, in milliseconds. */
    readonly linger: number | undefined;
    /** The schemas, procedures and mapping derived from the definition, built lazily. */
    private derived: Derived = ObjectType.#underived();

    /** Retain a definition with its table and methods, and assemble its policy. */
    constructor(owner: Package, definition: ObjectTypeDefinition<Configuration>) {
        // require a logged table, and keep the definition with its lists present
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

        // retain the methods, requiring the conversions up to the package's release
        this.methods = definition.methods;
        ObjectType.#requireConversions(definition, owner);
        this.declaration = ObjectType.#declarationOf(definition);

        // compute the scopes, nesting, sharing, presentation, ordering, search and linger once
        this.scopes = ObjectType.#scopeTypes(this.definition.scope);
        this.parent = nestedParent(this, definition);
        this.roles = Roles.of(definition);
        this.presentation = Presentable.of(definition);
        this.ordering = Orderable.of(definition);
        this.search =
            definition.search === undefined ? undefined : ObjectSearch.of(definition.search);
        this.linger = this.storage === "ephemeral" ? lingerOf(definition) : undefined;

        // resolve the relations, add trait policies and register the policy
        const assembly = ObjectAccess.policy(this, definition, owner);
        this.permissions = assembly.permissions;
        this.policy = assembly.policy;

        // list the tables a database needs, then require valid members and traits
        this.tables = this.#databaseTables();
        this.#requireValid(definition);
    }

    /** The definition as written, its methods built, which descriptions encode. */
    get source(): ObjectSource {
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

    /** The reference relation to each object's owner, absent if the scope is the owner. */
    get owner(): string | undefined {
        return this.definition.owner;
    }

    /** The principal kind each object stands for, absent for objects standing for none. */
    get represents(): Policy | undefined {
        return this.definition.represents;
    }

    /** Where the objects live. */
    get storage(): Configuration["storage"] {
        return this.definition.storage;
    }

    /** The fields each object has, for objects declared by fields. */
    get fields(): Configuration["fields"] {
        return this.definition.fields;
    }

    /** The fields needing their own read permission. */
    get guarded(): readonly string[] {
        return this.definition.guarded;
    }

    /** The fields callers write. */
    get written(): readonly string[] {
        return this.definition.written;
    }

    /** The columns with sensitive values. */
    get sensitive(): readonly string[] {
        return this.definition.sensitive;
    }

    /** The text fields, kept in chunks. */
    get text(): readonly string[] {
        return this.definition.text;
    }

    /** The aggregates of these objects their holder keeps, by field name. */
    get aggregates(): Readonly<Record<string, ObjectAggregate>> {
        return this.definition.aggregates;
    }

    /** The attachments the objects take. */
    get attachments(): readonly Attachment[] {
        return this.definition.attachments;
    }

    /** The unique indexes the key index keeps across databases, by name. */
    get indexes(): Readonly<Record<string, ObjectIndex>> {
        return this.definition.indexes;
    }

    /** The objects' natural key, absent for objects with generated identifiers. */
    get keyed(): schema.Schema<string> | undefined {
        return this.definition.key;
    }

    /** The rows the scopes inside each object's scope copy, for inherited objects. */
    get inherited(): { readonly where?: Condition } | undefined {
        return this.definition.inherited;
    }

    /** The kind whose resources the objects are, absent for other objects. */
    get provisioned(): ProvisionedDefinition | undefined {
        return this.definition.provisioned;
    }

    /** The type whose rows the objects project, absent for objects projecting none. */
    get projected(): ProjectedDefinition | undefined {
        return this.definition.projected;
    }

    /** Read the types projecting the objects into recipients' homes, as declared. */
    get projecting(): (() => readonly ObjectType[]) | undefined {
        return this.definition.projections;
    }

    /** Whether reads of the objects are audited. */
    get isReadAudited(): boolean {
        return this.definition.audited?.reads === true;
    }

    /** The traits the objects take, with their options. */
    get traits(): readonly TraitInstance[] {
        const traits = this.derived.traits ?? traitsOf(this.definition);
        this.derived.traits = traits;

        return traits;
    }

    /** How the objects are declared, deleted, expired, versioned, tracked and converted. */
    get lifecycle(): ObjectLifecycle<Configuration["declared"], Configuration["permissions"]> {
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

    /** Where access finds objects in another package's table, as an intrinsic type maps it. */
    get intrinsic(): Omit<TableMapping, "policy"> | undefined {
        return this.source[INTRINSIC]?.mapping;
    }

    /** The name the type is served, routed and audited under: its name in camel case. */
    get key(): string {
        return camelCase(this.name);
    }

    /** Reference one object in its scope. */
    reference(scope: string, id: string): ObjectReference {
        return this.policy.reference(scope, id);
    }

    /** Decide whether a value is an object type. */
    static is(value: unknown): value is ObjectType {
        return value instanceof ObjectType;
    }

    /** The package-qualified type of the objects, as access refers to it. */
    get typeReference(): ObjectTypeReference {
        return this.policy.typeReference;
    }

    /** The schema of the objects' identifiers: natural key, or generated typed identifiers. */
    get idSchema(): schema.Schema<string> {
        if (this.keyed !== undefined) {
            return this.keyed;
        }
        const known = IDENTIFIERS.get(this.identity) ?? schema.identifier(this.identity);
        IDENTIFIERS.set(this.identity, known);

        return known;
    }

    /** Whether an identifier identifies one of the type's objects. */
    identifies(id: string): boolean {
        return this.idSchema.safeParse(id).success;
    }

    /** Generate a new object's identifier, refusing a type with a natural key. */
    generateId(): Identifier<string> {
        this.requireGenerated();

        return Identifier.create(this.identity);
    }

    /** Refuse generating an identifier for a type with a natural key. */
    private requireGenerated(): void {
        if (this.keyed !== undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `a creation of ${this.name} gives its key`,
            });
        }
    }

    /** Derive a request object's identifier at a position, stable on repeat. */
    deriveId(requestId: string, position: number): Identifier<string> {
        this.requireGenerated();

        return Identifier.derive(this.identity, requestId, position);
    }

    /** Read an identifier of the type's identity, refusing any other. */
    identifier<Self extends ObjectType>(this: Self, id: string): Identifier<IdentityOf<Self>>;
    /**
     * Parse the identifier by the type's identity.
     *
     * @construct the parse requires `this.identity`'s prefix, the literal IdentityOf reads.
     */
    identifier(id: string): string {
        return this.idSchema.parse(id);
    }

    /** Decide whether another type shares this one's table. */
    same(other: ObjectType | "any" | undefined): boolean {
        return other instanceof ObjectType && other.table === this.table;
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
        const declared = methodsOf(this)[name];
        for (const conversions of [this.lifecycle.convert, declared?.convert ?? {}]) {
            for (const [release, assignments] of Object.entries(conversions)) {
                releases[release] = { ...releases[release], ...assignments };
            }
        }

        return releases;
    }

    /** Read a declared method by name, refusing a name the type does not declare. */
    method(name: string): Method {
        const declared = Object.hasOwn(this.methods, name) ? methodsOf(this)[name] : undefined;
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

    /** Where the objects' routes keep their scope. */
    get route(): ScopeRoute {
        return scopeRoute(this);
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

        // leave out the field a pushed call replays under, kept where a written field of its name replaces it
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

    /** The schema of one object as callers see it, typed by the type's fields. */
    rowSchema<Self extends ObjectType>(this: Self): RowSchema<Self>;
    /**
     * Read the row schema of the type's own fields.
     *
     * @construct the row schema comes from this type's fields, as RowSchema maps the same type.
     */
    rowSchema(): ObjectSchema["row"] {
        return this.schema.row;
    }

    /** The procedures of the object's methods. */
    get procedures(): ObjectProcedures<ObjectType<Configuration>> {
        return objectProcedures<ObjectType<Configuration>>(this);
    }

    /** The procedures every object type shares. */
    get shared(): { readonly replica: ReplicaProcedures } {
        return { replica: replicaProcedures };
    }

    /** Derive the audit action `Noun.method` recording one method. */
    audit(name: string): AuditAction {
        // read the method's audit details
        const declared = methodsOf(this)[name]?.audit;

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
        const declared = methodsOf(this)[name];
        if (declared?.target === true) {
            const id = schema.string().parse(input["id"]);
            const target = { type: this.name, id };

            return { action: this.audit(name), values: { target, details: {} } };
        }

        // target the scope's collection
        const target = { type: this.plural, id: scope };

        return { action: this.audit(name), values: { target, details: {} } };
    }

    /** Call one of the type's methods as a value, which a store or a call runs as its caller. */
    call<Self extends ObjectType, Name extends MethodName<Self>>(
        this: Self,
        name: Name,
        input: NoInfer<ObjectCallInput<Self, Name>>,
    ): ObjectCall<CallOutput<Self, Name>>;
    /**
     * Call one of the type's methods as a value.
     *
     * @construct the result is the method's procedure output, as CallOutput reads the same method.
     * The method's input schema validates the input when the call runs.
     */
    call(name: string, input: Readonly<Record<string, unknown>>): ObjectCall {
        return { method: `${this.name}.${name}`, input, release: this.package.version };
    }

    /** Point a call at an object: its scope as the type's scope field, and its identifier. */
    target<Self extends ObjectType>(
        this: Self,
        row: { readonly scope: string; readonly id: string },
    ): TargetInput<Self>;
    /**
     * Point a call at an object.
     *
     * @construct the scope field is the type's route field, as TargetInput reads the ScopeField.
     */
    target(row: { readonly scope: string; readonly id: string }): Readonly<Record<string, string>> {
        return { ...this.in(row.scope), id: row.id };
    }

    /** Place a call in a scope, as the type's scope field, for calls creating objects there. */
    in<Self extends ObjectType>(this: Self, scope: string): ScopeInput<Self>;
    /**
     * Place a call in a scope.
     *
     * @construct the scope field is the type's route field, as ScopeInput reads the ScopeField.
     */
    in(scope: string): Readonly<Record<string, string>> {
        const field = this.route.field;

        return field === undefined ? {} : { [field]: scope };
    }

    /** Build calls of mutating methods, to run later in their sent scope or input scope. */
    calls<Self extends ObjectType>(this: Self): Calls<Self>;
    /**
     * Build calls of the mutating methods.
     *
     * @construct each key is a mutating method of this type, whose call records its declared input.
     */
    calls(): Readonly<Record<string, (input: JsonObject) => sync.Call>> {
        const methods = Object.entries(methodsOf(this));
        const calls = methods
            .filter(([, declared]) => declared.mutates)
            .map(([name]): [string, (input: JsonObject) => sync.Call] => [
                name,
                (input) => Call.record(this, name, input),
            ]);

        return Object.fromEntries(calls);
    }

    /** Compile a query or include of the objects. */
    query(shape: OpenQueryOptions, objects: readonly ObjectType[]): Omit<Query, "scopes"> {
        return compile(this, shape, objects);
    }

    /** Compile a scope's named object queries. */
    static queries(
        objects: readonly ObjectType[],
        queries: Readonly<Record<string, ObjectQuery>> | undefined,
        chain: readonly string[],
    ): Record<string, Query> {
        return compileQueries(objects, queries, chain);
    }

    /** Connect to the object's methods at a serving service, speaking its release. */
    connect<Self extends ObjectType>(
        this: Self,
        service: Pick<Service, "package">,
        options: ClientOptions,
    ): Client<ObjectProcedures<Self>> {
        return createClient({ package: service.package, router: objectProcedures(this) }, options);
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
                `aggregate ${name} of ${measured.name} fills no ${aggregate.function} field of ${this.name}`,
            );
        }

        // require its readers to list every measured row: the field's guard, or the read methods' permissions
        const guard = this.fields[name].access?.read;
        const reads = Object.values(methodsOf(this))
            .filter((declared) => declared.kind === "get" || declared.kind === "list")
            .map((declared) => declared.permission);
        const readers = new Set(guard === undefined ? reads : [guard]);
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

    /** Match the objects living in one scope, as SQL. */
    inScope(scope: string): SQL {
        return eq(this.table[TABLE].column("scope"), scope);
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
            const name = kebabCase(field.relationAt(property));
            const isRelation = Object.hasOwn(this.policy.definition.relations, name);

            return isRelation && field.type === "reference" && field.target && !field.qualified
                ? [field.target()]
                : [];
        });
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
    scopesOf(chain: readonly string[]): string[] {
        const held = this.table[TABLE].column("scope").definition.schema;

        return this.inherited === undefined
            ? [aligned(chain, 0)]
            : chain.filter((scope) => held.safeParse(scope).success);
    }

    /** Decide whether the objects live in a scope. */
    livesIn(scope: ObjectReference): boolean {
        return (
            this.scope !== Scope.universe.id &&
            (this.scopes.length === 0 || this.scopes.some((type) => type.policy.is(scope)))
        );
    }

    /** Report whether a database copies the type's durable rows in a scope from their home. */
    async isCopy(snapshot: Snapshot, scope: string): Promise<boolean> {
        return (
            this.storage === "durable" &&
            (snapshot.database.copies(this.table) ||
                (await Replica.isCopied(snapshot, scope, this.table)))
        );
    }

    /** List the types projecting the objects into recipients' homes, refusing other sources. */
    get projections(): readonly ObjectType[] {
        // read the declared projections, refusing one projecting another type
        const projecting = this.projecting?.() ?? [];
        const other = projecting.find(
            (object) =>
                object.projected === undefined || !projected.sourceOf(object.projected).same(this),
        );
        if (other !== undefined) {
            throw new TypeError(`object ${other.name} projects no ${this.name}`);
        }

        return projecting;
    }

    /** Decide whether objects of another type attach to these objects. */
    attaches(child: ObjectType): boolean {
        return (
            this.attachments.some((attachment) => child.same(attachment.object)) ||
            (child.table === chunk && this.text.length > 0)
        );
    }

    /** Read a unique key duplicate as a conflict listing its fields, else keep the failure. */
    conflict(error: unknown): unknown {
        // read the violated key of the type's own table
        const definition = this.table[TABLE];
        const key = error instanceof DatabaseError ? error.key : undefined;
        if (key === undefined || key.table !== definition.sqlName) {
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

        // list each key column once, by its relation or its field
        const names = key.columns.map((column) => {
            const [field] = present(
                Object.entries(definition.columns).find(
                    ([, declared]) => declared.definition.name === column,
                ),
                `the field of ${this.name}'s column ${column}`,
            );

            return named.get(field) ?? field;
        });
        const listed = new Intl.ListFormat("en").format([...new Set(names)]);

        return new ServiceError("CONFLICT", {
            message: `another ${this.name} has the same ${listed}`,
            cause: error,
        });
    }

    /** The table mapping access reads the objects through, built on its first read. */
    get mapping(): TableMapping {
        const mapping = this.derived.mapping ?? ObjectAccess.mapping(this);
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
            ...(this.parent === undefined || this.parent.object === "any" ? [] : ["parentId"]),
            ...(ownerColumn === undefined ? [] : [ownerColumn]),
        ];
    }

    /** Read how stacks declare a type: a provisioned kind's handler, or the create input as is. */
    static #declarationOf(
        definition: ObjectDefinition<unknown, string, Readonly<Record<string, Method>>>,
    ): DeclarationHandler | undefined {
        // declare a provisioned kind through its handler
        if (definition.provisioned !== undefined) {
            return Provisioned.declaration(definition.provisioned.kind);
        }
        // declare a type from its create input with its values as declared
        else if (definition.declarable !== undefined && "from" in definition.declarable) {
            return {};
        }

        return undefined;
    }

    /** Read the schema of one stack declaration. */
    private declarationSchema(): schema.Schema | undefined {
        // read a declared schema, or none for a type no stack declares
        const declared = this.definition.declarable;
        if (declared === undefined) {
            return undefined;
        } else if ("schema" in declared) {
            return declared.schema;
        }

        // derive the declaration from the create input once
        const { supplied } = this;
        const fields = InputSchema.fields(this.input("create")).shape;
        const derived =
            this.derived.declaration ??
            schema.object(
                Object.fromEntries(
                    Object.entries(fields).filter(([field]) => !supplied.includes(field)),
                ),
            );
        this.derived.declaration = derived;

        return derived;
    }

    /** The column of the reference the objects are owned through, absent for unowned objects. */
    get ownerColumn(): string | undefined {
        const { owner } = this;
        if (owner === undefined) {
            return undefined;
        }

        return present(this.mapping.relations[owner], `the owner of ${this.name}`).column;
    }

    /** The permission listing the objects needs, absent without a list method. */
    get listing(): Permission | undefined {
        return methodPermission(this, "list");
    }

    /** The permission reading one object needs, the listing's without a get method, or none. */
    get reading(): Permission | undefined {
        return methodPermission(this, "get") ?? this.listing;
    }

    /** Whether a controller reconciles the objects to their desired generation. */
    get isControlled(): boolean {
        return this.traits.some((applied) => applied.trait === controlled);
    }

    /** Implement the type: attach method handlers, its controller and its stack handler. */
    implement<Self extends ObjectType, Collected = DeclarationOf<Self>, Resolved = Collected>(
        this: Self,
        implementation: ObjectImplementation<Self, Collected, Resolved> &
            (Self["table"] extends ManagedTable ? unknown : { readonly declaration?: never }),
    ): Self;
    /**
     * Implement the type, keeping its signature.
     *
     * @construct the copy shares this type's table, methods and policy, adding handlers.
     */
    implement(this: ObjectType, implementation: ObjectImplementation): ObjectType {
        // wrap each given method's handler
        const methods: Record<string, Method> = { ...this.methods };
        for (const [name, handler] of Object.entries(implementation.methods ?? {})) {
            const declared = methods[name];
            if (declared === undefined) {
                throw new TypeError(`object ${this.name} has no method ${name}`);
            } else if (handler !== undefined) {
                methods[name] = declared.handle(handler);
            }
        }

        // keep ephemeral and external objects off the controllers
        const { controller, declaration } = implementation;
        if (controller !== undefined && this.storage !== "durable") {
            throw new TypeError(`${this.storage} object ${this.name} takes no controller`);
        }

        // copy the object type with the handled methods, the controller and the declaration handler
        return this.with({
            methods,
            ...(controller === undefined ? {} : { controller }),
            ...(declaration === undefined ? {} : { declaration }),
        });
    }

    /** Describe the type as data: its package and written definition, refusing intrinsic types. */
    describe(): ObjectDescription {
        return encodeType(this);
    }

    /** Declare an object type from its description against the known declarations. */
    static of(description: ObjectDescription, known: Iterable<KnownDeclaration>): ObjectType {
        return ObjectType.#declare(description, DeclarationIndex.of(known));
    }

    /** Declare object types from descriptions against the known declarations. */
    static ofAll(
        descriptions: readonly ObjectDescription[],
        known: Iterable<KnownDeclaration>,
    ): ObjectType[] {
        // declare each type once the ones it refers to now are known
        const index = DeclarationIndex.of(known);
        const pending = new Map(descriptions.map((each) => [ObjectDescription.key(each), each]));
        const declared = new Map<string, ObjectType>();
        while (pending.size > 0) {
            const ready = [...pending].filter(([, each]) =>
                ObjectDescription.dependencies(each, index).every((key) => !pending.has(key)),
            );
            if (ready.length === 0) {
                throw new TypeError(
                    `object types ${[...pending.keys()].join(", ")} refer to each other now; refer to one lazily`,
                );
            }
            for (const [key, each] of ready) {
                declared.set(key, ObjectType.#declare(each, index));
                pending.delete(key);
            }
        }

        return descriptions.map((each) =>
            present(declared.get(ObjectDescription.key(each)), "the declared type"),
        );
    }

    /** Declare an object type from its description and add it to its lazy references' index. */
    static #declare(description: ObjectDescription, index: DeclarationIndex): ObjectType {
        // decode the definition and declare the type, its methods built
        const { methods, ...definition } = ObjectDescription.decode(description, index);
        const declared = declareObject(
            { ...definition, ...(methods === undefined ? {} : { methods: () => methods }) },
            { package: description.package },
        );

        // add the type for the lazy references that read it
        index.add(declared);

        return declared;
    }

    /** Copy the object type with some members changed, sharing its table. */
    with<Self extends ObjectType>(
        this: Self,
        changes: Partial<Pick<Self, "methods" | "controller" | "declaration" | "definition">>,
    ): Self;
    /**
     * Copy the object type with some members changed, sharing its table.
     *
     * @construct the copy keeps this signature, each changed member typed as the one it replaces.
     */
    with(
        changes: Partial<Pick<ObjectType, "methods" | "controller" | "declaration" | "definition">>,
    ): ObjectType {
        // copy the members, building the changed type's schemas and procedures afresh
        return Object.assign(inherit(prototypeOf(this)), this, changes, {
            derived: ObjectType.#underived(),
        });
    }

    /** Assemble an object type from its definition. */
    static assemble(
        owner: Package,
        definition: ObjectDefinition & {
            readonly table: Table;
            readonly methods: Readonly<Record<string, Method>>;
            readonly source: ObjectSource;
        },
    ): ObjectType {
        // list the fields by what callers may do with them
        const fields = Object.entries(definition.fields ?? {});
        const written = fields.filter(([, declared]) => declared.written).map(([name]) => name);
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
            storage: definition.storage ?? "durable",
            written,
            guarded,
            sensitive,
            text: texts,
        });
    }

    /** List object types with the chunk type their text fields need. */
    static served(objects: readonly ObjectType[]): ObjectType[] {
        const owners = objects.filter((object) => object.text.length > 0);

        return owners.length === 0 ? [...objects] : [...objects, Chunk.object(owners)];
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

    /** List the object types among a definition's scopes, leaving out the universe. */
    static #scopeTypes(scope: ObjectScope | readonly ObjectScope[]): ObjectType[] {
        const scopes: readonly ObjectScope[] = [scope].flat();

        return scopes.filter((type): type is ObjectType => type !== Scope.universe.id);
    }

    /** List the tables the objects need: the store's and text chunks beside durable rows. */
    #databaseTables(): readonly Table[] {
        // keep ephemeral and external rows in their table alone
        if (this.storage !== "durable") {
            return [this.table];
        }

        return [this.table, ...storeTables, ...(this.text.length === 0 ? [] : [chunk, chunkRun])];
    }

    /** Require valid methods, fields, aggregates, scopes, indexes and traits. */
    #requireValid(definition: ObjectTypeDefinition<Configuration>): void {
        // require the type's own members
        this.#requireMethods();
        this.#requireAggregates();
        this.#requireScopes();
        this.#requireIndexes();
        this.#requireOwner();
        this.#requireStorage(definition, this.traits);

        // let each trait validate its options
        for (const { trait, options } of this.traits) {
            trait.validate?.(options, this, definition);
        }
    }

    /** Require each conversion up to the release, and moved fields clear of current fields. */
    static #requireConversions(
        definition: ObjectDefinition<unknown, string, Readonly<Record<string, Method>>>,
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
                    `field ${definition.name}.${field} moved from ${previous}, which is a current field`,
                );
            }
        }
        Version.requireUpTo(definition.convert ?? {}, owner.version, definition.name);
    }

    /** Require client method names functions lack, and declared permissions. */
    #requireMethods(): void {
        // refuse client method names functions have
        const shadowed = Object.entries(methodsOf(this)).find(
            ([name, declared]) =>
                declared.isInternal !== true &&
                Object.getOwnPropertyNames(Function.prototype).includes(name),
        )?.[0];
        if (shadowed !== undefined) {
            throw new TypeError(
                `object ${this.name} has a method ${shadowed}, which functions have`,
            );
        }

        // require declared permissions and valid methods
        for (const declared of Object.values(methodsOf(this))) {
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
                    `aggregate ${name} of ${this.name} fills no ${aggregate.function} field of its holder`,
                );
            } else if (holder !== "any") {
                holder.requireAggregate(this, name, aggregate);
            }
        }

        // refuse aggregate fields in method inputs
        for (const [name, declared] of Object.entries(methodsOf(this))) {
            const { input } = declared;
            const variants = input instanceof schema.DiscriminatedUnion ? input.options : [input];
            const written = variants.flatMap((variant) =>
                variant instanceof schema.Object ? Object.keys(variant.shape) : [],
            );
            const aggregated = written.find((field) => this.fields[field]?.aggregate !== undefined);
            if (aggregated !== undefined) {
                throw new TypeError(
                    `method ${name} of ${this.name} writes aggregate field ${aggregated}`,
                );
            }
        }
    }

    /** Require readable scope types and valid local object scopes. */
    #requireScopes(): void {
        // refuse a containment grant on a type living in the universe alone, which contains no principal
        const definition = this.policy.definition;
        const isUniversal = this.scope === Scope.universe.id && definition.scope !== true;
        const contained = Object.entries(definition.permissions).find(([, expression]) =>
            grantsContainment(expression),
        );
        if (isUniversal && contained !== undefined) {
            throw new TypeError(
                `object ${this.name} lives in the universe, which contains no principal, so contained() in its ${contained[0]} permission admits no one`,
            );
        }

        // require each scope type to be a readable scope
        for (const scope of this.storage === "local" ? [] : this.scopes) {
            // refuse a type that is no scope
            if (scope.policy.definition.scope !== true) {
                throw new TypeError(
                    `object ${this.name} lives in ${scope.name}, which is no scope`,
                );
            }
            // refuse a scope without the read permission
            else if (!scope.permissions.includes(SCOPE_READ)) {
                throw new TypeError(
                    `object ${this.name} lives in ${scope.name}, which declares no ${SCOPE_READ} permission`,
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
            ([property, each]) => kebabCase(each.relationAt(property)) === owner,
        )?.[1];
        if (
            owner !== undefined &&
            (field?.type !== "reference" || field.target === undefined || field.qualified)
        ) {
            throw new TypeError(
                `object ${this.name} is owned through ${owner}, which no reference relates`,
            );
        }
    }

    /** Refuse durable-only declarations, writes and work on ephemeral, external, local objects. */
    #requireStorage(definition: ObjectDefinition, traits: readonly TraitInstance[]): void {
        const durable = durableDeclarations(definition, traits);

        // refuse lingering outside memory
        if (this.storage !== "ephemeral" && definition.linger !== undefined) {
            throw new TypeError(`object ${this.name} lingers without being ephemeral`);
        }
        // refuse durable declarations and writes on external objects
        else if (this.storage === "external") {
            const writing = Object.entries(methodsOf(this)).find(
                ([, declared]) => declared.mutates,
            );
            if (durable.length > 0) {
                throw new TypeError(`external object ${this.name} takes no ${durable[0]}`);
            } else if (writing !== undefined) {
                throw new TypeError(`external object ${this.name} writes in method ${writing[0]}`);
            }
        }
        // refuse durable declarations and external work on ephemeral objects
        else if (this.storage === "ephemeral") {
            const external = Object.entries(methodsOf(this)).find(
                ([, declared]) => declared.prepare !== undefined || Method.settles(declared),
            );
            if (durable.length > 0) {
                throw new TypeError(`ephemeral object ${this.name} takes no ${durable[0]}`);
            } else if (external !== undefined) {
                throw new TypeError(
                    `ephemeral object ${this.name} does no external work in method ${external[0]}`,
                );
            }
        }
        // refuse durable declarations, work beyond the client and several scopes on local objects
        else if (this.storage === "local") {
            this.#requireLocal(durable);
        }
    }

    /** Refuse durable declarations, off-client methods and several scope types on local objects. */
    #requireLocal(durable: readonly string[]): void {
        // refuse prepared and settled work, and a record method declared to run on the server alone
        const remote = Object.entries(methodsOf(this)).find(
            ([, declared]) =>
                (!declared.isPredicted && declared.kind !== "custom") ||
                declared.prepare !== undefined ||
                Method.settles(declared),
        );
        if (durable.length > 0) {
            throw new TypeError(`local object ${this.name} takes no ${durable[0]}`);
        } else if (remote !== undefined) {
            throw new TypeError(
                `local object ${this.name} runs method ${remote[0]} off the client`,
            );
        } else if (ObjectScope.isList(this.scope) || this.scope === Scope.universe.id) {
            throw new TypeError(`local object ${this.name} lives in one scope type`);
        }
    }
}

/** An aggregate of objects their holder keeps in one field. */
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

/** Build an object type's constraints over its derived columns, as its declaration would. */
export type ObjectConstraints = Bivariant<(columns: ColumnMap) => readonly TableConstraint[]>;

/** A type of attachments a host takes. */
export interface Attachment<Object extends ObjectType = ObjectType> {
    /** The attachment type, nested in any parent. */
    readonly object: Object;
    /** The host permission whose holders attach. */
    readonly by: string;
}

/** An object definition deriving its table from fields and traits. */
export type ObjectFieldsDefinition<
    Name extends string,
    Fields extends Readonly<Record<string, Field>>,
    Permissions extends string,
    Methods extends Readonly<Record<string, Method>>,
    Scope extends ObjectScope | readonly ObjectScope[],
    Identity extends string = Name,
    Storage extends ObjectStorage = "durable",
    Granted extends string = Permissions,
    Plural extends string = string,
> = Omit<
    ObjectDefinition<unknown, Permissions, Methods, Scope, Granted>,
    "name" | "plural" | "identity" | "constraints" | "storage" | keyof ObjectTraits
> & {
    /** The singular name, unique within the declaring package. */
    readonly name: Name;
    /** The plural name. */
    readonly plural: Plural;
    /** The prefix of the objects' identifiers, the name when absent. */
    readonly identity?: Identity;
    /** The fields each object has, by property name. */
    readonly fields: Fields;
    /** Where the objects live. */
    readonly storage?: Storage;
    /** Indexes, unique constraints and checks over the derived columns. */
    readonly constraints?: (
        columns: ConstraintColumns<
            Name,
            ScopeIdentityOf<Scope>,
            ObjectFields<Fields>,
            Identity,
            Storage
        >,
    ) => readonly TableConstraint[];
};

/** The traits whose options type an object's table, methods or declarations, one each. */
type TypedTrait =
    | "nested"
    | "recoverable"
    | "versioned"
    | "controlled"
    | "declarable"
    | "suspendable"
    | "adoptable"
    | "tracked"
    | "shareable"
    | "provisioned"
    | "bindable"
    | "attachments"
    | "presentable"
    | "orderable"
    | "key";

/** The trait options a definition sets, without the absent ones. */
type TraitOptions<Traits> = {
    readonly [Key in keyof Traits as [Traits[Key]] extends [undefined] ? never : Key]: Traits[Key];
};

/** The traits of an object definition, without the definition's other options. */
export type TraitsOf<Definition> = Pick<Definition, keyof Definition & keyof ObjectTraits>;

/** The traits as a derived table reads them, a nested parent by its identity. */
export type TableTraitsOf<Traits> = {
    readonly [Key in keyof Traits]: Key extends "nested"
        ? Traits[Key] extends { readonly in: infer Parent }
            ? Omit<Traits[Key], "in"> & {
                  readonly in: Parent extends ObjectType ? IdentityOf<Parent> : Parent;
              }
            : Traits[Key]
        : Traits[Key];
};

/** The fields a definition's objects have: its own, and those its sharing, kind and traits add. */
type DefinedFields<
    Fields,
    Sharing,
    IsScope extends boolean,
    Provisioning,
    Presenting,
    Ordered,
> = ObjectFields<
    Fields &
        RoleFieldsOf<Sharing, IsScope> &
        ProvisionedFieldsOf<Provisioning> &
        PresentationFieldsOf<Presenting> &
        OrderedFieldsOf<Ordered>
>;

/** The traits a definition's objects take: its own, and those its kind adds. */
type DefinedTraits<Traits, Provisioning> = TraitsOf<Traits> & ProvisionedTraitsOf<Provisioning>;

/** The permissions a definition's objects have. */
type DefinedPermissions<Permissions, Sharing, Provisioning, Traits> =
    | Permissions
    | RolePermissionsOf<Sharing>
    | ProvisionedPermissionOf<Provisioning>
    | TraitPermissionsOf<DefinedTraits<Traits, Provisioning>>;

/** The shape of one stack declaration of a declarable object. */
export type StackDeclarationOf<Traits> =
    TraitOf<Traits, "declarable"> extends DeclarableDefinition<infer Declared> ? Declared : never;

/** The permissions traits derive for their methods, granted only by roles unless declared. */
export type TraitPermissionsOf<Traits> =
    | ([TraitOf<Traits, "controlled">] extends [undefined] ? never : "control")
    | (TraitOf<Traits, "controlled"> extends { readonly approval: true } ? "approve" : never)
    | ([TraitOf<Traits, "declarable">] extends [undefined] ? never : "declare")
    | ([TraitOf<Traits, "expiring">] extends [undefined] ? never : "expire")
    | ([TraitOf<Traits, "bindable">] extends [undefined] ? never : "bind");

/** The methods an object's traits derive beside its declared ones. */
export type TraitMethods<Fields, Traits> = TransitionMethodMap<Fields> &
    RecoverableMethodMap<TraitOf<Traits, "recoverable">> &
    ControlledMethodMap<TraitOf<Traits, "controlled">> &
    NestedMethodMap<TraitOf<Traits, "nested">> &
    ShareableMethodMap<SharingGateOf<TraitOf<Traits, "shareable">>> &
    SuspendableMethodMap<GateOf<TraitOf<Traits, "suspendable">>> &
    DeclarableMethodMap<TraitOf<Traits, "declarable">> &
    BindableMethodMap<TraitOf<Traits, "bindable">> &
    AdoptableMethodMap<GateOf<TraitOf<Traits, "adoptable">>> &
    TrackedMethodMap<TraitOf<Traits, "tracked">> &
    TextMethodMap<Fields>;

/** The methods of an object type, built on its table and needing only its permissions. */
export type ObjectMethods<
    Definition extends Table,
    Methods,
    Granted extends string = string,
    Written extends string = string,
> = {
    /** Build the operations callers may execute, keyed by method name. */
    readonly methods?: (
        method: MethodBuilder<Definition, Written>,
    ) => Methods & PermittedMethods<Methods, NoInfer<Granted>>;
};

/** The methods whose permissions the type grants, others typed never. */
type PermittedMethods<Methods, Granted extends string> = {
    readonly [Name in keyof Methods]: Methods[Name] extends { readonly permission: infer Needed }
        ? [Needed] extends [Granted]
            ? Methods[Name]
            : never
        : never;
};

/** Declare an object type from its fields. */
export function defineObject<
    const Name extends string,
    Fields extends Readonly<Record<string, Field>>,
    Permissions extends string = never,
    const Methods extends Readonly<Record<string, Method>> = {},
    const Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope,
    const Nested extends NestedDefinition | undefined = undefined,
    const Recoverable extends
        | RecoverableDefinition<NoInfer<Permissions> | RolePermission>
        | undefined = undefined,
    const Versioned extends VersionsDefinition | undefined = undefined,
    const Controlled extends ControlledDefinition | undefined = undefined,
    const Declarable extends DeclarableDefinition | undefined = undefined,
    const Suspendable extends Gated<NoInfer<Permissions> | RolePermission> | undefined = undefined,
    const Adoptable extends Gated<NoInfer<Permissions> | RolePermission> | undefined = undefined,
    const Bindable extends true | Gated<NoInfer<Permissions> | RolePermission> | undefined =
        undefined,
    const Tracked extends TrackedDefinition<NoInfer<Permissions> | RolePermission> | undefined =
        undefined,
    const Identity extends string = Name,
    const Storage extends ObjectStorage = "durable",
    const Sharing extends ShareableDefinition<NoInfer<Permissions> | RolePermission> | undefined =
        undefined,
    const IsScope extends boolean = false,
    const Provisioning extends ProvisionedDefinition | undefined = undefined,
    const Plural extends string = string,
    const Attachments extends readonly Attachment[] = [],
    const Key extends schema.Schema<string> | undefined = undefined,
    const Presenting extends PresentableDefinition | undefined = undefined,
    const Ordering extends OrderableDefinition | undefined = undefined,
    Traits = TraitOptions<{
        readonly key: Key;
        readonly nested: Nested;
        readonly recoverable: Recoverable;
        readonly versioned: Versioned;
        readonly controlled: Controlled;
        readonly declarable: Declarable;
        readonly suspendable: Suspendable;
        readonly adoptable: Adoptable;
        readonly bindable: Bindable;
        readonly tracked: Tracked;
    }>,
>(
    definition: Omit<
        ObjectFieldsDefinition<
            Name,
            Fields,
            Permissions,
            Methods,
            Scope,
            Identity,
            Storage,
            DefinedPermissions<Permissions, Sharing, Provisioning, Traits>,
            Plural
        >,
        "methods"
    > &
        ObjectMethods<
            ObjectTable<
                Name,
                ScopeIdentityOf<Scope>,
                NoInfer<
                    DefinedFields<Fields, Sharing, IsScope, Provisioning, Presenting, Ordering>
                >,
                NoInfer<TableTraitsOf<DefinedTraits<Traits, Provisioning>>>,
                Identity,
                Storage
            >,
            Methods,
            DefinedPermissions<Permissions, Sharing, Provisioning, Traits>,
            NoInfer<
                WrittenField<
                    DefinedFields<Fields, Sharing, IsScope, Provisioning, Presenting, Ordering>
                >
            >
        > &
        Omit<ObjectTraits, TypedTrait> & {
            /** The parent object type, or "self" for a tree. */
            readonly nested?: Nested;
            /** Deleted objects stay restorable for a window. */
            readonly recoverable?: Recoverable;
            /** The objects are immutable, numbered versions of their parent. */
            readonly versioned?: Versioned;
            /** A controller reconciles the objects. */
            readonly controlled?: Controlled;
            /** Stacks declare the objects. */
            readonly declarable?: Declarable;
            /** Holders of the permission suspend and resume a scope object. */
            readonly suspendable?: Suspendable;
            /** Permission holders detach a declared record, and adopt a direct record into one. */
            readonly adoptable?: Adoptable;
            /** Installations bind to the objects, and permission holders bind one at a time. */
            readonly bindable?: Bindable;
            /** Every change stays in the log, grouped into activities. */
            readonly tracked?: Tracked;
            /** Project another type's rows into the objects, each with its own state. */
            readonly projected?: ProjectedDefinition;
            /** The types projecting the objects into their recipients' homes. */
            readonly projections?: () => readonly ObjectType[];
            /** How callers share the objects: through roles, or the type's own relations. */
            readonly shareable?: Sharing;
            /** Whether the objects are scopes containing other objects. */
            readonly isScope?: IsScope;
            /** Whether each object is a partition, with contents living apart from its parent's. */
            readonly partition?: true;
            /** The kind whose resources the objects are. */
            readonly provisioned?: Provisioning;
            /** The attachments the objects take. */
            readonly attachments?: Attachments;
            /** The objects' natural key, which callers and sources identify them by. */
            readonly key?: Key;
            /** How pickers, mentions and titles show the objects. */
            readonly presentable?: Presenting;
            /** How the objects are ordered among their siblings. */
            readonly orderable?: Ordering;
        },
    module?: ModuleMetadata,
): ObjectType<{
    readonly name: Name;
    readonly plural: Plural;
    readonly identity: Identity;
    readonly table: ObjectTable<
        Name,
        ScopeIdentityOf<Scope>,
        DefinedFields<Fields, Sharing, IsScope, Provisioning, Presenting, Ordering>,
        TableTraitsOf<DefinedTraits<Traits, Provisioning>>,
        Identity,
        Storage
    >;
    readonly fields: DefinedFields<Fields, Sharing, IsScope, Provisioning, Presenting, Ordering>;
    readonly methods: Methods &
        ProvisionedMethodsOf<Provisioning> &
        TraitMethods<
            ObjectFields<Fields>,
            TraitsOf<Traits> & {
                readonly shareable: ProvisionedSharingOf<Provisioning, Sharing>;
            } & ProvisionedTraitsOf<Provisioning>
        >;
    readonly permissions: DefinedPermissions<Permissions, Sharing, Provisioning, Traits>;
    readonly scope: ScopeIdentityOf<Scope>;
    readonly parent: Nested extends { readonly in: infer Parent }
        ? Parent extends ObjectType
            ? IdentityOf<Parent>
            : Parent extends "self" | "any"
              ? Parent
              : undefined
        : undefined;
    readonly attachments: IdentityOf<Attachments[number]["object"]>;
    readonly storage: Storage;
    readonly declared: StackDeclarationOf<Traits & ProvisionedTraitsOf<Provisioning>>;
}>;
/** Declare an object type over a table another package owns. */
export function defineObject<
    Definition extends Table,
    Declared = never,
    Permissions extends string = never,
    const Methods extends Readonly<Record<string, Method>> = {},
    const Scope extends ObjectScope | readonly ObjectScope[] = ObjectScope,
>(
    definition: Omit<
        ObjectDefinition<Declared, Permissions, Methods, Scope>,
        | "methods"
        | "identity"
        | "nested"
        | "recoverable"
        | "versioned"
        | "controlled"
        | "adoptable"
        | "shareable"
        | "suspendable"
        | "tracked"
        | "attachments"
        | "constraints"
        | "aggregates"
    > &
        ObjectMethods<Definition, Methods, Permissions> & {
            readonly [INTRINSIC]: Intrinsic<Definition>;
        },
    module?: ModuleMetadata,
): ObjectType<{
    readonly name: string;
    readonly plural: string;
    readonly identity: string;
    readonly table: Definition;
    readonly fields: {};
    readonly methods: Methods;
    readonly permissions: Permissions;
    readonly scope: ScopeIdentityOf<Scope>;
    readonly parent: undefined;
    readonly attachments: never;
    readonly storage: "durable";
    readonly declared: Declared;
}>;
/**
 * Declare an object type.
 *
 * @construct the table, fields, methods, traits and scope derive as the signatures above read them.
 */
export function defineObject(input: ObjectInput, module?: ModuleMetadata): ObjectType {
    return declareObject(input, module);
}

/** A definition as `defineObject` takes it, with methods built by a builder function. */
type ObjectInput = Omit<ObjectDefinition, "methods"> &
    ObjectMethods<Table, Readonly<Record<string, Method>>> & {
        readonly [INTRINSIC]?: Intrinsic;
    };

/** Declare an object type from a definition, deriving its table, traits and methods. */
function declareObject(input: ObjectInput, module: ModuleMetadata | undefined): ObjectType {
    // build the declared methods on the object's table
    const owner = ModuleMetadata.require(module, "defineObject").package;
    const { methods: build, ...written } = input;
    const declaration =
        written.fields === undefined ? written : { ...written, fields: keyFields(written.fields) };
    const built = build?.(method);

    // collect the traits, adding what the roles define
    const definition = Orderable.expand(
        Presentable.expand(
            Roles.expand(
                Provisioned.expand({
                    ...declaration,
                    ...(built === undefined ? {} : { methods: built }),
                }),
            ),
        ),
    );
    const intrinsic = definition[INTRINSIC];
    const traits = traitsOf(definition);

    // derive the table
    const packageId = (definition.represents?.package ?? owner).id;
    const table = intrinsic?.table ?? deriveTable(definition, traits, packageId, module);

    // add trait methods
    const declared: Readonly<Record<string, Method>> = definition.methods ?? {};
    const derived = traits.map(({ trait, options }) => trait.methods(options, declared));
    const clash = derived.flatMap(Object.keys).find((name) => Object.hasOwn(declared, name));
    if (clash !== undefined) {
        throw new TypeError(
            `object ${definition.name} declares method ${clash}, which a trait derives`,
        );
    }
    const methods = Object.fromEntries([
        ...Object.entries(declared),
        ...derived.flatMap((each) => Object.entries(each)),
    ]);

    return ObjectType.assemble(owner, {
        ...definition,
        table,
        methods,
        source: { ...written, ...(built === undefined ? {} : { methods: built }) },
    });
}

/** List the traits a definition opts into with their options, none for an intrinsic type. */
function traitsOf(
    definition: ObjectDefinition & { readonly [INTRINSIC]?: Intrinsic },
): TraitInstance[] {
    return definition[INTRINSIC] === undefined
        ? TRAITS.flatMap((trait) => {
              const options = trait.options(definition);

              return options === undefined ? [] : [{ trait, options }];
          })
        : [];
}

/** The name of a field whose values need their own read permission. */
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

/** The field a creation fills with the calling principal, written only internally. */
export type CallerField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly isCallerFilled: true }
        ? Name
        : never;
}[keyof Fields & string];

/** The name of a field callers write. */
export type WrittenField<Fields extends Readonly<Record<string, Field>>> = {
    [Name in keyof Fields & string]: Fields[Name] extends { readonly written: false }
        ? never
        : Name;
}[keyof Fields & string];

/** Key each identifier reference under `<relation>Id`, declaring its relation. */
function keyFields(fields: Readonly<Record<string, Field>>): Readonly<Record<string, Field>> {
    return Object.fromEntries(
        Object.entries(fields).map(([name, field]) =>
            field.isReference ? [`${name}Id`, field.relate(name)] : [name, field],
        ),
    );
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

    // search the permission's expression
    const expression = permissions[name];

    return expression !== undefined && admits(permissions, expression, relation, permission, seen);
}

/** Decide whether an expression admits a related permission's holders, via unions and names. */
function admits(
    permissions: Readonly<Record<string, AccessExpression>>,
    expression: AccessExpression,
    relation: string,
    permission: string,
    seen: Set<string>,
): boolean {
    // search each branch of a union
    if (expression.kind === "union") {
        return expression.expressions.some((branch) =>
            admits(permissions, branch, relation, permission, seen),
        );
    }
    // follow a named permission
    else if (expression.kind === "permission") {
        return covers(permissions, expression.name, relation, permission, seen);
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
function methodsOf(object: ObjectType): Readonly<Record<string, Method>> {
    return object.methods;
}

/** Report whether an expression grants a scope's containment set anywhere in it. */
function grantsContainment(expression: AccessExpression): boolean {
    switch (expression.kind) {
        case "contained":
            return true;
        case "union":
        case "intersection":
            return expression.expressions.some(grantsContainment);
        case "exclusion":
            return grantsContainment(expression.include) || grantsContainment(expression.exclude);
        case "none":
        case "relation":
        case "permission":
        case "resource":
        case "context":
        case "through":
        case "granters":
        case "readers":
            return false;
    }
}

/** List the durable-only declarations of a definition: its durable traits and keys. */
function durableDeclarations(
    definition: ObjectDefinition,
    traits: readonly TraitInstance[],
): string[] {
    const keys = ["audited", "inherited", "isScope", "partition", "aggregates", "indexes"] as const;

    return [
        ...traits
            .filter(({ trait }) => trait.isDurable)
            .map(({ trait }) => present(trait.key, "the key of a durable trait")),
        ...keys.filter((key) => definition[key] !== undefined && definition[key] !== false),
    ];
}

/** Read the parent of a nested type, absent for a type that is not nested. */
function nestedParent(
    type: ObjectType,
    definition: ObjectDefinition<unknown, string, Readonly<Record<string, Method>>>,
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

/** Read how long an ephemeral type's objects outlive their session, in milliseconds. */
function lingerOf(
    definition: ObjectDefinition<unknown, string, Readonly<Record<string, Method>>>,
): number {
    // take the default without a declared linger
    if (definition.linger === undefined) {
        return LINGER_MILLISECONDS;
    }

    // require a valid declared linger
    Duration.require(definition.linger, `linger of ${definition.name}`);

    return Duration.milliseconds(definition.linger);
}

/** Read the permission a type's standard method of a kind needs, absent without one. */
function methodPermission(object: ObjectType, kind: MethodKind): Permission | undefined {
    const standard = Object.values(object.methods).find((declared) => declared.kind === kind);

    return standard === undefined ? undefined : object.permission(standard.permission);
}
