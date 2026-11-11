import { through, universe } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { account } from "./account.ts";
import { HostnamePattern } from "./domain.ts";
import { propagated } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A hostname or hostname wildcard an account serves its spaces' routes on. */
export const hostname = defineObject(
    {
        name: "hostname",
        plural: "hostnames",
        title: t`${plural(argument("count"), { one: "Hostname", other: "Hostnames" })}`,
        scope: account,
        fields: {
            /** The canonical lowercase hostname, such as notes.acme.com. */
            name: field.string(HostnamePattern),
        },
        indexes: {
            name: { on: ["name"], across: () => universe },
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
        access: {
            permissions: {
                read: through("account", "read"),
                manage: through("account", "manage"),
            },
        },
        traits: [
            // replicate an account's hostnames into its spaces
            propagated({}),
        ],
    },
    (method) => ({
        create: method.create("manage", { fields: ["name"] }),
        delete: method.delete("manage"),
    }),
);
