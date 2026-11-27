import { installation, Installation } from "@destack/stack/object";
import {
    AccessContext,
    Caller,
    type Delegation,
    Grant,
    type GrantPermission,
    PermissionSet,
    principal,
} from "@destack/access";
import { Snapshot } from "@destack/db/database";
import { grant, type MethodContext, PackageDescription, role } from "@destack/object";
import { canonicalize, present } from "@destack/schema";
import { type Call, Scope, type Subject } from "@destack/sync";

/** The name under which an installation manages its permissions binding and space grant. */
const GRANT_NAME = "permissions";

/** One change to an installation's access: the calls one caller makes. */
export interface AccessChange {
    /** The caller making the calls: the consenter, or the space lending its authority. */
    readonly caller: AccessContext;
    /** The calls upserting the role or the grant. */
    readonly calls: readonly Call[];
}

/** The roles and grant an installation gets in its space, and its region home's relation. */
export const InstallationAccess = {
    /** Upsert an installation's roles in its space and its space's grant, each as its caller. */
    async upsert(
        context: MethodContext,
        target: Installation,
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
        target: Installation,
        description: PackageDescription | undefined,
        subject: Subject,
        now: number,
    ): Promise<AccessChange[]> {
        // act for the consenter, so access refuses grants beyond their permissions
        const consenter = Caller.onBehalfOf(Installation.consenter(target), subject);

        // ask consent for its requests on other packages' types, its own types going by their policies
        const set = Installation.permissions(target, description, "space");
        const installed = principal.installation.reference(target.scope, target.id);
        const requested = set.space ?? [];
        const consented = requested.filter(
            (permission) => permission.packageId !== target.packageId,
        );

        // bind the consented permissions in the role the consenter manages
        const manager = {
            installationId: target.id,
            packageId: target.packageId,
            name: GRANT_NAME,
        };
        const binding = role.call("declare", {
            ...role.in(target.scope),
            name: `installation/${target.id}`,
            description: `What ${target.name} requests in this space`,
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
                      objectId: target.id,
                      mode: "approval" as const,
                  },
              ]
            : [];
        const granting = await InstallationAccess.grant(
            reader,
            installed,
            [...permissions, ...asking],
            now,
            "self",
        );

        // let it act in the space for the actors whose changes invoke it, within what it asks of each user there
        const asked = Installation.permissions(target, description, "user").space ?? [];
        const forActors = asked.map((permission) => ({ ...permission, scope: target.scope }));
        const acting = await InstallationAccess.grant(reader, installed, forActors, now, "actors");

        // grant as the space, whose authority the grant lends
        const delegator = principal.space.reference(Scope.universe.id, target.scope);
        const lending = Caller.onBehalfOf(delegator, subject);

        return [
            { caller: AccessContext.of(consenter, now), calls: [binding] },
            ...(granting.length + acting.length === 0
                ? []
                : [{ caller: AccessContext.of(lending, now), calls: [...granting, ...acting] }]),
        ];
    },

    /** Build the calls giving an installation its space's one grant of a delegation. */
    async grant(
        reader: Pick<Snapshot, "read">,
        installed: ReturnType<typeof principal.installation.reference>,
        permissions: readonly GrantPermission[],
        now: number,
        delegation: Delegation,
    ): Promise<Call[]> {
        // leave a grant giving exactly the permissions
        const delegator = principal.space.reference(Scope.universe.id, installed.scope);
        const current = await Grant.find(
            reader,
            { delegator, delegate: installed },
            now,
            delegation,
        );
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
            delegation,
        });
        const revoke = current === undefined ? [] : [grant.call("revoke", grant.target(current))];

        return permissions.length === 0 ? revoke : [create];
    },
};
