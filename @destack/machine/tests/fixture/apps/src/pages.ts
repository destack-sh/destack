import { space } from "@destack/account/object";
import { principal, relation, through, union } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A workspace of pages, whose members read and edit its pages. */
export const workspace = defineObject(
    {
        name: "workspace",
        plural: "workspaces",
        scope: space,
        fields: {
            /** The principal who created the workspace. */
            owner: field.subject().caller(),
            /** The workspace's name. */
            name: field.string(schema.string().min(1).max(100)),
        },
        relations: { member: { subjects: [principal.user] } },
        permissions: {
            read: union(relation("owner"), relation("member")),
            write: union(relation("owner"), relation("member")),
            manage: relation("owner"),
        },
        shareable: { by: "manage" },
    },
    (method) => ({ create: method.create("manage", { fields: ["name"] }) }),
);

/** A page of a workspace, whose body its members edit live. */
export const page = defineObject(
    {
        name: "page",
        plural: "pages",
        scope: space,
        nested: { in: workspace, delete: "cascade", receive: "write" },
        fields: {
            /** The page's title. */
            title: field.string(schema.string().min(1).max(200)),
            /** The page's body. */
            body: field.text(),
        },
        permissions: { read: through("parent", "read"), write: through("parent", "write") },
    },
    (method) => ({
        create: method.create("write", { fields: ["title"] }),
        update: method.update("write", { fields: ["title"] }),
    }),
);
