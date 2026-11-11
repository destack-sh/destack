import { PackageVersion } from "@destack/package";
import type {} from "@destack/package/import-meta";

/** The observability package, as clients address its intake. */
export const OBSERVABILITY_PACKAGE: PackageVersion = PackageVersion.parse(
    import.meta.destack.package,
);
