import type { Registry } from "@destack/forge/client";
import { LATEST_TAG } from "@destack/forge/object";
import type { PackageId } from "@destack/package";
import { BuildCache } from "@destack/package/manifest";
import { FeatureCatalog } from "../feature/index.ts";

/** The catalogs read from each build so far. */
const CATALOGS = new BuildCache((reader) => FeatureCatalog.read(reader));

/** Read the features and meters a package's latest published release declares, each build's once. */
export function latestCatalog(
    registry: Pick<Registry, "open">,
    packageId: PackageId,
): Promise<FeatureCatalog> {
    return registry.open(packageId, LATEST_TAG).then((reader) => CATALOGS.read(reader));
}
