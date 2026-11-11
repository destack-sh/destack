import { space } from "@destack/account/object";
import { defineObject } from "@destack/object";
import { GitStorageKind } from "@destack/git";
import { provisioned } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A space's Git storage resource: the repositories an installation stores. */
export const gitStorage = defineObject({
    name: "git",
    plural: "gitStorages",
    title: t`${plural(argument("count"), { one: "Git storage", other: "Git storages" })}`,
    scope: space,
    fields: {},
    access: {
        suspended: ["read", "manage"],
    },
    traits: [provisioned({ kind: GitStorageKind })],
});
