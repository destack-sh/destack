import { defineAdapter, definePackage } from "@destack/object";
import { RegistryError } from "./error/error.ts";
import { packageObject, reference, release, repository, tag } from "./object/index.ts";

/** The registry protocol: packages, releases, tags, repositories, references and pushed builds. */
export default definePackage({
    objects: { repository, reference, package: packageObject, release, tag },
    adapters: { builds: defineAdapter("builds", { path: "/builds" }) },
    errors: RegistryError,
});
