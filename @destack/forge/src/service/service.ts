import { PackageId } from "@destack/package";
import { Digest, schema } from "@destack/schema";
import { defineProcedure, defineService } from "@destack/service";
import { dependency, packageObject, reference, release, repository, tag } from "../object/index.ts";

/** The path below the forge's mount serving the builds of published releases by their manifest's digest. */
export const BUILDS_PATH = "/builds/";

/** The path below the forge's mount serving npm's reads of published releases. */
export const NPM_PATH = "/npm/";

/** The manifests of published releases, by package and version. */
export const manifests = {
    /** Find the digest of the manifest a package's published release names. */
    find: defineProcedure({ authentication: "identity", permission: null, audit: "access" })
        .route({ method: "GET", path: "/packages/{packageId}/releases/{version}/manifest" })
        .input(
            schema.object({
                /** The package. */
                packageId: PackageId,
                /** The released version. */
                version: schema.string().min(1),
            }),
        )
        .output(
            schema.object({
                /** The digest of the release's manifest. */
                manifest: Digest,
            }),
        ),
};

/** Repositories and their references, packages and their releases, tags and dependencies, served as objects, and the manifests of the releases. */
export const forgeService = defineService("forge", {
    objects: {
        repository,
        reference,
        package: packageObject,
        release,
        tag,
        dependency,
    },
    manifests,
});
