import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { account } from "./account.ts";
import { argument, plural, t } from "@destack/locale";

/** A deployment stage an account's spaces share, such as dev, declared by its stack. */
export const stage = defineObject(
    {
        name: "stage",
        plural: "stages",
        title: t`${plural(argument("count"), { one: "Stage", other: "Stages" })}`,
        scope: account,
        fields: {
            /** The account-local stage name. */
            name: field.string(),
        },
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
        access: {
            permissions: ["read", "write"],
        },
    },
    (method) => ({
        create: method.create("write", { visibility: "package" }),
        get: method.get("read"),
        list: method.list("read"),
        delete: method.delete("write", { visibility: "package" }),
    }),
);
/** A persisted account stage. */
export type Stage = Select<typeof stage.table>;
