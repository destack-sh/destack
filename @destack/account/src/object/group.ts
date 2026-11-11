import { principal, through } from "@destack/access";
import { type Select } from "@destack/db";
import { approval, defineObject, field } from "@destack/object";
import { account } from "./account.ts";
import { organization } from "./organization.ts";
import { user } from "./user.ts";
import { DisplayName } from "./display-name.ts";
import { nested, shareable } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A set of principals roles bind to at once, nested in a parent group of the same scope. */
export const group = defineObject(
    {
        name: "group",
        plural: "groups",
        title: t`${plural(argument("count"), { one: "Group", other: "Groups" })}`,
        scope: [account, organization],
        fields: { name: field.string(DisplayName) },
        constraints: {
            scopeName: { kind: "unique", on: ["scope", "name"] },
            scopeId: { kind: "unique", on: ["scope", "id"] },
        },
        access: {
            represents: principal.group,
            // let a change ask a group's members as one set, as GitHub asks a team
            contributes: [{ object: () => approval, relation: "approver", members: "member" }],
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
        },
        traits: [
            nested({
                in: "self",
                optional: true,
                delete: "restrict",
                add: "update",
                move: "update",
            }),
            shareable({ by: "share" }),
        ],
    },
    (method) => ({
        create: method.create("create", { fields: ["name"] }),
        update: method.update("update", { fields: ["name"] }),
        delete: method.delete("delete"),
    }),
);
/** A group of an account or an organization. */
export type Group = Select<typeof group.table>;
