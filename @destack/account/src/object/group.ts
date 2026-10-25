import { principal, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { account } from "./account.ts";
import { organization } from "./organization.ts";
import { user } from "./user.ts";
import { DisplayName } from "./display-name.ts";

/** A set of principals roles bind to at once, nested in a parent group of the same scope. */
export const group = defineObject(
    {
        name: "group",
        plural: "groups",
        scope: [account, organization],
        nested: {
            in: "self",
            optional: true,
            delete: "restrict",
            receive: "update",
            move: "update",
        },
        represents: principal.group,
        fields: { name: field.string(DisplayName) },
        relations: {
            member: {
                subjects: [
                    user,
                    principal.installation,
                    principal.group.members("member"),
                    account.members("member"),
                ],
            },
        },
        permissions: {
            create: through("organization", "manage"),
            read: through("organization", "read"),
            update: through("organization", "manage"),
            delete: through("organization", "manage"),
            share: through("organization", "manage"),
        },
        shareable: { by: "share" },
        constraints: {
            scopeName: { kind: "unique", on: ["scope", "name"] },
            scopeId: { kind: "unique", on: ["scope", "id"] },
        },
    },
    (method) => ({
        create: method.create("create", { fields: ["name"] }),
        update: method.update("update", { fields: ["name"] }),
        delete: method.delete("delete"),
    }),
);
/** A group of an account or an organization. */
export type Group = Select<typeof group.table>;
