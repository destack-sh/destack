import { none, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { account } from "./account.ts";

/** Who administers an account's trusted publishers: its managers. */
const MANAGERS = through("account", "manage");

/** A CI workflow an account trusts to publish under its name, by its issuer's OIDC tokens. */
export const trustedPublisher = defineObject(
    {
        name: "trusted-publisher",
        plural: "trustedPublishers",
        scope: account,
        // publish under its account's name, as one of its publishers
        contributes: [{ object: () => account, relation: "publisher" }],
        fields: {
            /** The OIDC issuer signing the workflow's tokens, such as GitHub Actions'. */
            issuer: field.string(schema.url()),
            /** The workflow's subject at its issuer, such as `repo:acme/notes:environment:ci`. */
            subject: field.string(schema.string().min(1)),
        },
        permissions: { read: MANAGERS, create: MANAGERS, delete: MANAGERS, declare: none() },
        constraints: {
            // publish each workflow's releases under one account alone
            workflow: { kind: "unique", on: ["issuer", "subject"] },
        },
    },
    (method) => ({
        create: method.create("create", { fields: ["issuer", "subject"] }),
        delete: method.delete("delete"),
        declare: method.create("declare", {
            visibility: "package",
            fields: ["issuer", "subject"],
            onConflict: "workflow",
        }),
    }),
);
/** A persisted trusted publisher record. */
export type TrustedPublisher = Select<typeof trustedPublisher.table>;
