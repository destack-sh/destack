import { principal, through } from "@destack/access";
import { schema } from "@destack/schema";
import { defineObject, device, field } from "../../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** People, the user principal local drafts belong to. */
export const user = defineObject({
    name: "user",
    plural: "users",
    title: t`${plural(argument("count"), { one: "User", other: "Users" })}`,
    scope: "universe",
    fields: {},
    access: {
        represents: principal.user,
        permissions: [],
    },
});

/** A draft one device stores only for its signed-in user. */
export const draft = defineObject(
    {
        name: "draft",
        plural: "drafts",
        title: t`${plural(argument("count"), { one: "Draft", other: "Drafts" })}`,
        scope: user,
        storage: "local",
        fields: { text: field.string(schema.string().max(1000)) },
        access: {
            permissions: ["read", "edit"],
        },
    },
    (method) => ({
        create: method.create("edit"),
        update: method.update("edit"),
    }),
);

/** A panel width one device stores for every person using it. */
export const panel = defineObject(
    {
        name: "panel",
        plural: "panels",
        title: t`${plural(argument("count"), { one: "Panel", other: "Panels" })}`,
        scope: device,
        storage: "local",
        fields: { width: field.number() },
        access: {
            permissions: ["read", "edit"],
        },
    },
    (method) => ({
        create: method.create("edit"),
        update: method.update("edit"),
    }),
);

/** A note a device stores for the people signed in on it. */
export const sticky = defineObject(
    {
        name: "sticky",
        plural: "stickies",
        title: t`${plural(argument("count"), { one: "Sticky", other: "Stickies" })}`,
        scope: device,
        storage: "local",
        fields: { text: field.string(schema.string().max(1000)) },
        access: {
            permissions: { use: through("device", "use") },
        },
    },
    (method) => ({
        list: method.list("use"),
        create: method.create("use"),
    }),
);
