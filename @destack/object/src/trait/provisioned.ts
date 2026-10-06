import { permission, principal, relation, union } from "@destack/access";
import {
    check,
    type Select,
    sql,
    unique,
    type Column,
    type ColumnMap,
    type Table,
} from "@destack/db";
import { PackageId } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import {
    ResourceDefinition,
    ResourcePlacement,
    ResourceRetention,
    type KindState,
    type ResourceKind,
} from "@destack/resource";
import { Digest, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { field, type FieldOf } from "../field/field.ts";
import { method } from "../method/method.ts";
import {
    ObjectPermissions,
    ObjectScope,
    type ObjectDefinition,
    type ObjectIndex,
    type ObjectOf,
    type ObjectType,
    type ScopeIdentityOf,
} from "../object/object.ts";
import type { ObjectTable } from "../object/table.ts";
import type { ObjectDeclaration } from "../server/stack.ts";
import type { ControlledMethodMap } from "./controlled.ts";
import type { DeclarableDefinition } from "./declarable.ts";
import { CONSUMER } from "./bindable.ts";
import type { RolePermission } from "./shareable.ts";
import type { Erasure } from "./trait.ts";

import { adopt, specify } from "../method/provisioned.ts";
/** Objects that are a kind's resources, provisioned by the kind's providers. */
export interface ProvisionedDefinition<Kind extends ResourceKind = ResourceKind> {
    /** The resource kind, whose specification the objects keep. */
    readonly kind: Kind;
}

/** The fields of a kind's resources. */
function resourceFields(kind: ResourceKind) {
    return {
        /** The space-local name, unique across every kind. */
        name: field.string(),
        /** The immutable package release defining the resource's kind. */
        definitionPackageId: field.string(PackageId),
        /** The defining package's calendar version. */
        definitionVersion: field.string(),
        /** The definition's exported name in the release manifest. */
        definitionName: field.string(),
        /** The desired specification. */
        spec: field.json(kind.spec),
        /** The desired states the active consumers need, which the provider applies. */
        states: field.json(desiredStates(kind)).default([]),
        /** The desired states only draining consumers still need, which a recreation drops after stopping them. */
        drainingStates: field.json(desiredStates(kind)).default([]),
        /** The digest of the desired states the provider applied, absent before the first apply. */
        appliedState: field.string(Digest).optional(),
        /** Whether a host's provider provisions the resource, or its stack declares its reference. */
        origin: field.enum(["provisioned", "declared"]).default("provisioned"),
        /** The requested placement, absent for automatic selection. */
        placement: field.json(ResourcePlacement).optional(),
        /** The provider supplying the resource, absent before provisioning. */
        provider: field.string().optional(),
        /** The provider-assigned resource reference. */
        reference: field.string().optional(),
        /** The actual provider location, absent before provisioning. */
        location: field.string().optional(),
        /** The machine keeping the resource, absent for provider-managed storage. */
        machineId: field.string(schema.identifier("machine")).optional(),
        /** What deletion does to the stored content: destroy it, keep it for a window, or keep it. */
        retention: field.json(ResourceRetention).default("forever"),
        /** The application installation owning the resource, absent for stack resources and after release. */
        owner: field.subject(principal.installation).optional(),
    };
}

/** Read the schema of a kind's desired states: values of its state, none for a kind without a state. */
function desiredStates(kind: ResourceKind): schema.Schema<ResourceState[]>;
/**
 * Read the schema of a kind's desired states by its state schema.
 *
 * @construct a kind's state schema parses the JSON objects its declarations require of a resource.
 */
function desiredStates(kind: ResourceKind): schema.Schema {
    return kind.state === undefined
        ? schema.array(schema.record(schema.string(), schema.json())).max(0)
        : schema.array(kind.state);
}

/** The fields a definition provisioned as a kind's resources takes, none otherwise. */
export type ProvisionedFieldsOf<Provisioning> = Provisioning extends {
    readonly kind: infer Kind extends ResourceKind;
}
    ? Omit<ReturnType<typeof resourceFields>, "spec" | "states" | "drainingStates"> & {
          /** The desired specification, typed by the kind. */
          readonly spec: ReturnType<typeof field.json<schema.Output<Kind["spec"]>>>;
          /** The desired states the active consumers need, typed by the kind. */
          readonly states: FieldOf<{ value: readonly DesiredStateOf<Kind>[]; default: true }>;
          /** The desired states only draining consumers still need, typed by the kind. */
          readonly drainingStates: FieldOf<{
              value: readonly DesiredStateOf<Kind>[];
              default: true;
          }>;
      }
    : {};

/** A desired state of a kind's resources: its state's value, or any state where code is generic over kinds. */
type DesiredStateOf<Kind extends ResourceKind> = string extends Kind["name"]
    ? ResourceState
    : KindState<Kind> & ResourceState;

/** The table of a kind's resources in a scope type, as code generic over kinds reads it. */
export type ProvisionedTable<Scope> = ObjectTable<
    string,
    ScopeIdentityOf<Scope>,
    ProvisionedFieldsOf<ProvisionedDefinition>,
    ProvisionedTraitsOf<ProvisionedDefinition>
>;

/** A kind's resources in a scope type, as code generic over kinds reads them. */
export type ProvisionedObject<Scope> =
    ProvisionedTable<Scope> extends infer Definition extends Table
        ? ObjectOf<{
              table: Definition;
              scope: Extract<ScopeIdentityOf<Scope>, string>;
              methods: ProvisionedMethodsOf<ProvisionedDefinition> &
                  ControlledMethodMap<{ readonly approval: true }>;
          }>
        : never;

/** A kind's resource in its space, as code generic over kinds reads it. */
export type ProvisionedRecord = Select<ProvisionedTable<"space">>;

/** The permission every resource takes, none for other objects. */
export type ProvisionedPermissionOf<Provisioning> = [Provisioning] extends [undefined]
    ? never
    : "write" | RolePermission;

/** The traits every resource takes, none for other objects. */
export type ProvisionedTraitsOf<Provisioning> = [Provisioning] extends [undefined]
    ? {}
    : {
          readonly controlled: { readonly approval: true };
          readonly bindable: true;
          readonly declarable: DeclarableDefinition<ResourceDefinition>;
      };

/** How resources are shared, through the roles, or as a definition declares otherwise. */
export type ProvisionedSharingOf<Provisioning, Sharing> = [Provisioning] extends [undefined]
    ? Sharing
    : {};

/** The methods every resource takes, none for other objects. */
export type ProvisionedMethodsOf<Provisioning> = [Provisioning] extends [undefined]
    ? {}
    : ReturnType<typeof resourceMethods>;

/** The definition keys every resource takes from its kind. */
type ProvisionedKey =
    | "controlled"
    | "bindable"
    | "declarable"
    | "shareable"
    | "fields"
    | "indexes"
    | "constraints"
    | "permissions"
    | "methods";

/** The resources of a kind: their fields, declaration, name index, constraints and methods. */
export const Provisioned = {
    /** Add what every resource takes to a definition provisioned as a kind's resources. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, ProvisionedKey> {
        // keep a definition of other objects
        const provisioned = definition.provisioned;
        if (provisioned === undefined) {
            return definition;
        }

        // require one scope type with the resources
        const scope = definition.scope;
        if (scope === undefined || ObjectScope.isList(scope) || scope === Scope.universe.id) {
            throw new TypeError(`resources ${definition.name} live in exactly one scope type`);
        }

        // require the declaration the kind implies
        if (definition.declarable !== undefined) {
            throw new TypeError(
                `resources ${definition.name} take their declaration from their kind`,
            );
        }

        // require permissions declared as expressions
        const permissions = definition.permissions ?? {};
        if (ObjectPermissions.isList(permissions)) {
            throw new TypeError(
                `resources ${definition.name} declare their permissions as expressions`,
            );
        }

        // declare, reconcile, bind, share and adopt the resources as every kind does
        const declared = definition.constraints;

        return {
            ...definition,
            controlled: { approval: true },
            bindable: true,
            declarable: { schema: ResourceDefinition },
            shareable: {},
            fields: { ...resourceFields(provisioned.kind), ...definition.fields },
            indexes: { ...Provisioned.indexes(scope), ...definition.indexes },
            constraints: (columns: ColumnMap) => [
                ...Provisioned.constraints(definition.name, columns),
                ...(declared?.(columns) ?? []),
            ],
            permissions: {
                read: relation(CONSUMER),
                write: union(relation(CONSUMER), permission("edit")),
                ...permissions,
            },
            methods: { ...resourceMethods(), ...definition.methods },
        };
    },

    /** Keep resource names unique in a scope across every kind, keyed like an object type's own. */
    indexes(scope: ObjectType): Readonly<Record<string, ObjectIndex>> {
        return {
            name: {
                on: ["name"],
                unique: true,
                across: () => scope,
                namespace: `${scope.policy.definition.packageId}/resource/name`,
            },
        };
    },

    /** Constrain a kind's resource table, its constraints named after the kind, its scope checked by each call against the scope chain this database keeps. */
    constraints(kind: string, columns: Record<string, Column>) {
        return [
            unique(`${kind}_scope_name`).on(
                present(columns["scope"], "the scope column"),
                present(columns["name"], "the name column"),
            ),
            unique(`${kind}_scope_id`).on(
                present(columns["scope"], "the scope column"),
                present(columns["id"], "the id column"),
            ),
            check(
                `${kind}_provider`,
                sql`(${columns["provider"]} IS NULL) = (${columns["reference"]} IS NULL)`,
            ),
            check(
                `${kind}_location`,
                sql`${columns["location"]} IS NULL OR (${columns["provider"]} IS NOT NULL AND length(${columns["location"]}) > 0)`,
            ),
            check(
                `${kind}_machine`,
                sql`${columns["machineId"]} IS NULL OR ${columns["provider"]} IS NOT NULL`,
            ),
        ];
    },

    /** Declare how a stack's resources of a kind become their records, provisioned or declared at a reference. */
    declaration(
        kind: ResourceKind,
    ): ObjectDeclaration<ObjectType, ResourceDefinition, ResourceDefinition> {
        return {
            keys: ["resources"],
            collect: (document) =>
                Object.fromEntries(
                    Object.entries(
                        schema
                            .record(schema.string(), ResourceDefinition)
                            .parse(document["resources"] ?? {}),
                    ).filter(([, declared]) => declared.declaration.kind === kind.name),
                ),
            values: (name, declared) => declaredValues(name, declared),
        };
    },

    /** Decide whether an object type is a kind's resources. */
    is(object: ObjectType): object is ProvisionedObject<"space"> {
        return object.provisioned !== undefined;
    },
};

/** Declare the methods every resource takes beside those of its kind. */
function resourceMethods() {
    return {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, {
            isSystem: true,
            fields: [
                "name",
                "definitionPackageId",
                "definitionVersion",
                "definitionName",
                "spec",
                "placement",
                "retention",
                "owner",
            ],
        }),
        update: method.update(null, {
            isSystem: true,
            fields: ["definitionVersion", "spec", "retention"],
        }),
        delete: method.delete(null, { isSystem: true }),
        adopt,
        specify,
    };
}

/** Write the record a stack's resource declares, provisioned or declared at a reference. */
function declaredValues(name: string, declared: ResourceDefinition) {
    // TODO #Incomplete: adopt resources from other spaces after ownership and residency checks
    if (declared.adopt) {
        throw new ObjectError(
            "UNSUPPORTED_DECLARATION",
            `resource adoption is not supported: ${name}`,
        );
    }

    // require the provider connecting a resource whose reference the stack declares
    const { declaration, placement, reference } = declared;
    const provider = placement?.provider;
    if (reference !== undefined && provider === undefined) {
        throw new ObjectError(
            "INVALID_DECLARATION",
            `resource ${name} declares its reference without the provider connecting to it`,
        );
    }

    return {
        name,
        definitionPackageId: declaration.package.id,
        definitionVersion: declaration.package.version,
        definitionName: declaration.name,
        spec: declaration.spec,
        retention: declared.retention,
        placement: placement ?? null,
        ...(reference === undefined || provider === undefined
            ? { origin: "provisioned" as const }
            : { origin: "declared" as const, reference, provider }),
        tags: declared.tags,
    };
}
