import { none, principal, relation } from "@destack/access";
import type { Table } from "@destack/db";
import { space } from "@destack/account/object";
import { MethodBuilder, ObjectType } from "@destack/object";
import type { Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { Token } from "../credential/credential.ts";

/** What an installation token opens: some of the installation's resources and permissions. */
export const InstallationGrant = defineSchema(
    schema.object({
        /** The resources the token opens, such as repositories, by identifier or name. */
        resources: schema
            .union([
                schema.object({ ids: schema.array(schema.string().min(1)) }),
                schema.object({ names: schema.array(schema.string().min(1)) }),
            ])
            .exactOptional(),
        /** The permissions the token has, by name, such as contents read. */
        permissions: schema.record(schema.string().min(1), schema.string().min(1)).exactOptional(),
    }),
);
/** What an installation token opens. */
export type InstallationGrant = schema.Infer<typeof InstallationGrant>;

/** The installation a completed installation page redirects back with. */
export const IssuedInstallation = defineSchema(
    schema.object({
        /** The app's installation on an external account. */
        installation: schema.string().min(1),
    }),
);

/** The apps a package installs on external accounts, whose tokens it issues to connections. */
export const Issuer = {
    /** Define a space's issuers, each called by the connection installations it relates. */
    definition() {
        return {
            name: "issuer",
            plural: "issuers",
            scope: space,
            fields: {},
            relations: {
                // the connection installations that mint the app's tokens
                consumer: { subjects: [principal.installation] },
            },
            // let the scope's own principal manage issuers, and their consumers call them
            permissions: {
                read: relation("consumer"),
                manage: none(),
                issue: relation("consumer"),
            },
        } as const;
    },

    /** Declare an issuing package's issuer type, as connection calls it under that package. */
    type(issuing: Package): ObjectType {
        return ObjectType.declare(
            { ...Issuer.definition(), methods: Issuer.methods(new MethodBuilder()) },
            { package: issuing },
        );
    },

    /** Declare an issuer's methods: issuing, uninstalling and reading installations. */
    methods: <Definition extends Table>(
        method: MethodBuilder<Definition, never, "read" | "manage" | "issue">,
    ) => ({
        create: method.create("manage", { fields: [], isPredicted: false }),
        delete: method.delete("manage"),
        /** Issue a token of an installation, narrowed to a grant. */
        issue: method.query({
            permission: "issue",
            input: schema.object({
                /** The app's installation on an external account. */
                installation: schema.string().min(1),
                /** What the token opens, all the installation opens when absent. */
                grant: InstallationGrant.exactOptional(),
            }),
            output: schema.sensitive(Token),
            prepared: schema.sensitive(Token),
            isAudited: true,
        }),
        /** Remove the app from the external account of an installation. */
        uninstall: method.query({
            permission: "issue",
            input: schema.object({
                /** The app's installation on an external account. */
                installation: schema.string().min(1),
            }),
            output: schema.object({}),
            prepared: schema.object({}),
            isAudited: true,
        }),
        /** Read the installation a completed installation page redirects back with. */
        installation: method.query({
            permission: "issue",
            input: schema.object({
                /** The redirect's query parameters. */
                parameters: schema.record(schema.string(), schema.string()),
            }),
            output: IssuedInstallation,
        }),
    }),
};
