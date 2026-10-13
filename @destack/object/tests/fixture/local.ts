import { principal, through } from "@destack/access";
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
    create: method.create("edit"),
    update: method.update("edit"),
}));

/** A note a client keeps for the people signed in on it. */
export const sticky = defineObject({
    name: "sticky",
    plural: "stickies",
    scope: client,
    storage: "local",
    fields: { text: field.string(schema.string().max(1000)) },
    permissions: { use: through("client", "use") },
}).methods((method) => ({
    list: method.list("use"),
    create: method.create("use"),
}));
