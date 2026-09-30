import { and, eq, type DatabaseConnection } from "@destack/db";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { identifier, type Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { InstallationBuild } from "../../declare/installation.ts";
import { installation, installationRevision } from "../../object/index.ts";

/** Open a package's build through the cell's store of builds. */
export type OpenBuild = (packageId: PackageId, build: InstallationBuild) => Promise<BuildReader>;

/** Open the build of a package's release in a space: the revision an installation follows, or else the one installation of the package. */
export async function openRelease(
    database: DatabaseConnection,
    open: OpenBuild,
    spaceId: string,
    packageId: PackageId,
    installationId?: Identifier<"installation">,
): Promise<BuildReader> {
    // read the revisions the given installation, or the package's installations in the space, follow
    const followed = await database
        .select({ build: installationRevision.table.build })
        .from(installation.table)
        .innerJoin(
            installationRevision.table,
            eq(installationRevision.table.id, installation.table.revisionId),
        )
        .where(
            and(
                eq(installation.table.scope, identifier("space").parse(spaceId)),
                eq(installation.table.packageId, packageId),
                installationId === undefined
                    ? undefined
                    : eq(installation.table.id, installationId),
            ),
        );

    // require exactly one
    const [only] = followed;
    if (only === undefined || followed.length > 1) {
        throw new ServiceError("NOT_FOUND", {
            message: `no single installation in ${spaceId} follows a release of ${packageId}`,
        });
    }

    return open(packageId, only.build);
}
