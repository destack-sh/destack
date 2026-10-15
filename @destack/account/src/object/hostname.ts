import { through, universe } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { account } from "./account.ts";
import { HostnamePattern } from "./domain.ts";

/** A hostname or hostname wildcard an account serves its spaces' routes on. */
export const hostname = defineObject({
    name: "hostname",
    plural: "hostnames",
    scope: account,
    // replicate an account's hostnames into its spaces
    inherited: {},
    fields: {
        /** The canonical lowercase hostname, such as notes.acme.com. */
        name: field.string(HostnamePattern),
    },
    indexes: {
        name: { on: ["name"], across: () => universe },
    },
    permissions: {
        read: through("account", "read"),
        manage: through("account", "manage"),
    },
    constraints: {
        account: {
            kind: "foreignKey",
            on: ["scope"],
            references: { object: () => account, on: ["id"] },
            onDelete: "no action",
        },
        nameForm: { kind: "check", condition: { name: { characters: "a-z0-9.*-" } } },
    },
}).methods((method) => ({
    create: method.create("manage", { fields: ["name"] }),
    delete: method.delete("manage"),
}));
