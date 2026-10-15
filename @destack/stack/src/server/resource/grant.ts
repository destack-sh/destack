import type { PermissionReference } from "@destack/access";
import { Space } from "@destack/account/object";
import { eq } from "@destack/db";
import { type Call, type CallOf, type ObjectReconciliation } from "@destack/object";
import { Address, Plan } from "@destack/resource";
import { aligned, present } from "@destack/schema";
import {
    BindingGrant,
    bindingGrant,
    type Installation,
    installation,
    role,
} from "../../object/index.ts";
import { Approval } from "../installation/approval.ts";

/** Implement binding grants: approve their plans and keep each approved grant's role. */
export function implementBindingGrant() {
    return bindingGrant.implement({
        methods: {
            approve: async (call) => {
                // require the reviewed plan waiting now and accept it for the controller to apply
                const { plan } = call.target;
                const digest = await Approval.require(plan, call.input.plan);
                await call.mutate(call.self, [
                    bindingGrant.call("approvePlan", { id: call.target.id, plan: digest }),
                ]);

                return present(plan, "the approved plan");
            },
            grant: async (call) => {
                // keep the role granting the call
                const permissions = [installation.permission("call")];
                await BindingGrantRole.keep(call, call.target, await called(call), permissions);

                return {};
            },
            finalize: async (call, next) => {
                // drop the role of the deleted grant
                await BindingGrantRole.keep(call, call.target, await called(call), []);

                return next();
            },
        },
        controller: {
            pending: {},
            reconcile: async (reconciliation) => {
                await reconcileGrant(aligned(reconciliation.rows, 0), reconciliation);

                return undefined;
            },
        },
    });
}

/** The role a binding grant keeps on its installation, bound to the grantee alone. */
export const BindingGrantRole = {
    /** Keep a grant's role on its installation granting exactly some permissions. */
    async keep(
        call: Call,
        grant: BindingGrant,
        target: Installation,
        permissions: readonly PermissionReference[],
    ): Promise<void> {
        // bind the role on the installation to the grantee
        const manager = { installationId: target.id, packageId: target.packageId, name: grant.id };
        await call.mutate(call.self, [
            role.call("keep", {
                ...role.in(grant.scope),
                name: `${bindingGrant.name}/${grant.id}`,
                description: `What installation ${grant.granteeInstallationId} of space ${grant.granteeSpaceId} calls of ${target.name}`,
                permissions: [...permissions],
                subject: BindingGrant.grantee(grant),
                object: installation.reference(target.scope, target.id),
                manager,
            }),
        ]);
    },

    /** Drop the roles of the grants on an installation being removed, whose grants go with it. */
    async drop(call: Call, target: Installation): Promise<void> {
        const grants = await call
            .read(bindingGrant)
            .findMany({ where: { installationId: target.id } });
        for (const grant of grants) {
            await BindingGrantRole.keep(call, grant, target, []);
        }
    },
};

/** Keep one grant: remove it once deleted, wait for its plan's approval, else keep its role. */
async function reconcileGrant(
    grant: BindingGrant,
    reconciliation: ObjectReconciliation<typeof bindingGrant>,
): Promise<void> {
    // leave a grant of a space another machine serves, such as one moving here
    if (!(await Space.isServing(reconciliation.database, grant.scope))) {
        return;
    }

    // remove a deleted grant with its role
    if (grant.deletedAt !== null) {
        await reconciliation.mutate([bindingGrant.call("finalize", bindingGrant.target(grant))]);

        return;
    }

    // wait for an approver to accept the plan of the grant as it stands
    const [target] = await reconciliation.database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, grant.installationId));
    const plan = grantPlan(grant, present(target, `installation ${grant.installationId}`));
    const digest = await Plan.digest(plan);
    if (grant.approvedPlan !== digest) {
        const message = `approve plan ${digest} with ${plan.steps.length} steps`;
        await reconciliation.observe(bindingGrant, grant, {
            conditions: { ready: { status: "false", reason: "AwaitingApproval", message } },
            fields: { plan },
        });

        return;
    }

    // keep the role granting the call
    await reconciliation.mutate([bindingGrant.call("grant", bindingGrant.target(grant))]);
    await reconciliation.observe(bindingGrant, grant, {
        conditions: { ready: { status: "true", reason: "Granted", message: "" } },
        fields: { plan: null },
    });
}

/** Read the installation a grant lets its grantee call. */
async function called(
    call: CallOf<typeof bindingGrant, "grant" | "finalize">,
): Promise<Installation> {
    const target = await call
        .read(installation)
        .findFirst({ where: { id: call.target.installationId } });

    return present(target, `installation ${call.target.installationId}`);
}

/** Plan a grant: one step letting its grantee call its installation. */
function grantPlan(grant: BindingGrant, target: Installation): Plan {
    const calls = grant.calls.length === 0 ? "" : ` (${grant.calls.join(", ")})`;

    return {
        steps: [
            {
                action: "create",
                target: Address.join(bindingGrant.name, grant.id),
                risk: "safe",
                detail: `let installation ${grant.granteeInstallationId} of space ${grant.granteeSpaceId} call ${target.name}${calls}`,
            },
        ],
    };
}
