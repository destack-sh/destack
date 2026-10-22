import { ListPage } from "@destack/object";
import type { PackageId } from "@destack/package";
import { type Identifier, present } from "@destack/schema";
import type { Installation } from "../object/installation.ts";
import type { StackClient } from "./client.ts";

/** A declaration in the build an installation applies. */
export interface Declaration {
    /** The installation applying the build. */
    readonly installation: Installation;
    /** The declaration in the build's graph index. */
    readonly declaration: Awaited<ReturnType<StackClient["declaration"]["list"]>>["items"][number];
}

/** Reads of the declarations a space's installations apply. */
export const Declaration = {
    /** Read the declarations of some kinds each enabled, applied installation of a space declares. */
    async read(
        stack: StackClient,
        spaceId: Identifier<"space">,
        kinds: readonly { readonly kindPackageId: PackageId; readonly kind: string }[],
    ): Promise<Declaration[]> {
        // list the enabled installations applied at a revision
        const installations = await Array.fromAsync(
            ListPage.all(
                (input: Parameters<typeof stack.installation.list>[0]) =>
                    stack.installation.list(input),
                { spaceId, where: { status: "enabled", appliedRevisionId: { isNotNull: true } } },
            ),
        );
        if (installations.length === 0) {
            return [];
        }

        // read the build of each applied revision
        const revisionIds = installations.map((each) =>
            present(each.appliedRevisionId, "a revision"),
        );
        const revisions = await Array.fromAsync(
            ListPage.all(
                (input: Parameters<typeof stack.installationRevision.list>[0]) =>
                    stack.installationRevision.list(input),
                { spaceId, where: { id: { in: revisionIds } } },
            ),
        );

        // read each build's declarations of the kinds from its graph index
        const declarations = await Promise.all(
            installations.map(async (installation) => {
                const revision = present(
                    revisions.find((entry) => entry.id === installation.appliedRevisionId),
                    `the revision ${installation.name} applies`,
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

                return declared.map((declaration) => ({ installation, declaration }));
            }),
        );

        return declarations.flat();
    },
};
