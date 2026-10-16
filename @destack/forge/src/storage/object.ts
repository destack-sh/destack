import { space } from "@destack/account/object";
import { defineObject } from "@destack/object";
import { GitStorageKind } from "./kind.ts";

/** A space's Git storage resource: the repositories an installation stores. */
export const gitStorage = defineObject({
    name: "git",
    plural: "gitStorages",
    scope: space,
    provisioned: { kind: GitStorageKind },
    fields: {},
    administration: ["read", "manage"],
});
