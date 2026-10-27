import { ListPage } from "@destack/object";
import type { ClientOf } from "@destack/object/client";
import type objectPackage from "@destack/object/package";
import type { PackageId } from "@destack/package";
import { type Identifier, present } from "@destack/schema";
import { Rollout, type NamedInstallation } from "../object/rollout.ts";
import type { StackClient } from "./client.ts";

/** A declaration in the build an installation applies. */
export interface Declaration {
    /** The rollout applying the build, with its installation's fields. */
    readonly rollout: Rollout & { readonly installation: NamedInstallation };
    /** The declaration in the build's graph index. */
    readonly declaration: Awaited<ReturnType<StackClient["declaration"]["list"]>>["items"][number];
}

/** Reads of the declarations a space's installations apply. */
export const Declaration = {
    /** Read the declarations of some kinds a space's enabled, applied installations declare. */
    async read(
        stack: StackClient,
        objects: ClientOf<typeof objectPackage>,
        spaceId: Identifier<"space">,
        kinds: readonly { readonly kindPackageId: PackageId; readonly kind: string }[],
    ): Promise<Declaration[]> {
        // list the rollouts applied at a revision, and the enabled installations of the space
        const applied = await Array.fromAsync(
            ListPage.all(
                (input: Parameters<typeof stack.rollout.list>[0]) => stack.rollout.list(input),
                { spaceId, where: { appliedRevisionId: { isNotNull: true } } },
            ),
        );
        const enabled = await Array.fromAsync(
            ListPage.all(
                (input: Parameters<typeof objects.installation.list>[0]) =>
                    objects.installation.list(input),
                { scope: spaceId, where: { status: "enabled" } },
            ),
        );

        // join the applied rollouts of enabled installations with them
        const byId = new Map(enabled.map((each) => [each.id, each]));
        const rollouts = applied.flatMap((row) => {
            const installed = byId.get(row.id);

            return installed === undefined ? [] : [Rollout.join(row, installed)];
        });
        if (rollouts.length === 0) {
            return [];
        }

        // read the build of each applied revision
        const revisionIds = rollouts.map((each) => present(each.appliedRevisionId, "a revision"));
        const revisions = await Array.fromAsync(
            ListPage.all(
                (input: Parameters<typeof stack.installationRevision.list>[0]) =>
                    stack.installationRevision.list(input),
                { spaceId, where: { id: { in: revisionIds } } },
            ),
        );

        // read each build's declarations of the kinds from its graph index
        const declarations = await Promise.all(
            rollouts.map(async (rollout) => {
                const revision = present(
                    revisions.find((entry) => entry.id === rollout.appliedRevisionId),
                    `the revision ${rollout.installation.name} applies`,
                );
                const declared = await Array.fromAsync(
                    ListPage.all(
                        (input: Parameters<typeof stack.declaration.list>[0]) =>
                            stack.declaration.list(input),
                        {
                            buildId: revision.buildId,
                            where: { OR: [...kinds] },
                        },
                    ),
                );

                return declared.map((declaration) => ({ rollout, declaration }));
            }),
        );

        return declarations.flat();
    },
};
