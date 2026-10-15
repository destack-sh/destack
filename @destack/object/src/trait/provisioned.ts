import { permission, principal, relation, union } from "@destack/access";
import type { Select, Table } from "@destack/db";
import { PackageId } from "@destack/package";
import {
    CONNECTION_PROVIDER,
    ResourceDefinition,
    ResourcePlacement,
    ResourceRetention,
    type ResourceKind,
} from "@destack/resource";
import { Digest, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { field } from "../field/field.ts";
import { method } from "../method/record.ts";
import {
    ObjectPermissions,
    type ObjectDefinition,
    type ObjectOf,
    type ObjectType,
    type ScopeIdentityOf,
} from "../object/object.ts";
import type { ObjectConstraint } from "../object/constraint.ts";
import type { ObjectTable } from "../object/table.ts";
import type { DeclarationHandler } from "../server/stack.ts";
import type { ControlledMethodMap } from "./controlled.ts";
import type { DeclarableDefinition } from "./declarable.ts";
import { CONSUMER } from "./bindable.ts";
import type { RolePermission } from "./shareable.ts";
import type { Erasure } from "./trait.ts";

import { adopt, specify } from "../method/provisioned.ts";

/** The constraints every resource takes. */
const RESOURCE_CONSTRAINTS: Readonly<Record<string, ObjectConstraint>> = {
    scopeName: { kind: "unique", on: ["scope", "name"] },
    scopeId: { kind: "unique", on: ["scope", "id"] },
    provider: {
        kind: "check",
        condition: {
            OR: [
                { provider: { isNull: true }, reference: { isNull: true } },
                { provider: { isNotNull: true }, reference: { isNotNull: true } },
            ],
        },
    },
    location: {
        kind: "check",
        condition: {
            OR: [
                { location: { isNull: true } },
                { provider: { isNotNull: true }, location: { length: { gt: 0 } } },
            ],
        },
    },
    machine: {
        kind: "check",
        condition: { OR: [{ machineId: { isNull: true } }, { provider: { isNotNull: true } }] },
    },
};

/** Objects that are a kind's resources, provisioned by the kind's providers. */
export interface ProvisionedDefinition<Kind extends ResourceKind = ResourceKind> {
    /** The resource kind, whose specification the objects keep. */
    readonly kind: Kind;
}

/** The fields of a kind's resources. */
function resourceFields(kind: ResourceKind) {
    return {
        /** The name, unique among the kind's resources in its scope. */
        name: field.string(),
        /** The immutable package release defining the resource's kind. */
        definitionPackageId: field.string(PackageId),
        /** The defining package's calendar version. */
        definitionVersion: field.string(),
        /** The definition's exported name in the release manifest. */
        definitionName: field.string(),
        /** The desired specification. */
        spec: field.json(kind.spec),
        /** The digests of the states the active consumers need, which the provider applies. */
        states: field.json(desiredStates(kind)).default([]),
        /** The digests of the states only draining consumers need, dropped after they stop. */
        drainingStates: field.json(desiredStates(kind)).default([]),
        /** The digest of the desired states the provider applied, absent before the first apply. */
        appliedState: field.string(Digest).optional(),
        /** Whether a provider provisions it, a stack declares it, or a connection lends it. */
        origin: field.enum(["provisioned", "declared", "connection"]).default("provisioned"),
        /** The space connection whose credential the host lends to every call, if any. */
        connection: field.string(schema.identifier("connection")).optional(),
        /** The requested placement, absent for automatic selection. */
        placement: field.json(ResourcePlacement).optional(),
        /** The provider supplying the resource, absent before provisioning. */
        provider: field.string().optional(),
        /** The provider-assigned resource reference. */
        reference: field.string().optional(),
        /** The actual provider location, absent before provisioning. */
        location: field.string().optional(),
        /** The machine keeping the resource, absent before provisioning. */
        machineId: field.string(schema.identifier("machine")).optional(),
        /** What deletion does to stored content: destroy it, keep it a while, or keep it. */
        retention: field.json(ResourceRetention).default("forever"),
        /** The application installation owning it, absent for stack resources and after release. */
        owner: field.subject(principal.installation).optional(),
    };
}

/** Read the schema of a kind's desired states by digest, none for a stateless kind. */
function desiredStates(kind: ResourceKind) {
    return kind.state === undefined ? schema.array(Digest).max(0) : schema.array(Digest);
}

/** The fields a definition provisioned as a kind's resources takes, none otherwise. */
export type ProvisionedFieldsOf<Provisioning> = Provisioning extends {
    readonly kind: infer Kind extends ResourceKind;
}
    ? Omit<ReturnType<typeof resourceFields>, "spec"> & {
          /** The desired specification, typed by the kind. */
          readonly spec: ReturnType<typeof field.json<schema.Output<Kind["spec"]>>>;
      }
    : {};

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

        // require the scope types the resources live in
        const scope = definition.scope;
        if (scope === undefined || scope === Scope.universe.id) {
            throw new TypeError(`resources ${definition.name} must live in scope types`);
        }

        // require the declaration the kind implies
        if (definition.declarable !== undefined) {
            throw new TypeError(
                `resources ${definition.name} cannot be declarable: their kind provides the declaration`,
            );
        }

        // require permissions declared as expressions
        const permissions = definition.permissions ?? {};
        if (ObjectPermissions.isList(permissions)) {
            throw new TypeError(
                `resources ${definition.name} must declare their permissions as expressions`,
            );
        }

        // declare, reconcile, bind, share and adopt the resources as every kind does
        return {
            ...definition,
            controlled: { approval: true },
            bindable: true,
            declarable: { schema: ResourceDefinition },
            shareable: {},
            fields: { ...resourceFields(provisioned.kind), ...definition.fields },
            constraints: { ...RESOURCE_CONSTRAINTS, ...definition.constraints },
            permissions: {
                read: relation(CONSUMER),
                write: union(relation(CONSUMER), permission("edit")),
                ...permissions,
            },
            methods: { ...resourceMethods(), ...definition.methods },
        };
    },

    /** Declare how a stack's resources become records, provisioned or declared by reference. */
    declaration(
        kind: ResourceKind,
    ): DeclarationHandler<ObjectType, ResourceDefinition, ResourceDefinition> {
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
            values: (name, declared) => Provisioned.values(name, declared),
        };
    },

    /** Write the record a stack's resource declares, provisioned or declared at a reference. */
    values(name: string, declared: ResourceDefinition) {
        // require the provider connecting a resource whose reference the stack declares
        const { declaration, placement, reference, connection } = declared;
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
            ...(connection !== undefined || provider === CONNECTION_PROVIDER
                ? {
                      origin: "connection" as const,
                      connection: connection ?? null,
                      provider: CONNECTION_PROVIDER,
                  }
                : reference === undefined || provider === undefined
                  ? { origin: "provisioned" as const }
                  : { origin: "declared" as const, reference, provider }),
            tags: declared.tags,
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
        create: method.create("declare", {
            isInternal: true,
            fields: [
                "name",
                "definitionPackageId",
                "definitionVersion",
                "definitionName",
                "spec",
                "placement",
                "retention",
                "ownerPackageId",
                "ownerType",
                "ownerScope",
                "ownerId",
                "origin",
                "connection",
                "provider",
                "reference",
                "tags",
            ],
        }),
        update: method.update("declare", {
            isInternal: true,
            fields: ["definitionVersion", "spec", "retention"],
        }),
        delete: method.delete("declare", { isInternal: true }),
        adopt,
        specify,
    };
}
