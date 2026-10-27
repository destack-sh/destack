import { none, through } from "@destack/access";
import { residency, space } from "@destack/account/object";
import { Issuer } from "@destack/connection/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** The GitHub App's issuers in a residency's home, called by its accounts' installations. */
export const issuer = defineObject(
    Issuer.definition({
        relations: {
            // the residency whose home declares the issuer, related as the issuer is born
            residency: { subjects: [residency.policy], grantedBy: "manage" },
        },
        // let the installations in the residency's accounts call the issuer, managed by the home's stack alone
        permissions: {
            read: through("residency", "reside"),
            manage: none(),
            issue: through("residency", "reside"),
        },
    }),
    Issuer.methods,
);

/** A GitHub App's installation on an external account, claimed by the one setting it up. */
export const appInstallation = defineObject(
    {
        name: "app-installation",
        plural: "app-installations",
        scope: space,
        fields: {
            /** GitHub's identifier of the installation. */
            installation: field.string(schema.string().min(1)),
            /** The account whose installation completed the installation's setup. */
            accountId: field.string(schema.identifier("account")),
            /** The connection installation that completed the setup, the one issuing its tokens. */
            claimer: field.string(schema.identifier("installation")),
        },
        permissions: { read: none(), manage: none() },
        constraints: {
            installation: { kind: "unique", on: ["scope", "installation"] },
        },
    },
    (method) => ({
        create: method.create("manage", {
            fields: ["installation", "accountId", "claimer"],
            visibility: "package",
            isPredicted: false,
        }),
    }),
);

/** A claimed GitHub App installation. */
export type AppInstallation = Select<typeof appInstallation.table>;
