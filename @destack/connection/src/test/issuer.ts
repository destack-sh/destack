import { defineObject } from "@destack/object";
import { Package, PackageId } from "@destack/package";
import { Issuer } from "../object/issuer.ts";
import connectionPackage from "../package.ts";

/** The package whose installation issues the fixture's installed apps' tokens. */
export const ISSUING: Package = {
    ...Package.of(connectionPackage),
    id: PackageId.parse("package-01996ab0-0000-7000-8000-0000000015e5"),
    name: "@destack/issuing",
};

/** The issuing package's issuers. */
export const testIssuer = defineObject(Issuer.definition(), { package: ISSUING }).methods(
    Issuer.methods,
);
