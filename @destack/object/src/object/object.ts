import { AuditTarget } from "@destack/audit";
import {
    Authorizer,
    Policy,
    none,
    type AccessExpression,
    type Elevation,
    type RelationInput,
    type PolicySubject,
    principal,
    relationsOf,
    universe,
    type Permission,
    type TableMapping,
} from "@destack/access";
import {
    Scope,
    type ObjectReference,
    type ObjectTypeReference,
    type Query,
    type Subject,
} from "@destack/sync";
import type * as sync from "@destack/sync";
import {
    Change,
    type ColumnMap,
    Snapshot,
    Expression,
    Condition,
    eq,
    inArray,
    type DatabaseConnection,
    type RowImage,
    type Row,
    type SQL,
    TABLE,
    type Table,
    type TableConstraint,
    type Tree,
} from "@destack/db";
import {
    aligned,
    canonicalize,
    Duration,
    found,
    Identifier,
    present,
    schema,
    Version,
    type JsonObject,
} from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Claim, Directory, ObjectClaims } from "@destack/directory";
import type { AuditAction } from "@destack/audit";
import { ModuleMetadata, type Package, type PackageId } from "@destack/package";
import type { Field, ObjectFields, TextField } from "../field/field.ts";
import {
    Call,
    type Bivariant,
    type Handler,
    type HandlersOf,
    type Lifecycle,
} from "../method/call.ts";
import type { Calls, RowSchema } from "../method/procedure.ts";
import { method, Method, type MethodBuilder, type MethodKind } from "../method/method.ts";
import type { Client, Service } from "@destack/service";
import { createClient, type ClientOptions } from "@destack/service/client";
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
    type OpenQueryOptions,
    type ObjectQuery,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { compile, compileQueries } from "../query/query.ts";
import { record } from "../trait/record.ts";
import {
    declarable,
    detachable,
    type DeclarableDefinition,
    type DeclarableMethodMap,
    type DetachableMethodMap,
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
import type { GateOf, Gated, Trait, TraitObject, TraitPolicy } from "../trait/trait.ts";
import { INTRINSIC, type Intrinsic } from "./intrinsic.ts";
import { serverTables } from "../stack/db.ts";
import type { ObjectController } from "./controller.ts";
import type { DeclarationOf, ManagedTable, ObjectDeclaration } from "../server/stack.ts";
import { camelCase, kebabCase } from "./name.ts";
import { deriveTable, type ConstraintColumns, type ObjectTable, type TraitOf } from "./table.ts";

/** The schemas of the identities object types identify by, one per identity, shared by derived copies of a type. */
const IDENTIFIERS = new Map<string, ReturnType<typeof schema.identifier>>();

/** The default ephemeral linger, in milliseconds: 10 s spans two 1–3 s reconnects. */
const LINGER_MILLISECONDS = 10_000;

/** The method kinds writing records. */
const RECORD_KINDS: ReadonlySet<string> = new Set(["create", "update", "delete", "updateMany"]);

/** The permission on a scope's own object that shows the scope. */
export const SCOPE_READ = "read";

/** The permission to act as the principal an object stands for, such as the cell serving a space's zone. */
export const REPRESENT = "represent";

/** The permission to copy the access rows of the scopes above a scope: on the caller's own object living in it, or on the scope's own object. */
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
    detachable,
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

/** A scope as signatures keep it: "universe", its scope type's identity, or any string for several scope types. */
export type ScopeIdentityOf<Scope> = Scope extends readonly unknown[]
    ? string
    : Scope extends ObjectType
      ? IdentityOf<Scope>
      : Scope;

/** The permissions an object type declares: names roles grant, or named expressions derived from relations. */
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
    /** The rules after which the system removes the objects. */
    readonly expiring?: readonly ExpiryRule[];
    /** Project another type's rows into the objects, each its own object with its own state. */
    readonly projected?: ProjectedDefinition;
    /** The types projecting the objects into their recipients' homes, as their `projected` names them. */
    readonly projections?: () => readonly ObjectType[];
    /** The objects' natural key, which callers and sources name them by, instead of generated identifiers. */
    readonly key?: schema.Schema<string>;
    /** The scopes inside each object's scope copy the objects, those matching the condition when given. */
    readonly inherited?: { readonly where?: Condition };
    /** The objects are immutable, numbered versions of their parent. */
    readonly versioned?: VersionsDefinition;
    /** A system controller reconciles the objects. */
    readonly controlled?: ControlledDefinition;
    /** Installations bind to the objects, becoming their consumers while live deployments capture them, or while the holders of a permission bind them one at a time. */
    readonly bindable?: true | Gated<Permissions>;
    /** The objects are a kind's resources, which its providers provision. */
    readonly provisioned?: ProvisionedDefinition;
    /** Stacks declare the objects. */
    readonly declarable?: DeclarableDefinition<Declared>;
    /** Holders of the permission detach a declared record from its declaration. */
    readonly detachable?: Gated<Permissions>;
    /** Holders of the permission share the objects. */
    readonly shareable?: ShareableDefinition<Permissions>;
    /** Holders of the permission suspend and resume a scope object. */
    readonly suspendable?: Gated<Permissions>;
    /** Every change stays in the log, grouped into activities. */
    readonly tracked?: TrackedDefinition<Permissions>;
    /** Whether the audit also records reads. */
    readonly audited?: { readonly reads: true };
    /** How pickers, mentions and titles show the objects: a title, a subtitle, an icon, an accent and a cover. */
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
    /** Attributes permission expressions read. */
    readonly attributes?: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** Further relations kept in relationships. */
    readonly relations?: Readonly<Record<string, ObjectRelationInput>>;
    /** Permission names roles grant, or named expressions derived from relations. */
    readonly permissions?: ObjectPermissions<Permissions>;
    /** Permissions only their expressions grant, never roles. */
    readonly reserved?: readonly NoInfer<Granted>[];
    /** Sensitive permissions that apply only after the authentication each names. */
    readonly elevated?: Readonly<Partial<Record<NoInfer<Granted>, Elevation>>>;
    /** Permissions available while the object's scope is suspended. */
    readonly administration?: readonly NoInfer<Granted>[];
    /** An access-owned type whose identity the objects take. */
    readonly represents?: Policy;
    /** Whether the objects are scopes containing other objects. */
    readonly isScope?: boolean;
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
    /** The fields each release computes from stored rows and earlier callers' inputs, by the release introducing them. */
    readonly convert?: Readonly<Record<Version, Readonly<Record<string, Expression>>>>;
    /** How long ephemeral objects outlive their session, 10 s by default. */
    readonly linger?: Duration;
}

/** Where objects live: durable in the scope's database, ephemeral in instances' memory, or external in immutable files read per scope. */
export type ObjectStorage = "durable" | "ephemeral" | "external";

/** How an object type's objects are declared, deleted, expired, versioned, tracked and converted across releases. */
export interface ObjectLifecycle<Declared = unknown, Permissions extends string = string> {
    /** The schema of one declaration in a stack, when stacks may declare the objects. */
    readonly declarationSchema: schema.Schema<Declared> | undefined;
    /** How deleted objects stay restorable, for recoverable objects. */
    readonly recoverable: RecoverableDefinition<Permissions> | undefined;
    /** When the system removes the objects, for expiring objects. */
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

/** A definition as an object type retains it, with its identity, storage and field lists resolved. */
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
};

/** A unique index the key index keeps across databases. */
export interface ObjectIndex {
    /** The indexed fields, in key order. */
    readonly on: readonly string[];
    /** Whether each key names at most one object. */
    readonly unique: true;
    /** The scope type the keys are unique within: the universe or a type enclosing the objects, read lazily as references are. */
    readonly across: () => ObjectType | typeof universe;
    /** The namespace the keys share with other object types' indexes of the same name, the type's own when absent. */
    readonly namespace?: string;
}

/** An object as its readers see it: its logged fields and texts, guarded fields absent where a reader may not read them. */
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

/** An object type's signature: its table, fields, methods, permissions, scope, parent, storage and declarations. */
export interface ObjectConfiguration {
    /** The singular name. */
    readonly name: string;
    /** The plural name, which names the relation from a parent or referenced type. */
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
    /** The scope containing the objects: "universe", its scope type's identity, or any string for several scope types. */
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
    /** The singular name. */
    readonly name: Configuration["name"];
    /** The prefix of the objects' identifiers. */
    readonly identity: Configuration["identity"];
    /** The plural name. */
    readonly plural: Configuration["plural"];
    /** The table with one record per object. */
    readonly table: Configuration["table"];
    /** The scope types the objects live in, none for any scope. */
    readonly scope: ObjectScope | readonly ObjectScope[];
    /** The scope types the objects live in, none for universe objects and objects in any scope. */
    readonly scopes: readonly ObjectType[];
    /** The object's relations and permissions, evaluated by access. */
    readonly policy: Policy;
    /** The fields each object has, for objects declared by fields. */
    readonly fields: Configuration["fields"];
    /** The fields needing their own read permission. */
    readonly guarded: readonly string[];
    /** The fields callers write. */
    readonly written: readonly string[];
    /** The columns with sensitive values. */
    readonly sensitive: readonly string[];
    /** The text fields, kept in chunks. */
    readonly text: readonly string[];
    /** Whether reads of the objects are audited. */
    readonly isReadAudited: boolean;
    /** The rows the scopes inside each object's scope copy, for inherited objects. */
    readonly inherited: { readonly where?: Condition } | undefined;
    /** The ancestor index of a tree of this object type. */
    readonly tree: Tree | undefined;
    /** The tables a database with the object needs, access tables included. */
    readonly tables: readonly Table[];
    /** The permission names callers may have on the object. */
    readonly permissions: readonly string[];
    /** The operations callers may execute, keyed by method name. */
    readonly methods: Configuration["methods"];
    /** The system controller reconciling the objects with work waiting. */
    readonly controller?: ObjectController;
    /** The roles the objects are shared through, absent for objects shared otherwise or not at all. */
    readonly roles: RolesDefinition | undefined;
    /** The kind whose resources the objects are, absent for other objects. */
    readonly provisioned: ProvisionedDefinition | undefined;
    /** How a stack's declarations of the objects become their managed records. */
    readonly declaration: ObjectDeclaration | undefined;
    /** The type whose rows the objects project, absent for objects projecting none. */
    readonly projected: ProjectedDefinition | undefined;
    /** Read the types projecting the objects into their recipients' homes, as the definition declares them. */
    readonly projecting: (() => readonly ObjectType[]) | undefined;
    /** The objects' natural key, absent for objects with generated identifiers. */
    readonly keyed: schema.Schema<string> | undefined;
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
    /** The aggregates of these objects their holder keeps, by field name. */
    readonly aggregates: Readonly<Record<string, ObjectAggregate>>;
    /** The attachments the objects take. */
    readonly attachments: readonly Attachment[];
    /** The traits the objects take, with their options. */
    readonly traits: readonly TraitInstance[];
    /** The unique indexes the key index keeps across databases, by name. */
    readonly indexes: Readonly<Record<string, ObjectIndex>>;
    /** How pickers, mentions and titles show the objects, absent for a type presenting none. */
    readonly presentation: Presentation | undefined;
    /** How the objects are ordered among their siblings, absent for a type ordering none. */
    readonly ordering: Ordering | undefined;
    /** The fields full-text search ranks, lists filter and lists order the objects by, absent for a type declaring none. */
    readonly search: ObjectSearch | undefined;
    /** Where the objects live. */
    readonly storage: Configuration["storage"];
    /** How long ephemeral objects outlive their session, in milliseconds. */
    readonly linger: number | undefined;
    /** Where access finds objects stored in a table another package owns. */
    readonly intrinsic: Omit<TableMapping, "policy"> | undefined;
    /** How the objects are declared, deleted, expired, versioned, tracked and converted across releases. */
    readonly lifecycle: ObjectLifecycle<Configuration["declared"], Configuration["permissions"]>;

    /** Retain a definition with its table, methods and traits, and assemble its policy. */
    constructor(
        owner: Package,
        definition: ObjectTypeDefinition<Configuration>,
        traits: readonly TraitInstance[],
        intrinsic?: Omit<TableMapping, "policy">,
    ) {
        // require a logged table, and retain identity and scopes
        ObjectType.#requireLogged(definition.table);
        this.package = owner;
        this.name = definition.name;
        this.identity = definition.identity;
        this.plural = definition.plural;
        this.table = definition.table;
        this.scope = definition.scope ?? [];
        this.scopes = ObjectType.#scopeTypes(this.scope);

        // retain the lifecycle, projection and natural key the definition declares
        this.lifecycle = ObjectType.#lifecycle(definition, owner);
        this.projected = definition.projected;
        this.projecting = definition.projections;
        this.keyed = definition.key;

        // retain auditing, sharing, and provisioning with the declaration it implies
        this.isReadAudited = definition.audited?.reads === true;
        this.roles = Roles.of(definition);
        this.provisioned = definition.provisioned;
        this.declaration =
            definition.provisioned && Provisioned.declaration(definition.provisioned.kind);

        // retain methods and fields
        this.inherited = definition.inherited;
        this.methods = definition.methods;
        this.fields = definition.fields ?? {};
        this.text = definition.text;
        this.written = definition.written;
        this.sensitive = definition.sensitive;
        this.guarded = definition.guarded;

        // retain nesting, attachments, traits and storage
        this.parent = nestedParent(this, definition);
        this.aggregates = definition.aggregates ?? {};
        this.attachments = definition.attachments ?? [];
        this.traits = traits;
        this.intrinsic = intrinsic;
        this.indexes = definition.indexes ?? {};
        this.presentation = Presentable.of(definition);
        this.ordering = Orderable.of(definition);
        this.search =
            definition.search === undefined ? undefined : ObjectSearch.of(definition.search);
        this.storage = definition.storage;
        this.linger = this.storage === "ephemeral" ? lingerOf(definition) : undefined;

        // resolve the relations, add trait policies and register the policy
        const assembly = this.#assemblePolicy(definition, owner);
        this.permissions = assembly.permissions;
        this.policy = assembly.policy;

        // list the tables a database needs
        this.tree = this.table[TABLE].tree;
        this.tables = this.#databaseTables();

        // require valid methods, fields, aggregates, scopes, indexes and traits
        this.#requireValid(definition);
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

    /** Assemble the policy of the declared permissions and relations with the traits' policies. */
    #assemblePolicy(
        definition: ObjectDefinition,
        owner: Package,
    ): { permissions: string[]; policy: Policy } {
        // resolve the relations declared, read from fields and enclosing scopes
        const permissions = ObjectPermissions.expressions(definition.permissions ?? []);
        const relations = this.#relations(definition, owner, permissions);

        // collect the traits' policies and the permissions they derive
        const names = Object.keys(permissions);
        const object = traitObject(this, (definition.represents?.package ?? owner).id);
        const policies = this.traits.flatMap(({ trait, options }) =>
            trait.policy === undefined ? [] : [trait.policy(options, object, names)],
        );
        const derived = Object.fromEntries(
            policies.flatMap((policy) => Object.entries(policy.permissions ?? {})),
        );

        // register the policy over declared and derived permissions
        const policy = objectPolicy(owner, definition, {
            relations,
            permissions,
            derived,
            policies,
        });

        return { permissions: [...names, ...Object.keys(derived)], policy };
    }

    /** List the tables a database needs for the objects: the server's and text chunks beside durable rows. */
    #databaseTables(): readonly Table[] {
        // keep ephemeral and external rows in their table alone
        if (this.storage !== "durable") {
            return [this.table];
        }

        return [this.table, ...serverTables, ...(this.text.length === 0 ? [] : [chunk, chunkRun])];
    }

    /** Require valid methods, fields, aggregates, scopes, indexes and traits. */
    #requireValid(definition: ObjectTypeDefinition<Configuration>): void {
        // require the type's own members
        this.#requireMethods();
        this.#requireAggregates();
        this.#requireScopes();
        this.#requireIndexes();
        this.#requireStorage(definition, this.traits);

        // let each trait validate its options
        for (const { trait, options } of this.traits) {
            trait.validate?.(options, this, definition);
        }
    }

    /** Read the lifecycle a definition declares, requiring its conversions up to the package's release. */
    static #lifecycle<Configuration extends ObjectConfiguration>(
        definition: ObjectTypeDefinition<Configuration>,
        owner: Package,
    ): ObjectLifecycle<Configuration["declared"], Configuration["permissions"]> {
        // require each conversion and moved field
        ObjectType.#requireConversions(definition, owner);

        return {
            declarationSchema: definition.declarable?.schema,
            recoverable: definition.recoverable,
            expiring: definition.expiring,
            versioned: definition.versioned,
            tracked: definition.tracked,
            moved: definition.moved?.fields ?? {},
            convert: definition.convert ?? {},
        };
    }

    /** Require each conversion up to the package's release, and moved fields naming no current field. */
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

        // require moved fields to name previous fields only
        for (const [field, previous] of Object.entries(definition.moved?.fields ?? {})) {
            if (previous in (definition.fields ?? {})) {
                throw new TypeError(
                    `field ${definition.name}.${field} moved from ${previous}, which names a current field`,
                );
            }
        }
        Version.requireUpTo(definition.convert ?? {}, owner.version, definition.name);
    }

    /** Resolve the relations a definition declares, those its permissions read from fields, and its enclosing scopes. */
    #relations(
        definition: ObjectDefinition,
        owner: Package,
        permissions: Readonly<Record<string, AccessExpression>>,
    ): Record<string, RelationInput> {
        // resolve declared relations
        const relations: Record<string, RelationInput> = {};
        for (const [name, relation] of Object.entries(definition.relations ?? {})) {
            relations[name] = {
                subjects: relation.subjects.map((subject) => subjectOf(subject, owner)),
                ...(relation.grantedBy === undefined ? {} : { grantedBy: relation.grantedBy }),
                ...(relation.concealed === true ? { concealed: true } : {}),
            };
        }

        // derive relations from fields permissions read
        const decided = new Set(Object.values(permissions).flatMap(relationsOf));
        for (const [property, field] of Object.entries(definition.fields ?? {})) {
            const name = kebabCase(field.relationAt(property));
            const relation = decided.has(name) ? fieldRelation(field) : undefined;
            if (relation !== undefined) {
                relations[name] = relation;
            }
        }

        // relate the objects to their scope when a permission reads through it
        const enclosing = this.scopes.filter(
            (scope) => decided.has(scope.name) && relations[scope.name] === undefined,
        );
        for (const scope of enclosing) {
            relations[scope.name] = { subjects: [scope.policy], grantedBy: null, isScope: true };
        }

        return relations;
    }

    /** Require client method names functions lack, declared permissions, and permissions on record writes outside the system. */
    #requireMethods(): void {
        // refuse client method names functions have
        const shadowed = Object.entries(methodsOf(this)).find(
            ([name, declared]) =>
                declared.isSystem !== true &&
                Object.getOwnPropertyNames(Function.prototype).includes(name),
        )?.[0];
        if (shadowed !== undefined) {
            throw new TypeError(
                `object ${this.name} names a method ${shadowed}, which functions have`,
            );
        }

        // require declared permissions and valid methods
        for (const declared of Object.values(methodsOf(this))) {
            if (declared.permission !== null && !this.permissions.includes(declared.permission)) {
                throw new TypeError(`object ${this.name} has no permission ${declared.permission}`);
            } else if (
                declared.permission === null &&
                RECORD_KINDS.has(declared.kind) &&
                declared.isSystem !== true
            ) {
                throw new TypeError(
                    `object ${this.name} ${declared.kind}s without a permission outside the system`,
                );
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
            const shape = declared.input instanceof schema.Object ? declared.input.shape : {};
            const aggregated = Object.keys(shape).find(
                (field) => this.fields[field]?.aggregate !== undefined,
            );
            if (aggregated !== undefined) {
                throw new TypeError(
                    `method ${name} of ${this.name} writes aggregate field ${aggregated}`,
                );
            }
        }
    }

    /** Require scope types that are readable scopes. */
    #requireScopes(): void {
        for (const scope of this.scopes) {
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
                throw new TypeError(`index ${name} of ${this.name} names no field ${missing}`);
            } else if (unlogged !== undefined) {
                throw new TypeError(
                    `index ${name} of ${this.name} names unlogged field ${unlogged}`,
                );
            } else if (!isEnclosing) {
                throw new TypeError(
                    `index ${name} of ${this.name} must be unique within the universe or a scope type enclosing its objects`,
                );
            }
        }
    }

    /** Refuse durable-only declarations on ephemeral and external objects, and their writes or external work. */
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
    }

    /** The input conversions of a method: renamed fields, the object's conversions and the method's own, in order. */
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

    /** The name the type is served, routed and audited under: its name in camel case. */
    get key(): string {
        return camelCase(this.name);
    }

    /** Derive the audit action `Noun.method` recording one method. */
    audit(name: string, target: string = this.key): AuditAction {
        // read the method's audit details
        const declared = methodsOf(this)[name]?.audit;

        return {
            package: this.package,
            name: `${this.key}.${name}`,
            targets: schema.object({ [target]: AuditTarget }),
            details: declared === undefined ? schema.object({}) : declared.details.partial(),
        };
    }

    /** Build the audit action and values of a call: its object for a targeted method, else the scope's collection. */
    auditCall(name: string, input: JsonObject, scope: string) {
        // target one object
        const declared = methodsOf(this)[name];
        if (declared?.target === true) {
            const id = schema.string().parse(input["id"]);
            const targets = { [this.key]: { type: this.name, id } };

            return { action: this.audit(name), values: { targets, details: {} } };
        }

        // target the scope's collection
        const targets = { collection: { type: this.plural, id: scope } };

        return { action: this.audit(name, "collection"), values: { targets, details: {} } };
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

    /** Decide whether a value is an object type. */
    static is(value: unknown): value is ObjectType {
        return value instanceof ObjectType;
    }

    /** The package-qualified type of the objects, as access names it. */
    get typeReference(): ObjectTypeReference {
        return { packageId: this.policy.definition.packageId, type: this.policy.definition.name };
    }

    /** The table mapping access reads the objects through. */
    get mapping(): TableMapping {
        // copy every inherited row the condition matches
        const inherited =
            this.inherited === undefined ? {} : { inherited: this.inherited.where ?? {} };

        // reuse an intrinsic mapping
        if (this.intrinsic !== undefined) {
            return { ...this.intrinsic, policy: this.policy, ...inherited };
        }

        // map each declared relation to its field and add trait mappings
        const relations = fieldRelations(this);
        const placed: Partial<Pick<TableMapping, "parent" | "trees">> = {};
        for (const { relations: traitRelations, ...rest } of traitMappings(this)) {
            Object.assign(relations, traitRelations);
            Object.assign(placed, rest);
        }

        // map the self relation to the principal each object stands for under its own identifier
        if (Object.hasOwn(this.policy.definition.relations, "self")) {
            relations["self"] = { column: "id" };
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
            ...placed,
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

        // require its readers to list every measured row: the field's guard, or the read methods' permissions
        const guard = this.fields[name].access?.read;
        const reads = Object.values(methodsOf(this))
            .filter((declared) => declared.kind === "get" || declared.kind === "list")
            .flatMap((declared) => (declared.permission === null ? [] : [declared.permission]));
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

    /** The permission listing the objects needs, absent without a list method. */
    get listing(): Permission | undefined {
        return methodPermission(this, "list");
    }

    /** The permission reading one object needs, the listing's without a get method, or none. */
    get reading(): Permission | undefined {
        return methodPermission(this, "get") ?? this.listing;
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

    /** Build calls of the objects' mutating methods, to run later in the scope they are sent to or the one their input adds. */
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

    /** Wrap methods' handlers. */
    handle<Self extends ObjectType>(this: Self, handlers: HandlersOf<Self>): Self;
    /**
     * Wrap methods' handlers, keeping the type's signature.
     *
     * @construct the copy shares this type's table, methods and policy, with handlers attached by method name.
     */
    handle(this: ObjectType, handlers: Readonly<Record<string, Handler | Lifecycle>>): ObjectType {
        // wrap each named method's handler
        const methods: Record<string, Method> = { ...this.methods };
        for (const [name, handler] of Object.entries(handlers)) {
            const declared = methods[name];
            if (declared === undefined) {
                throw new TypeError(`object ${this.name} has no method ${name}`);
            }
            methods[name] = declared.handle(handler);
        }

        // copy the object type with the handled methods
        return this.with({ methods: methods });
    }

    /** Set how a stack's declarations of the objects become their managed records. */
    declare<
        Self extends ObjectOf<{ table: ManagedTable }>,
        Collected = DeclarationOf<Self>,
        Resolved = Collected,
    >(this: Self, declaration: ObjectDeclaration<Self, Collected, Resolved>): Self {
        return this.with({ declaration: declaration });
    }

    /** Reconcile or follow the objects with work waiting as the system. */
    control<Self extends ObjectType>(this: Self, controller: ObjectController<Self>): Self {
        // keep ephemeral and external objects off the controllers
        if (this.storage !== "durable") {
            throw new TypeError(`${this.storage} object ${this.name} takes no controller`);
        }

        return this.with({ controller });
    }

    /** Copy the object type with some members changed, sharing its table. */
    with<Self extends ObjectType>(
        this: Self,
        changes: Partial<
            Pick<Self, "methods" | "lifecycle" | "traits" | "controller" | "declaration">
        >,
    ): Self;
    /**
     * Copy the object type with some members changed, sharing its table.
     *
     * @construct the copy keeps this type's signature, its changed members of the same types as the ones they replace.
     */
    with(
        changes: Partial<
            Pick<ObjectType, "methods" | "lifecycle" | "traits" | "controller" | "declaration">
        >,
    ): ObjectType {
        return Object.assign(inherit(prototypeOf(this)), this, changes);
    }

    /** Reference one of the object's declared permissions. */
    permission<Self extends ObjectType>(this: Self, name: PermissionOf<Self>): Permission {
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

    /** The schema of one object as callers see it, typed by the type's fields. */
    rowSchema<Self extends ObjectType>(this: Self): RowSchema<Self>;
    /**
     * Read the row schema of the type's own fields.
     *
     * @construct the row schema is built from this type's fields, which is how RowSchema maps the same type.
     */
    rowSchema(): ObjectSchema["row"] {
        return this.schema.row;
    }

    /** Whether a controller reconciles the objects to their desired generation. */
    get isControlled(): boolean {
        return this.traits.some((applied) => applied.trait === controlled);
    }

    /** The object types the relations of plain reference fields name, whose permissions the type's permissions read through them. */
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

    /** The schema of the objects' identifiers: their natural key, or generated identifiers of the type's identity. */
    get idSchema(): schema.Schema<string> {
        if (this.keyed !== undefined) {
            return this.keyed;
        }
        const known = IDENTIFIERS.get(this.identity) ?? schema.identifier(this.identity);
        IDENTIFIERS.set(this.identity, known);

        return known;
    }

    /** Whether an identifier names one of the type's objects. */
    identifies(id: string): boolean {
        return this.idSchema.safeParse(id).success;
    }

    /** Generate a new object's identifier, refusing a type whose objects are named by their natural key. */
    generateId(): Identifier<string> {
        if (this.keyed !== undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `a creation of ${this.name} names its key`,
            });
        }

        return Identifier.create(this.identity);
    }

    /** Read an identifier of the type's identity, refusing any other. */
    identifier<Self extends ObjectType>(this: Self, id: string): Identifier<IdentityOf<Self>>;
    /**
     * Parse the identifier by the type's identity.
     *
     * @construct the parse requires the prefix of `this.identity`, the literal IdentityOf reads from the same type.
     */
    identifier(id: string): string {
        return this.idSchema.parse(id);
    }

    /** Read the directory's identity of one of the type's unique indexes: its shared namespace, or the type's own. */
    index(name: string): string {
        return (
            this.indexes[name]?.namespace ??
            `${this.policy.definition.packageId}/${this.name}/${name}`
        );
    }

    /** Key values in one of the type's indexes, within the scope the index is unique across. */
    claim(name: string, values: readonly unknown[], scope?: string): Pick<Claim, "index" | "key"> {
        // require the index and a needed scope
        const declared = this.indexes[name];
        if (declared === undefined) {
            throw new TypeError(`object ${this.name} has no index ${name}`);
        }
        const within = declared.across() instanceof ObjectType ? scope : Scope.universe.id;
        if (within === undefined) {
            throw new TypeError(`index ${name} of ${this.name} keys values within a scope`);
        }

        return { index: this.index(name), key: canonicalize([within, ...values]) };
    }

    /** List the names a row claims in the type's indexes with enclosing scopes from a snapshot. */
    async claims(row: Row, snapshot: Snapshot): Promise<Claim[]> {
        // key each index the row has every value of
        const scope = schema.string().parse(row["scope"]);
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
                packageId: this.policy.definition.packageId,
                objectId: schema.string().parse(row["id"]),
                scope,
            });
        }

        return claims;
    }

    /** Describe the names an object claims after a write, none after deletion. */
    async claimsOf(
        objectId: string,
        row: Row | undefined,
        snapshot: Snapshot,
    ): Promise<ObjectClaims> {
        return {
            indexes: Object.keys(this.indexes).map((name) => this.index(name)),
            objectId,
            claims: row === undefined ? [] : await this.claims(row, snapshot),
        };
    }

    /** Refuse a write whose names other objects own, naming the index of the first. */
    static refuse(objects: readonly ObjectType[], taken: readonly Claim[]): void {
        // refuse the first taken name by its index's declared name
        const [first] = taken;
        if (first !== undefined) {
            const [name] = objects.flatMap((object) =>
                Object.keys(object.indexes).filter((index) => object.index(index) === first.index),
            );
            throw new ServiceError("CONFLICT", {
                message: `${present(name, `the index of claim ${first.index}`)} is taken`,
            });
        }
    }

    /** Look up the object owning values in one of the type's indexes. */
    async lookup<Self extends ObjectType>(
        this: Self,
        directory: Directory,
        name: string,
        values: readonly unknown[],
        scope?: string,
    ): Promise<(ObjectReference & { readonly id: Identifier<IdentityOf<Self>> }) | undefined> {
        const { index, key } = this.claim(name, values, scope);
        const owner = await directory.owner(index, key);

        return owner === undefined
            ? undefined
            : {
                  ...this.reference(owner.scope, owner.objectId),
                  id: this.identifier(owner.objectId),
              };
    }

    /** Read the names the indexed rows an open transaction wrote claim, one entry per written object. */
    static async written(
        transaction: DatabaseConnection,
        objects: readonly ObjectType[],
    ): Promise<ObjectClaims[]> {
        // read each written row's last image
        const indexed = objects.filter((object) => Object.keys(object.indexes).length > 0);
        const byTable = new Map(indexed.map((object) => [object.table, object]));
        const owned = new Map<string, ObjectClaims>();
        const snapshot = Snapshot.live(transaction);
        if (indexed.length > 0) {
            for (const change of await transaction.log.written([...byTable.keys()])) {
                const object = found(byTable, change.table);
                const objectId = change.key["id"];
                if (typeof objectId !== "string") {
                    throw new TypeError(`a written ${object.name} has no identifier`);
                }
                const row = Change.after(change) ?? undefined;
                owned.set(
                    `${object.name}/${objectId}`,
                    await object.claimsOf(objectId, row, snapshot),
                );
            }
        }

        return [...owned.values()];
    }

    /** Find the scope an index is unique across, from the scope an object lives in. */
    async within(
        across: ObjectIndex["across"],
        scope: string,
        snapshot: Snapshot,
    ): Promise<string> {
        // key universe-wide indexes by the universe, and indexes across the object's own scope by it
        const enclosingType = across();
        if (!(enclosingType instanceof ObjectType)) {
            return Scope.universe.id;
        } else if (enclosingType.same(this.ancestors[0])) {
            return scope;
        }

        // find the enclosing scope of the declared type
        const { packageId, name } = enclosingType.policy.definition;
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

    /** Read the scopes a query of the objects reads from a scope chain, nearest first: the nearest, or every scope of the chain that holds such objects for inherited ones. */
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

    /** Decide whether another type shares this one's table. */
    same(other: ObjectType | "any" | undefined): boolean {
        return other instanceof ObjectType && other.table === this.table;
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

    /** The procedures of the object's methods. */
    get procedures(): ObjectProcedures<ObjectType<Configuration>> {
        return objectProcedures<ObjectType<Configuration>>(this);
    }

    /** Connect to the object's methods at a service serving them, speaking the service's release. */
    connect<Self extends ObjectType>(
        this: Self,
        service: Pick<Service, "package">,
        options: ClientOptions,
    ): Client<ObjectProcedures<Self>> {
        return createClient({ package: service.package, router: objectProcedures(this) }, options);
    }

    /** Assemble an object type from its definition: the fields callers write, the guarded, sensitive and text ones, and where its objects live. */
    static assemble(
        owner: Package,
        definition: ObjectDefinition & {
            readonly table: Table;
            readonly methods: Readonly<Record<string, Method>>;
        },
        traits: readonly TraitInstance[],
        intrinsic?: Omit<TableMapping, "policy">,
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

        return new ObjectType(
            owner,
            {
                ...definition,
                identity: definition.identity ?? definition.name,
                storage: definition.storage ?? "durable",
                written,
                guarded,
                sensitive,
                text: texts,
            },
            traits,
            intrinsic,
        );
    }

    /** Build the authorizer of object types, of every scope type enclosing them, of the types they relate to, and of other policies and mappings. */
    static authorizer(
        objects: readonly ObjectType[],
        others: readonly (Policy | ObjectType | TableMapping)[],
        copies: (table: Table) => boolean,
    ): Authorizer {
        // collect the object types and the types they relate to
        const types = [...objects, ...others.filter((other) => other instanceof ObjectType)];
        const related = [
            ...new Set(
                types
                    .flatMap((object) => object.related)
                    .filter((target) => !types.some((type) => type.same(target))),
            ),
        ];

        return new Authorizer(
            [
                ...objects.flatMap((object) => [
                    object.policy,
                    ...object.scopes
                        .flatMap((scope) => [scope, ...scope.ancestors])
                        .map((scope) => scope.policy),
                ]),
                ...others.map((other) => (other instanceof Policy ? other : other.policy)),
                ...related.map((object) => object.policy),
            ],
            [
                ...[...types, ...related].map((object) => object.mapping),
                ...others.flatMap((other) =>
                    other instanceof Policy || other instanceof ObjectType ? [] : [other],
                ),
            ],
            { copies },
        );
    }

    /** List the types projecting the objects into their recipients' homes, refusing one naming another source. */
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

    /** List object types with the chunk type their text fields need. */
    static served(objects: readonly ObjectType[]): ObjectType[] {
        const owners = objects.filter((object) => object.text.length > 0);

        return owners.length === 0 ? [...objects] : [...objects, Chunk.object(owners)];
    }

    /** Decide whether objects of another type attach to these objects. */
    attaches(child: ObjectType): boolean {
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
export type ObjectAggregate = {
    /** The values of the aggregated objects' fields. */
    readonly where?: Readonly<Record<string, string | number | boolean | null>>;
    /** The reference field naming the holder object, the parent when absent. */
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

/** Build an object type's constraints over its derived columns, taking any object's columns as its declaration would. */
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

/** The traits whose options type an object's table, methods or declarations, each inferred from its own option. */
type TypedTrait =
    | "nested"
    | "recoverable"
    | "versioned"
    | "controlled"
    | "declarable"
    | "suspendable"
    | "detachable"
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

/** The shape of one stack declaration of a declarable object. */
export type StackDeclarationOf<Traits> =
    TraitOf<Traits, "declarable"> extends DeclarableDefinition<infer Declared> ? Declared : never;

/** The methods an object's traits derive beside its declared ones. */
export type TraitMethods<Fields, Traits> = TransitionMethodMap<Fields> &
    RecoverableMethodMap<TraitOf<Traits, "recoverable">> &
    ControlledMethodMap<TraitOf<Traits, "controlled">> &
    NestedMethodMap<TraitOf<Traits, "nested">> &
    ShareableMethodMap<SharingGateOf<TraitOf<Traits, "shareable">>> &
    SuspendableMethodMap<GateOf<TraitOf<Traits, "suspendable">>> &
    DeclarableMethodMap<TraitOf<Traits, "declarable">> &
    BindableMethodMap<TraitOf<Traits, "bindable">> &
    DetachableMethodMap<GateOf<TraitOf<Traits, "detachable">>> &
    TrackedMethodMap<TraitOf<Traits, "tracked">> &
    TextMethodMap<Fields>;

/** The methods an object type declares, built on its own table and needing only the permissions it grants. */
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

/** The methods whose permissions an object type grants, a method needing another permission typed never. */
type PermittedMethods<Methods, Granted extends string> = {
    readonly [Name in keyof Methods]: Methods[Name] extends { readonly permission: infer Needed }
        ? [Needed] extends [Granted | null]
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
    const Detachable extends Gated<NoInfer<Permissions> | RolePermission> | undefined = undefined,
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
        readonly detachable: Detachable;
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
            Permissions | RolePermissionsOf<Sharing> | ProvisionedPermissionOf<Provisioning>,
            Plural
        >,
        "methods"
    > &
        ObjectMethods<
            ObjectTable<
                Name,
                ScopeIdentityOf<Scope>,
                NoInfer<
                    ObjectFields<
                        Fields &
                            RoleFieldsOf<Sharing, IsScope> &
                            ProvisionedFieldsOf<Provisioning> &
                            PresentationFieldsOf<Presenting> &
                            OrderedFieldsOf<Ordering>
                    >
                >,
                NoInfer<TableTraitsOf<TraitsOf<Traits> & ProvisionedTraitsOf<Provisioning>>>,
                Identity,
                Storage
            >,
            Methods,
            Permissions | RolePermissionsOf<Sharing> | ProvisionedPermissionOf<Provisioning>,
            NoInfer<
                WrittenField<
                    ObjectFields<
                        Fields &
                            RoleFieldsOf<Sharing, IsScope> &
                            ProvisionedFieldsOf<Provisioning> &
                            PresentationFieldsOf<Presenting> &
                            OrderedFieldsOf<Ordering>
                    >
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
            /** A system controller reconciles the objects. */
            readonly controlled?: Controlled;
            /** Stacks declare the objects. */
            readonly declarable?: Declarable;
            /** Holders of the permission suspend and resume a scope object. */
            readonly suspendable?: Suspendable;
            /** Holders of the permission detach a declared record from its declaration. */
            readonly detachable?: Detachable;
            /** Installations bind to the objects, and the holders of a permission bind one consumer at a time. */
            readonly bindable?: Bindable;
            /** Every change stays in the log, grouped into activities. */
            readonly tracked?: Tracked;
            /** Project another type's rows into the objects, each its own object with its own state. */
            readonly projected?: ProjectedDefinition;
            /** The types projecting the objects into their recipients' homes. */
            readonly projections?: () => readonly ObjectType[];
            /** How callers share the objects: through the roles, or through the type's own relations. */
            readonly shareable?: Sharing;
            /** Whether the objects are scopes containing other objects. */
            readonly isScope?: IsScope;
            /** The kind whose resources the objects are. */
            readonly provisioned?: Provisioning;
            /** The attachments the objects take. */
            readonly attachments?: Attachments;
            /** The objects' natural key, which callers and sources name them by. */
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
        ObjectFields<
            Fields &
                RoleFieldsOf<Sharing, IsScope> &
                ProvisionedFieldsOf<Provisioning> &
                PresentationFieldsOf<Presenting> &
                OrderedFieldsOf<Ordering>
        >,
        TableTraitsOf<TraitsOf<Traits> & ProvisionedTraitsOf<Provisioning>>,
        Identity,
        Storage
    >;
    readonly fields: ObjectFields<
        Fields &
            RoleFieldsOf<Sharing, IsScope> &
            ProvisionedFieldsOf<Provisioning> &
            PresentationFieldsOf<Presenting> &
            OrderedFieldsOf<Ordering>
    >;
    readonly methods: Methods &
        ProvisionedMethodsOf<Provisioning> &
        TraitMethods<
            ObjectFields<Fields>,
            TraitsOf<Traits> & {
                readonly shareable: ProvisionedSharingOf<Provisioning, Sharing>;
            } & ProvisionedTraitsOf<Provisioning>
        >;
    readonly permissions:
        | Permissions
        | RolePermissionsOf<Sharing>
        | ProvisionedPermissionOf<Provisioning>;
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
        | "detachable"
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
 * @construct the table, fields, methods, traits and scope derive from the definition as the signatures above read it.
 */
export function defineObject(
    input: Omit<ObjectDefinition, "methods"> &
        ObjectMethods<Table, Readonly<Record<string, Method>>> & {
            readonly [INTRINSIC]?: Intrinsic;
        },
    module?: ModuleMetadata,
): ObjectType {
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
    const traits = TRAITS.flatMap((trait) => {
        const options = intrinsic === undefined ? trait.options(definition) : undefined;

        return options === undefined ? [] : [{ trait, options }];
    });

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

    return ObjectType.assemble(
        owner,
        { ...definition, table, methods },
        traits,
        intrinsic?.mapping,
    );
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

/** The name of a field creation fills with the calling principal, which only system creations write. */
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

/** Decide whether an expression admits the holders of a related object's permission, through unions and named permissions. */
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

/** Resolve a declared relation subject: an object type's policy, a named subject type of the package, or a subject as is. */
function subjectOf(
    subject: ObjectRelationInput["subjects"][number],
    owner: Package,
): RelationInput["subjects"][number] {
    // take an object type's policy
    if (subject instanceof ObjectType) {
        return subject.policy;
    }
    // name a subject type of the package
    else if (typeof subject === "string") {
        return Policy.subjectType(owner.id, subject);
    }
    // keep any other subject
    else {
        return subject;
    }
}

/** Relate a field to the subjects it names, absent for a field naming none. */
function fieldRelation(field: Field): RelationInput | undefined {
    // relate a principal reference to its kind
    if (field.type === "reference" && field.principals !== undefined) {
        return { subjects: field.principals, grantedBy: null };
    }
    // relate a plain reference to its target type
    else if (field.type === "reference" && field.target && !field.qualified) {
        return { subjects: [field.target().policy], grantedBy: null };
    }
    // relate a subject field to its principal kinds
    else if (field.type === "subject") {
        const subjects = field.principals ?? [
            principal.user,
            principal.machine,
            principal.installation,
        ];

        return { subjects, grantedBy: null };
    }
    // relate nothing else
    else {
        return undefined;
    }
}

/** List the durable-only declarations of a definition: its durable traits and keys. */
function durableDeclarations(
    definition: ObjectDefinition,
    traits: readonly TraitInstance[],
): string[] {
    const keys = ["audited", "inherited", "isScope", "aggregates", "indexes"] as const;

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

/** Assemble a type's policy from its declared and represented rules and its traits' policies. */
function objectPolicy(
    owner: Package,
    definition: ObjectDefinition<unknown, string, Readonly<Record<string, Method>>>,
    rules: {
        /** The relations the definition declares, reads from fields and enclosing scopes. */
        readonly relations: Readonly<Record<string, RelationInput>>;
        /** The permission expressions the definition declares. */
        readonly permissions: Readonly<Record<string, AccessExpression>>;
        /** The permission expressions the traits derive. */
        readonly derived: Readonly<Record<string, AccessExpression>>;
        /** The traits' policies. */
        readonly policies: readonly TraitPolicy[];
    },
): Policy {
    // refuse to represent a policy of another name
    const represented = definition.represents;
    if (represented && represented.name !== definition.name) {
        throw new TypeError(`object ${definition.name} cannot represent ${represented.name}`);
    }

    // combine the represented, declared and trait rules
    const { relations, permissions, derived, policies } = rules;
    const gate = Roles.gate(definition);

    return new Policy(represented?.package ?? owner, {
        name: definition.name,
        attributes: Object.fromEntries([
            ...policies.flatMap((policy) => Object.entries(policy.attributes ?? {})),
            ...Object.entries(definition.attributes ?? {}),
        ]),
        relations: {
            ...representedRelations(represented),
            ...relations,
            ...Object.fromEntries(
                policies.flatMap((policy) => Object.entries(policy.relations ?? {})),
            ),
        },
        permissions: {
            ...represented?.definition.permissions,
            ...permissions,
            ...derived,
        },
        ...(gate === undefined ? {} : { grantedBy: gate }),
        ...(definition.shareable?.read === undefined
            ? {}
            : { relationships: { read: definition.shareable.read } }),
        ...(definition.reserved === undefined ? {} : { reserved: definition.reserved }),
        ...(definition.elevated === undefined ? {} : { elevated: definition.elevated }),
        ...(definition.administration === undefined
            ? {}
            : { administration: definition.administration }),
        ...(definition.isScope === true || represented?.definition.scope === true
            ? { scope: true }
            : {}),
        contributes: policies.flatMap((policy) => policy.contributes ?? []),
    });
}

/** Map each declared relation a type's fields keep to its column. */
function fieldRelations(type: ObjectType): Record<string, TableMapping["relations"][string]> {
    const relations: Record<string, TableMapping["relations"][string]> = {};
    for (const [property, field] of Object.entries(type.fields)) {
        const name = kebabCase(field.relationAt(property));
        const isRelation = Object.hasOwn(type.policy.definition.relations, name);

        // keep a subject field of a relation as a subject key
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

    return relations;
}

/** List the mappings a type's traits add. */
function traitMappings(type: ObjectType) {
    const object = traitObject(type, type.policy.package.id);

    return type.traits.flatMap(({ trait, options }) =>
        trait.mapping === undefined ? [] : [trait.mapping(options, object)],
    );
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
    const standard = Object.values(object.methods).find((declared) => declared.kind === kind);

    return standard?.permission === undefined || standard.permission === null
        ? undefined
        : object.permission(standard.permission);
}

/** Describe an object type to its traits. */
function traitObject(type: ObjectType, packageId: PackageId): TraitObject {
    return {
        name: type.name,
        identity: type.identity,
        packageId,
        table: () => type.table,
        key: type.keyed,
        storage: type.storage,
    };
}
