import {
    AccessContext,
    Caller,
    Grant,
    type GrantPermission,
    Manager,
    PermissionSet,
    principal,
    Relationship,
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
        const binding = role.call("declare", {
            ...role.in(target.scope),
            name: `installation/${target.installationId}`,
            description: `What ${target.installation.name} requests in this space`,
            permissions: consented,
            subjects: [installed],
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

    /** Build the calls relating a residency home's installation at its deployed release. */
    async relate(
        reader: Pick<Snapshot, "read">,
        previous: Pick<InstallationRollout, "appliedRevisionId">,
        target: InstallationRollout,
        set: PermissionSet,
        version: string | undefined,
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

        // read whether the residency relates the installation now
        const at = residency.target({ scope: Scope.universe.id, id: homed.id });
        const { installationId: installed } = target;
        const { name, packageId, selection } = target.installation;
        const [related] = await Relationship.read(
            reader,
            residency.reference(Scope.universe.id, homed.id),
            {
                relation: "installation",
                subject: principal.installation.reference(target.scope, installed),
            },
        );
        const isAsking = (set.universe ?? []).length > 0;
        const isRelated = related !== undefined;
        const isUpgraded = target.appliedRevisionId !== previous.appliedRevisionId;

        // leave a settled relationship: related at the applied release while asking, else unrelated
        if (isAsking ? isRelated && !isUpgraded : !isRelated) {
            return [];
        }
        // unrelate one asking for nothing there
        else if (!isAsking) {
            return [residency.call("revoke", { ...at, installation: installed, name })];
        }
        // refuse one following no release, which every installation a residency's home declares follows
        else if (selection.kind !== "release" && selection.kind !== "host") {
            throw StackError.of(
                "RELEASE_BLOCKED",
                `cannot grant residency: home installation '${name}' follows no release`,
            );
        }
        // relate one once a release of it is deployed
        else if (version === undefined) {
            return [];
        }
        // relate one following a release for what its universe-scoped requests ask, again after an upgrade
        else {
            return [
                residency.call("touch", {
                    ...at,
                    installation: installed,
                    name,
                    packageId,
                    release: version,
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
