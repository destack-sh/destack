import { InstallationKind } from "@destack/runtime/declare";
import { defineObject } from "@destack/object";
import { space } from "@destack/account/object";

/** A named dependency of an installation on an installation of a package, at its address. */
export const dependency = defineObject({
    name: "dependency",
    plural: "dependencies",
    scope: space,
    provisioned: { kind: InstallationKind },
    fields: {},
    administration: ["read"],
});
