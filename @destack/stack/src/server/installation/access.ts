import {
    AccessContext,
    Caller,
    Grant,
    type GrantPermission,
    Manager,
    PermissionSet,
    principal,
} from "@destack/access";
import { Snapshot } from "@destack/db";
import { grant, installation, type MethodContext, PackageDescription, role } from "@destack/object";
import { canonicalize, present } from "@destack/schema";
import { type Call, Scope, type Subject } from "@destack/sync";
import { residency } from "@destack/account/object";
import { StackError } from "../../error/index.ts";
import { type InstallationRollout, Rollout } from "../../object/rollout.ts";

/** The name under which an installation manages its permissions binding and space grant. */
const GRANT_NAME = "permissions";

/** One change to an installation's access: the calls one caller makes. */
export interface AccessChange {
    /** The caller making the calls: the consenter, or the space lending its authority. */
    readonly caller: AccessContext;
    /** The calls upserting the role or the grant. */
    readonly calls: readonly Call[];
}

/** The roles and grant an installation gets in its space, and its residency home's relation. */
export const InstallationAccess = {
    /** Upsert an installation's roles in its space and its space's grant, each as its caller. */
    async upsert(
        context: MethodContext,
        target: InstallationRollout,
        description: PackageDescription | undefined,
    ): Promise<void> {
        // make each change as its caller, with the code running the call as the actor
        const reader: Pick<Snapshot, "read"> = { read: (table) => context.read(table) };
        const changes = await InstallationAccess.changes(
            reader,
            target,
            description,
            context.self.subject,
            context.now,
        );
        for (const change of changes) {
            await context.mutate(change.caller, change.calls);
        }
    },

    /** List the changes upserting an installation's roles and space grant, as some code now. */
    async changes(
        reader: Pick<Snapshot, "read">,
        target: InstallationRollout,
        description: PackageDescription | undefined,
        subject: Subject,
        now: number,
    ): Promise<AccessChange[]> {
        // act for the consenter, so access refuses grants beyond their permissions
        const consenter = Caller.acting(Rollout.consenter(target), subject);

        // ask consent for its requests on other packages' types, its own types going by their policies
        const set = Rollout.permissions(target.installation, description, "space");
        const installed = principal.installation.reference(target.scope, target.installationId);
        const requested = set.space ?? [];
        const consented = requested.filter(
            (permission) => permission.packageId !== target.installation.packageId,
        );

        // bind the consented permissions in the role the consenter manages
        const manager = {
            installationId: target.installationId,
            packageId: target.installation.packageId,
            name: GRANT_NAME,
        };
        const binding = role.call("upsertBinding", {
            ...role.in(target.scope),
            name: `installation/${target.installationId}`,
            description: `What ${target.installation.name} requests in this space`,
            permissions: consented,
            subject: installed,
            manager,
        });

        // grant the requested permissions outside the space, the universe's in every scope
        const [link] = await Scope.read(reader, [target.scope]);
        const found = present(link, `the scope of space ${target.scope}`);
        const permissions: GrantPermission[] = PermissionSet.grant(set, {
            space: target.scope,
            home: [],
            account: found.parent,
        })
            .map((entry) => entry.permission)
            .filter((permission) => permission.scope !== target.scope);

        // let the installation ask for its package's optional capabilities, each once its space approves
        const isAsking = Object.values(description?.capabilities ?? {}).some(
            (capability) => capability.optional === true,
        );
        const asking = isAsking
            ? [
                  {
                      ...installation.permission("allow"),
                      scope: target.scope,
                      objectId: target.installationId,
                      mode: "approval" as const,
                  },
              ]
            : [];
        const granting = await InstallationAccess.grant(
            reader,
            installed,
            [...permissions, ...asking],
            now,
        );

        // grant as the space, whose authority the grant lends
        const delegator = principal.space.reference(Scope.universe.id, target.scope);
        const lending = Caller.acting(delegator, subject);

        return [
            { caller: AccessContext.of(consenter, now), calls: [binding] },
            ...(granting.length === 0
                ? []
                : [{ caller: AccessContext.of(lending, now), calls: granting }]),
        ];
    },

    /** Build the calls relating a residency home's declared installation, or unrelating it. */
    async relate(
        reader: Pick<Snapshot, "read">,
        target: InstallationRollout,
        set: PermissionSet,
    ): Promise<Call[]> {
        // relate only the installations its stack manages, and only those of a residency's home
        if (!Manager.isManaged(target.installation)) {
            return [];
        }
        const [homed] = await reader
            .read(residency.table)
            .findMany({ where: { space: target.scope }, limit: 1 });
        if (homed === undefined) {
            return [];
        }

        // unrelate one asking for nothing there
        const at = residency.target({ scope: Scope.universe.id, id: homed.id });
        const { installationId: installed } = target;
        const { name, packageId, selection } = target.installation;
        if ((set.universe ?? []).length === 0) {
            return [residency.call("revoke", { ...at, installation: installed, name })];
        }
        // refuse one following no release, which every installation a residency's home declares follows
        else if (selection.kind !== "release") {
            throw StackError.of(
                "RELEASE_REQUIRED",
                `residency home installation ${name} follows no release`,
            );
        }
        // relate one following a release for what its universe-scoped requests ask
        else {
            const release = selection.version;

            return [
                residency.call("touch", {
                    ...at,
                    installation: installed,
                    name,
                    packageId,
                    release,
                }),
            ];
        }
    },

    /** Build the calls giving an installation its space's one grant of exactly some permissions. */
    async grant(
        reader: Pick<Snapshot, "read">,
        installed: ReturnType<typeof principal.installation.reference>,
        permissions: readonly GrantPermission[],
        now: number,
    ): Promise<Call[]> {
        // leave a grant giving exactly the permissions
        const delegator = principal.space.reference(Scope.universe.id, installed.scope);
        const current = await Grant.find(reader, { delegator, delegate: installed }, now);
        if (
            current !== undefined &&
            canonicalize(current.permissions) === canonicalize(permissions)
        ) {
            return [];
        }

        // replace the outdated grant with the exact one, or revoke it once nothing is requested
        const create = grant.call("create", {
            ...grant.in(installed.scope),
            delegate: installed,
            permissions: [...permissions],
        });
        const revoke = current === undefined ? [] : [grant.call("revoke", grant.target(current))];

        return permissions.length === 0 ? revoke : [create];
    },
};
