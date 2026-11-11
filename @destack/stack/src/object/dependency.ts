import { InstallationKind, defineObject } from "@destack/object";
import { space } from "@destack/account/object";
import { provisioned } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A named dependency of an installation on an installation of a package, at its address. */
export const dependency = defineObject({
    name: "dependency",
    plural: "dependencies",
    title: t`${plural(argument("count"), { one: "Dependency", other: "Dependencies" })}`,
    scope: space,
    fields: {},
    access: {
        suspended: ["read"],
    },
    traits: [provisioned({ kind: InstallationKind })],
});
