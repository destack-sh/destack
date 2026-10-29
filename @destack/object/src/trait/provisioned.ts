import { principal, relation, none } from "@destack/access";
import type { Select, Table } from "@destack/db";
import { PackageId } from "@destack/package";
import { ResourcePlacement, ResourceRetention, type ResourceKind } from "@destack/resource";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { field, type FieldsByColumn } from "../field/field.ts";
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
import type { ControlledMethodMap } from "./controlled.ts";
import { CONSUMER } from "./bindable.ts";
import type { RolePermission } from "./shareable.ts";
import type { Erasure } from "./trait.ts";

import { adopt, specify } from "../method/provisioned.ts";

/** The constraints of resources Destack creates: no connection lends them. */
const INTERNAL_CONSTRAINTS: Readonly<Record<string, ObjectConstraint>> = {
    connection: { kind: "check", condition: { connection: { isNull: true } } },
};

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
    /** The resource kind, whose specification the objects store. */
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
        /** The desired states the active consumers need, which the provider applies. */
        states: field.blobs().default([]),
        /** The desired states only draining consumers need, until they stop. */
        drainingStates: field.blobs().default([]),
        /** The desired states the provider applied, none before the first apply. */
        appliedStates: field.blobs().default([]),
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
        /** The machine running the resource, absent before provisioning. */
        machine: field.reference(principal.machine).optional(),
        /** What deletion does to stored content: destroy it, retain it a while, or retain it. */
        retention: field.json(ResourceRetention).default("forever"),
        /** The owner application installation, absent for stack resources and after release. */
        owner: field.subject(principal.installation).optional(),
    };
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
export type ResourceTable<Scope> = ObjectTable<
    string,
    ScopeIdentityOf<Scope>,
    FieldsByColumn<ProvisionedFieldsOf<ProvisionedDefinition>>,
    ProvisionedTraitsOf<ProvisionedDefinition>
>;

/** The object type of a kind's resources in a scope type, as code generic over kinds reads it. */
export type ResourceType<Scope> =
    ResourceTable<Scope> extends infer Definition extends Table
        ? ObjectOf<{
              table: Definition;
              scope: Extract<ScopeIdentityOf<Scope>, string>;
              methods: ProvisionedMethodsOf<ProvisionedDefinition> &
                  ControlledMethodMap<{ readonly approval: true }>;
          }>
        : never;

/** The row of a kind's resource in its space, as code generic over kinds reads it. */
export type Resource = Select<ResourceTable<"space">>;

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
        // leave a definition of other objects
        const provisioned = definition.provisioned;
        if (provisioned === undefined) {
            return definition;
        }

        // require the scope types the resources live in
        const scope = definition.scope;
        if (scope === undefined || scope === Scope.universe.id) {
            throw new TypeError(`resources ${definition.name} must belong to scope types`);
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
            shareable: {},
            fields: { ...resourceFields(provisioned.kind), ...definition.fields },
            constraints: {
                ...RESOURCE_CONSTRAINTS,
                ...(provisioned.kind.management === "external" ? {} : INTERNAL_CONSTRAINTS),
                ...definition.constraints,
            },
            permissions: {
                read: relation(CONSUMER),
                write: relation(CONSUMER),
                declare: none(),
                ...permissions,
            },
            methods: { ...resourceMethods(), ...definition.methods },
        };
    },

    /** Decide whether an object type is a kind's resources. */
    is(object: ObjectType): object is ResourceType<"space"> {
        return object.provisioned !== undefined;
    },

    /** Report whether a provider provisioned a resource, which then has its reference. */
    isProvisioned(row: { readonly reference: string | null }): boolean {
        return row.reference !== null;
    },
};

/** Declare the methods every resource takes beside those of its kind. */
function resourceMethods() {
    return {
        create: method.create("declare", {
            visibility: "installation",
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
                "connection",
                "provider",
                "reference",
                "tags",
            ],
        }),
        update: method.update("declare", {
            visibility: "installation",
            fields: ["definitionVersion", "spec", "retention"],
        }),
        delete: method.delete("declare", { visibility: "installation" }),
        adopt,
        specify,
    };
}
