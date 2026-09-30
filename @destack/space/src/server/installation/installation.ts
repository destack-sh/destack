import { and, eq } from "@destack/db";
import { Condition } from "@destack/db/query";
import type { Identifier } from "@destack/schema";
import { digest } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import * as base from "../../object/index.ts";
import {
    binding,
    deployment,
    type Installation,
    installationRevision,
    resource,
    Submission,
} from "../../object/index.ts";
import { type ApplicationOptions, reconcileApplication } from "./application.ts";
import { Approval } from "./approval.ts";
import { type Invoke, reconcileStack, recordSubmission, type StackOptions } from "./stack.ts";

/** Installations a stack declares, installing each application at the revision of its selected release. */
export const installation = base.installation.declare({
    values: (_name, desired) => ({
        packageId: desired.package.id,
        selection: { kind: "release" as const, version: desired.package.version },
        alias: desired.alias,
        status: desired.status,
        compute: desired.compute,
        tags: desired.tags,
    }),
    changed: async (database, row, desired) => {
        // report a release with a revision the installation does not follow yet
        if (row.revisionId === null) {
            return true;
        }
        const [followed] = await database
            .select({ digest: installationRevision.table.digest })
            .from(installationRevision.table)
            .where(eq(installationRevision.table.id, row.revisionId));

        return followed?.digest !== (await revisionOf(desired).digest);
    },
    written: async (context, row, desired) => {
        // retain the release's revision once, and follow it
        const revision = revisionOf(desired);
        const evaluated = await revision.digest;
        const [existing] = await context.database
            .select({ id: installationRevision.table.id })
            .from(installationRevision.table)
            .where(
                and(
                    eq(installationRevision.table.installationId, row.id),
                    eq(installationRevision.table.digest, evaluated),
                ),
            );
        const id =
            existing?.id ??
            (
                (await context.invoke(installationRevision, "create", {
                    installationId: row.id,
                    packageId: row.packageId,
                    build: revision.build,
                    digest: evaluated,
                })) as { readonly id: string }
            ).id;
        if (row.revisionId !== id) {
            await context.invoke(installation, "follow", { id: row.id, revisionId: id });
        }
    },
});

/** What a cell serves installations with: its stacks' and applications' options. */
export interface InstallationOptions extends StackOptions, ApplicationOptions {}

/** Serve installations: record submissions, approve waiting plans, and apply stacks or deploy applications. */
export function serveInstallations(options: InstallationOptions) {
    return installation
        .handle({
            plan: async (call) => {
                // join the plans the installation and what it manages, owns or binds wait on
                const target = call.target as unknown as Installation;

                return (await Approval.of(call.database, target)).plan();
            },
            submit: async (call) => {
                // refuse a build of another package than the installation's, whose identity stays
                const target = call.target as unknown as Installation;
                const submission = Submission.parse(call.input);
                const reader = await options.openBuild(target.packageId, submission.build);
                const built = reader.manifest.package.id;
                if (built !== target.packageId) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `${target.alias} installs ${target.packageId}, submitted a build of ${built}`,
                    });
                }

                // record the submission as the system for the controllers to apply
                const invoke: Invoke = (object, name, input) => call.invoke(object, name, input);
                await recordSubmission(call.database, invoke, target, submission);

                // answer the installation as it follows the submission now
                const [followed] = await call.database
                    .select()
                    .from(base.installation.table)
                    .where(eq(base.installation.table.id, target.id));

                return followed!;
            },
            approve: async (call) => {
                // accept each plan waiting as the approver reviewed them
                const target = call.target as unknown as Installation;
                const { plan } = call.input as { readonly plan: string };
                const invoke: Invoke = (object, name, input) => call.invoke(object, name, input);

                return (await Approval.of(call.database, target)).grant(plan, invoke);
            },
        })
        .control({
            pending: Condition.all(),
            // apply a space's one stack under its space, and deploy each application under its own key
            key: (row) =>
                row.role === "stack" ? { scope: row.scope, role: "stack" } : { id: row.id },
            watches: [
                {
                    // wake the space's stack while it waits for its resources
                    table: resource.table,
                    keys: async (row, database) => {
                        const [stack] = await database
                            .select({ conditions: base.installation.table.conditions })
                            .from(base.installation.table)
                            .where(
                                and(
                                    eq(
                                        base.installation.table.scope,
                                        row.scope as Identifier<"space">,
                                    ),
                                    eq(base.installation.table.role, "stack"),
                                ),
                            );
                        const isWaiting = stack?.conditions.ready?.reason === "WaitingForResources";

                        return isWaiting ? [{ scope: row.scope, role: "stack" }] : [];
                    },
                },
                { table: deployment.table, keys: (row) => [{ id: row.installationId }] },
                { table: binding.table, keys: (row) => [{ id: row.installationId }] },
            ],
            reconcile: (control) => {
                const target = control.rows[0] as unknown as Installation;

                return target.role === "stack"
                    ? reconcileStack(target.scope, control, options)
                    : reconcileApplication(target.id, control, options);
            },
        });
}

/** Describe the revision a declared release evaluates to: its build and the build's digest. */
function revisionOf(desired: { readonly package: { readonly version: string } }) {
    const build = { kind: "release" as const, version: desired.package.version };

    return { build, digest: digest({ build }) };
}
