import { permission, principal, relation, union } from "@destack/access";
import {
    check,
    foreignKey,
    sql,
    TABLE,
    unique,
    type Column,
    type ColumnMap,
    type Table,
} from "@destack/db";
import { PackageId } from "@destack/package";
import { ResourcePlacement, ResourceRetention, type ResourceKind } from "@destack/resource";
import { Digest, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { field } from "../field/field.ts";
import { method, type MethodBuilder } from "../method/method.ts";
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
import type { ControlledMethodMap } from "./controlled.ts";
import type { DeclarableDefinition } from "./declarable.ts";
import { CONSUMER } from "./bindable.ts";
import type { RolePermission } from "./shareable.ts";
import type { Erasure } from "./trait.ts";

/** Objects that are a kind's resources, provisioned by the kind's providers. */
export interface ProvisionedDefinition<Kind extends ResourceKind = ResourceKind> {
    /** The resource kind, whose specification the objects keep. */
    readonly kind: Kind;
}

/** The method declarations of provisioned resources. */
const provisionedMethod: MethodBuilder<ProvisionedTable<unknown>> = method;

/** Make an installation the owner of a retained resource it declares again, or release it, cancelling a deletion. */
const adopt = provisionedMethod
    .mutation({
        permission: null,
        isSystem: true,
        input: schema.object({
            /** The installation owning the resource, absent to release it. */
            owner: schema.string().min(1).nullable(),
        }),
    })
    .handle((call) => call.update({ owner: call.input.owner, deletionRequestedAt: null }));

/** The fields of a kind's resources. */
function fields(kind: ResourceKind) {
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
        /** The host keeping the resource, absent for provider-managed storage. */
        hostId: field.string(schema.identifier("host")).optional(),
        /** What deletion does to the stored content: destroy it, keep it for a window, or keep it. */
        retention: field.json(ResourceRetention).default("forever"),
        /** The application installation owning the resource, absent for stack resources and after release. */
        owner: field.subject(principal.installation).optional(),
    };
}

/** The fields a definition provisioned as a kind's resources takes, none otherwise. */
export type ProvisionedFieldsOf<Provisioning> = Provisioning extends {
    readonly kind: infer Kind extends ResourceKind;
}
    ? Omit<ReturnType<typeof fields>, "spec"> & {
          /** The desired specification, typed by the kind. */
          readonly spec: ReturnType<typeof field.json<schema.Output<Kind["spec"]>>>;
      }
    : {};

/** The table of a kind's resources in a scope type, as code generic over kinds reads it. */
export type ProvisionedTable<Scope> = ObjectTable<
    string,
    ScopeIdentityOf<Scope>,
    ProvisionedFieldsOf<ProvisionedDefinition>,
    ProvisionedTraitsOf<ProvisionedDefinition> & { readonly declarable: DeclarableDefinition }
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

/** The permission every resource takes, none for other objects. */
export type ProvisionedPermissionOf<Provisioning> = [Provisioning] extends [undefined]
    ? never
    : "write" | RolePermission;

/** The traits every resource takes, none for other objects. */
export type ProvisionedTraitsOf<Provisioning> = [Provisioning] extends [undefined]
    ? {}
    : { readonly controlled: { readonly approval: true }; readonly bindable: true };

/** How resources are shared, through the roles, or as a definition declares otherwise. */
export type ProvisionedSharingOf<Provisioning, Sharing> = [Provisioning] extends [undefined]
    ? Sharing
    : {};

/** The methods every resource takes, none for other objects. */
export type ProvisionedMethodsOf<Provisioning> = [Provisioning] extends [undefined]
    ? {}
    : ReturnType<typeof Provisioned.methods>;

/** The resources of a kind: their fields, name index, constraints, consumers and methods. */
export const Provisioned = {
    /** Add what every resource takes to a definition provisioned as a kind's resources. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<
        Definition,
        | "controlled"
        | "bindable"
        | "shareable"
        | "fields"
        | "indexes"
        | "constraints"
        | "permissions"
        | "methods"
    > {
        // keep a definition of other objects
        const provisioned = definition.provisioned;
        if (provisioned === undefined) {
            return definition;
        }

        // require one scope type with the resources
        const scope = definition.scope;
        if (ObjectScope.isList(scope) || scope === Scope.universe.id) {
            throw new TypeError(`resources ${definition.name} live in exactly one scope type`);
        }

        // require permissions declared as expressions
        const permissions = definition.permissions ?? {};
        if (ObjectPermissions.isList(permissions)) {
            throw new TypeError(
                `resources ${definition.name} declare their permissions as expressions`,
            );
        }

        // reconcile, bind, share and adopt the resources as every kind does
        const declared = definition.constraints;

        return {
            ...definition,
            controlled: { approval: true },
            bindable: true,
            shareable: {},
            fields: { ...fields(provisioned.kind), ...definition.fields },
            indexes: { ...Provisioned.indexes(scope), ...definition.indexes },
            constraints: (columns: ColumnMap) => [
                ...Provisioned.constraints(definition.name, scope, columns),
                ...(declared?.(columns) ?? []),
            ],
            permissions: {
                read: relation(CONSUMER),
                write: union(relation(CONSUMER), permission("edit")),
                ...permissions,
            },
            methods: { ...Provisioned.methods(), ...definition.methods },
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

    /** Constrain a kind's resource table, its constraints named after the kind. */
    constraints(kind: string, scope: ObjectType, columns: Record<string, Column>) {
        return [
            foreignKey({
                columns: [present(columns["scope"], "the scope column")],
                foreignColumns: [scope.table[TABLE].column("id")],
            }).onDelete("restrict"),
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
                `${kind}_host`,
                sql`${columns["hostId"]} IS NULL OR ${columns["provider"]} IS NOT NULL`,
            ),
        ];
    },

    /** Declare the methods every resource takes beside those of its kind. */
    methods() {
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
        };
    },
};
