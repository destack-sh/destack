import { none, principal, relation } from "@destack/access";
import {
    and,
    check,
    type DatabaseConnection,
    dialectSQL,
    eq,
    foreignKey,
    index,
    sql,
    unique,
    uniqueIndex,
    type Select,
} from "@destack/db";
import { defineObject, field, method, ObjectError, type ObjectType } from "@destack/object";
import type { Stack } from "@destack/object/server";
import { ComputeDefinition, DeclarationName, PackageId } from "@destack/package";
import { ViewDescription } from "@destack/package/view";
import { Digest } from "@destack/package/file";
import { Plan } from "@destack/resource";
import { type Identifier, identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { SettingDescription } from "@destack/setting/inspect";
import {
    InstallationBuild,
    InstallationSelection,
    SpaceInstallation,
} from "../declare/installation.ts";
import { SpaceDefinition } from "../declare/space.ts";
import { space, SpaceCell } from "./space.ts";
import { Submission } from "./stack.ts";

/** A view a build's output mounts. */
export const BuildView = ViewDescription.extend({
    /** The output mounting the view. */
    output: DeclarationName,
});
/** A view a build's output mounts. */
export type BuildView = schema.Infer<typeof BuildView>;

/** A location within a view, with its query, excluding another origin. */
export const ViewPath = schema.string().regex(/^\/(?!\/)[^\\\r\n]*$/);

/** A view of an installation's revision, as a client opens it: the build serving it, the view and the location within it. */
export const InstallationView = schema.object({
    /** The installation. */
    installationId: identifier("installation"),
    /** The revision whose build serves the view. */
    revisionId: identifier("installation-revision"),
    /** The build serving the view. */
    build: InstallationBuild,
    /** The view's name. */
    view: DeclarationName,
    /** The view as the build's output mounts it. */
    definition: BuildView,
    /** The location within the view. */
    path: ViewPath,
});
/** A view of an installation's revision, as a client opens it. */
export type InstallationView = schema.Infer<typeof InstallationView>;

/** Read the plan waiting for approval: the installation's own, and those of what it manages, owns or binds. */
const plan = method({ permission: "read", mutates: false, output: Plan });

/** Follow a submitted build. */
const submit = method({ permission: "apply", input: Submission });

/** Approve the plan waiting for approval by its digest. */
const approve = method({
    permission: "apply",
    input: schema.object({
        /** The digest of the reviewed plan. */
        plan: Digest,
    }),
    output: Plan,
});

/** Accept a plan digest for the installation's controller to apply once it plans the same steps. */
const accept = method({
    permission: null,
    isSystem: true,
    input: schema.object({
        /** The digest of the approved plan. */
        plan: Digest,
    }),
}).handle((call) => call.revise({ approvedPlan: (call.input as { plan: string }).plan }));

/** Read the build and view a client opens at a location. */
const open = method({
    permission: "open",
    mutates: false,
    audited: true,
    input: schema.object({
        /** The view the revision declares. */
        view: DeclarationName,
        /** The location within the view. */
        path: ViewPath,
    }),
    output: InstallationView,
}).handle(async (call) => {
    // require a revision declaring the view
    const target = call.target as unknown as Installation;
    const { view, path } = call.input as { readonly view: string; readonly path: string };
    const [revision] =
        target.revisionId === null
            ? []
            : ((await call.database
                  .select()
                  .from(installationRevision.table)
                  .where(
                      eq(installationRevision.table.id, target.revisionId),
                  )) as InstallationRevision[]);
    const definition = revision?.views[view];
    if (revision === undefined || definition === undefined) {
        throw new ServiceError("NOT_FOUND", {
            message: `${target.alias} declares no view ${view}`,
        });
    }

    return InstallationView.parse({
        installationId: target.id,
        revisionId: revision.id,
        build: revision.build,
        view,
        definition,
        path,
    });
});

/** Point an installation at the revision it follows as the system. */
const follow = method({
    permission: null,
    isSystem: true,
    input: schema.object({
        /** The release, repository reference or checkout the revision evaluated. */
        selection: InstallationSelection.optional(),
        /** The exported definition or parameterised function, for stacks. */
        export: schema.string().min(1).optional(),
        /** Arguments for a parameterised definition, for stacks. */
        parameters: schema.record(schema.string(), schema.json()).optional(),
        /** The revision to follow. */
        revisionId: identifier("installation-revision"),
    }),
}).handle((call) => call.revise(call.input));

/** A package installed into a space: its stack or one of its applications. */
export const installation = defineObject({
    name: "installation",
    plural: "installations",
    scope: space,
    represents: principal.installation,
    controlled: true,
    declarable: { schema: SpaceInstallation },
    fields: {
        /** The installed package. */
        packageId: field.string(PackageId),
        /** Whether the installation declares the space's contents or serves as an application. */
        role: field.enum(["stack", "application"]).default("application"),
        /** The release, repository reference or checkout the installation follows. */
        selection: field.json(InstallationSelection),
        /** The exported space definition or parameterised function, for stacks. */
        export: field.string().optional(),
        /** Arguments supplied to the exported definition function, for stacks. */
        parameters: field.json(schema.record(schema.string(), schema.json())).optional(),
        /** The revision to apply, absent before the first submission. */
        revisionId: field
            .reference<"installation-revision">((): ObjectType => installationRevision)
            .optional(),
        /** The revision fully applied, absent before the first application. */
        appliedRevisionId: field
            .reference<"installation-revision">((): ObjectType => installationRevision)
            .optional(),
        /** Whether the installation should serve requests. */
        status: field.enum(["enabled", "suspended"]).default("enabled"),
        /** The space-local address for this installation. */
        alias: field.string(DeclarationName),
        /** Workload compute settings checked against package and host policy. */
        compute: field.json(schema.record(DeclarationName, ComputeDefinition)).default({}),

        /** The plan of the installation's own changes waiting for approval. */
        plan: field.json(Plan).optional(),
        /** The digest of the plan an approver accepted, applied once the controller plans it again. */
        approvedPlan: field.string(Digest).optional(),
    },
    constraints: (installation) => [
        foreignKey({ columns: [installation.scope], foreignColumns: [space.table.id] }).onDelete(
            "restrict",
        ),
        foreignKey({
            columns: [installation.scope, installation.managerInstallationId],
            foreignColumns: [installation.scope, installation.id],
        }),
        unique("installation_scope_alias").on(installation.scope, installation.alias),
        unique("installation_scope_id").on(installation.scope, installation.id),
        unique("installation_space_package").on(
            installation.scope,
            installation.id,
            installation.packageId,
        ),
        uniqueIndex("installation_stack")
            .on(installation.scope)
            .where(sql`${installation.role} = 'stack'`),
        index("installation_package").on(installation.packageId),
        foreignKey({
            columns: [installation.id, installation.revisionId],
            foreignColumns: [
                installationRevision.table.installationId,
                installationRevision.table.id,
            ],
        }).onDelete("restrict"),
        foreignKey({
            columns: [installation.id, installation.appliedRevisionId],
            foreignColumns: [
                installationRevision.table.installationId,
                installationRevision.table.id,
            ],
        }).onDelete("restrict"),
        check(
            "installation_stack_export",
            sql`(${installation.role} = 'stack') = (${installation.export} IS NOT NULL AND ${installation.parameters} IS NOT NULL)`,
        ),
    ],
    relations: { self: { subjects: [principal.installation], grantedBy: null } },
    permissions: {
        read: none(),
        list: none(),
        create: none(),
        update: none(),
        delete: none(),
        apply: none(),
        open: none(),
        replicate: relation("self"),
        run: relation("self"),
    },
    reserved: ["replicate", "run"],
    methods: {
        get: method.get("read"),
        list: method.list("list"),
        create: method.create("create", {
            fields: ["packageId", "role", "selection", "export", "parameters", "alias"],
        }),
        update: method.update("update", { fields: ["selection", "alias", "status"] }),
        delete: method.delete("delete"),
        plan,
        submit,
        approve,
        accept,
        open,
        follow,
    },
});

/** An immutable evaluation of an installation's selected build: the build, its views and settings, and a stack's space definition. */
export const installationRevision = defineObject({
    name: "installation-revision",
    plural: "installationRevisions",
    scope: space,
    fields: {
        /** The evaluated installation. */
        installationId: field.reference<"installation">((): ObjectType => installation, {
            delete: "cascade",
        }),
        /** The exact build evaluated. */
        build: field.json(InstallationBuild),
        /** The views the build's outputs mount, keyed by name. */
        views: field.json(schema.record(DeclarationName, BuildView)),
        /** The settings the build declares. */
        settings: field.json(schema.array(SettingDescription)),
        /** The evaluated space definition, for stacks. */
        definition: field.json(SpaceDefinition).optional(),
        /** The SHA-256 digest of the canonical build, parameters and definition. */
        digest: field.string(Digest),
    },
    constraints: (revision) => [
        unique("installation_revision_installation_id").on(revision.installationId, revision.id),
        unique("installation_revision_digest").on(revision.installationId, revision.digest),
        check(
            "installation_revision_digest",
            dialectSQL({
                sqlite: sql`length(${revision.digest}) = 64 AND ${revision.digest} NOT GLOB '*[^a-f0-9]*'`,
                postgresql: sql`(${revision.digest} COLLATE "C") ~ '^[a-f0-9]{64}$'`,
            }),
        ),
    ],
    permissions: ["read"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, {
            isSystem: true,
            input: schema.object({
                /** The package whose build the revision evaluates. */
                packageId: PackageId,
            }),
        }),
    },
});

/** A persisted installation record. */
export type Installation = Select<typeof installation.table>;

/** Reads of installations that other packages resolve references by. */
export const Installation = {
    /** Require an installation of a space by identifier, rejecting the declaration otherwise. */
    async require(
        database: DatabaseConnection,
        spaceId: string,
        id: string,
    ): Promise<Identifier<"installation">> {
        const [existing] = await database
            .select({ id: installation.table.id })
            .from(installation.table)
            .where(
                and(
                    installation.inScope(identifier("space").parse(spaceId)),
                    eq(installation.table.id, identifier("installation").parse(id)),
                ),
            );
        if (existing === undefined) {
            throw new ObjectError(
                "INVALID_DECLARATION",
                `installation ${id} is no installation of the space`,
            );
        }

        return existing.id;
    },

    /** Resolve a declared installation: one the stack declares by name, or one of the space by id. */
    async resolve(
        reference: string | { readonly id: string },
        stack: Stack,
    ): Promise<Identifier<"installation">> {
        // resolve an installation the stack declares
        if (typeof reference === "string") {
            return identifier("installation").parse(await stack.require(installation, reference));
        }

        // require an existing installation of the space
        return Installation.require(stack.database, stack.scope, reference.id);
    },

    /** Read an installation this cell serves now: in a space it serves, enabled, and not being deleted. */
    async serving(
        database: DatabaseConnection,
        id: Identifier<"installation">,
    ): Promise<Installation | undefined> {
        // read the installation joined to a space this cell serves
        const [joined] = await database
            .select({ installation: installation.table })
            .from(installation.table)
            .innerJoin(space.table, eq(space.table.id, installation.table.scope))
            .where(and(eq(installation.table.id, id), SpaceCell.served()));
        const found = joined?.installation;
        const isServing =
            found !== undefined && found.status === "enabled" && found.deletionRequestedAt === null;

        return isServing ? found : undefined;
    },

    /** Find a space's installation by its alias, with the revision it follows, as its cell routes a view origin. */
    async find(
        database: DatabaseConnection,
        spaceId: string,
        alias: string,
    ): Promise<
        { installation: Installation; revision: InstallationRevision | undefined } | undefined
    > {
        // read the installation with the alias
        const [found] = await database
            .select()
            .from(installation.table)
            .where(
                and(
                    installation.inScope(identifier("space").parse(spaceId)),
                    eq(installation.table.alias, alias),
                ),
            );
        if (found === undefined) {
            return undefined;
        }

        // read the revision it follows
        const [revision] =
            found.revisionId === null
                ? []
                : await database
                      .select()
                      .from(installationRevision.table)
                      .where(eq(installationRevision.table.id, found.revisionId));

        return { installation: found as Installation, revision };
    },
};
/** An immutable evaluation applied to an installation. */
export type InstallationRevision = Select<typeof installationRevision.table>;
