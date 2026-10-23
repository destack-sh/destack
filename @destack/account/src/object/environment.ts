import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { account } from "./account.ts";

/** An account-defined classification its spaces share, declared by the account's stack. */
export const environment = defineObject(
    {
        name: "environment",
        plural: "environments",
        scope: account,
        fields: {
            /** The account-local environment name. */
            name: field.string(),
        },
        permissions: ["read", "write"],
        constraints: {
            account: {
                kind: "foreignKey",
                on: ["scope"],
                references: { object: () => account, on: ["id"] },
                onDelete: "no action",
            },
            scopeName: { kind: "unique", on: ["scope", "name"] },
            scopeId: { kind: "unique", on: ["scope", "id"] },
            name: {
                kind: "check",
                condition: {
                    name: {
                        length: { gt: 0 },
                        characters: "a-z0-9-",
                        NOT: {
                            OR: "-0123456789".split("").map((first) => ({ startsWith: first })),
                        },
                    },
                },
            },
        },
    },
    (method) => ({
        create: method.create("write", { isInternal: true }),
        get: method.get("read"),
        list: method.list("read"),
        delete: method.delete("write", { isInternal: true }),
    }),
);
/** A persisted account environment. */
export type Environment = Select<typeof environment.table>;
