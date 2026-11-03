import { Package } from "@destack/package";
import type {} from "@destack/package/import-meta";

/** The observability package, as clients address its intake. */
export const OBSERVABILITY_PACKAGE: Package = Package.parse(import.meta.destack.package);
