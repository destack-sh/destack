import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import { client, defineObject, field } from "../../src/index.ts";

/** People, the user principal local drafts belong to. */
export const user = defineObject({
    name: "user",
    plural: "users",
    scope: "universe",
    represents: principal.user,
    fields: {},
    permissions: [],
}).methods();

/** A draft one client keeps for its signed-in user alone. */
export const draft = defineObject({
    name: "draft",
    plural: "drafts",
    scope: user,
    storage: "local",
    fields: { text: field.string(schema.string().max(1000)) },
    permissions: ["read", "edit"],
}).methods((method) => ({
    list: method.list("read"),
    create: method.create("edit"),
    update: method.update("edit"),
}));

/** A panel width one client keeps for every person using it. */
export const panel = defineObject({
    name: "panel",
    plural: "panels",
    scope: client,
    storage: "local",
    fields: { width: field.number() },
    permissions: ["read", "edit"],
}).methods((method) => ({
    list: method.list("read"),
    create: method.create("edit"),
    update: method.update("edit"),
}));
