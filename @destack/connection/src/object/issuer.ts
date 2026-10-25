import { type AccessExpression, none, type RelationInput } from "@destack/access";
import type { Table } from "@destack/db";
import { space } from "@destack/account/object";
import { MethodBuilder, ObjectType } from "@destack/object";
import type { Package } from "@destack/package";
import { type DigestCrypto, Identifier, schema, type Uuid } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { Token } from "../credential/credential.ts";

/** Destack's issuer namespace: the DNS namespace's UUIDv8 of issuer.destack.app. */
const ISSUER_NAMESPACE: Uuid = "d8bd2c32-e5f0-8339-b3c3-547bc2215fba";

/** What an installation token opens: some of the installation's resources and permissions. */
export const InstallationGrant = schema.object({
    /** The resources the token opens, such as repositories, by identifier or name. */
    resources: schema
        .union([
            schema.object({ ids: schema.array(schema.string().min(1)) }),
            schema.object({ names: schema.array(schema.string().min(1)) }),
        ])
        .exactOptional(),
    /** The permissions the token has, by name, such as contents read. */
    permissions: schema.record(schema.string().min(1), schema.string().min(1)).exactOptional(),
});
/** What an installation token opens. */
export type InstallationGrant = schema.Infer<typeof InstallationGrant>;

/** Who reads, manages and calls an issuer: its relations and its three permissions. */
interface IssuerAccess {
    /** The relations deciding the permissions. */
    readonly relations: Readonly<Record<string, RelationInput>>;
    /** Who reads, manages and calls the issuer. */
    readonly permissions: Readonly<Record<"read" | "manage" | "issue", AccessExpression>>;
}

/** The installation a completed installation page redirects back with. */
export const IssuedInstallation = schema.object({
    /** The app's installation on an external account. */
    installation: schema.string().min(1),
});

/** The apps a package installs on external accounts, whose tokens it issues to connections. */
export const Issuer = {
    /** Define a space's issuers with the access deciding who reads, manages and calls them. */
    definition<const Access extends IssuerAccess>(
        access: Access,
    ): {
        readonly name: "issuer";
        readonly plural: "issuers";
        readonly scope: typeof space;
        readonly fields: Record<never, never>;
    } & Access {
        return { name: "issuer", plural: "issuers", scope: space, fields: {}, ...access };
    },

    /** Declare an issuing package's issuer type, as connection calls it under that package. */
    type(issuing: Package): ObjectType {
        const access = {
            relations: {},
            permissions: { read: none(), manage: none(), issue: none() },
        };

        return ObjectType.declare(
            { ...Issuer.definition(access), methods: Issuer.methods(new MethodBuilder()) },
            { package: issuing },
        );
    },

    /** Name the issuer a scope's stack declares under a name, as its integrations reference it. */
    declared(
        scope: Pick<ObjectReference, "id">,
        name: string,
        crypto: DigestCrypto,
    ): Promise<Identifier<"issuer">> {
        return Identifier.hash("issuer", ISSUER_NAMESPACE, `${scope.id}/declared/${name}`, crypto);
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
        uninstall: method.mutation({
            permission: "issue",
            input: schema.object({
                /** The app's installation on an external account. */
                installation: schema.string().min(1),
            }),
            output: schema.object({}),
            prepared: schema.object({}),
            isAudited: true,
        }),
        /** Claim the installation a completed installation page redirects back with. */
        installation: method.mutation({
            permission: "issue",
            input: schema.object({
                /** The redirect's query parameters. */
                parameters: schema.record(schema.string(), schema.string()),
            }),
            output: IssuedInstallation,
            isAudited: true,
        }),
    }),
};
