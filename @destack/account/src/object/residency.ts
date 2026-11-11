import {
    anyone,
    none,
    type PermissionReference,
    principal,
    relation,
    Relationship,
    union,
} from "@destack/access";
import { type DatabaseConnection, type Select, Snapshot } from "@destack/db";
import { defineObject, field, InstallationName } from "@destack/object";
import { PackageId } from "@destack/package";
import type { ObjectStore } from "@destack/object/server";
import { RequestId, Scope, Subject } from "@destack/sync";
import { type Identifier, present, schema, Version } from "@destack/schema";
import { AccountHandle } from "../address/handle.ts";
import { DisplayName } from "./display-name.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The declaration name a residency space's stack installs the residency's relay under. */
export const RESIDENCY_RELAY = "relay";

/** A residency's code, a lowercase DNS label such as eu or us. */
export const ResidencyCode = schema.string().regex(/^[a-z]{2,16}$/u);
/** A residency's code. */
export type ResidencyCode = schema.Infer<typeof ResidencyCode>;

/** A country by its ISO 3166-1 alpha-2 code, such as DE. */
export const CountryCode = schema.string().regex(/^[A-Z]{2}$/u);

/** A cloud machine serving a residency, with the URL it answers at. */
export const ServingMachine = schema.object({
    /** The cloud machine. */
    machine: schema.identifier("machine"),
    /** The URL it answers at. */
    url: schema.url(),
});
/** A cloud machine serving a residency, with the URL it answers at. */
export type ServingMachine = schema.Infer<typeof ServingMachine>;

/** An installation a residency's space declares: its identifier, name and release. */
export const ResidencyInstallation = schema.object({
    /** The installation, named by its space and its name. */
    installation: schema.identifier("installation"),
    /** The installation's name in the space's stack. */
    name: InstallationName,
    /** The installed package. */
    packageId: PackageId,
    /** The release the installation follows. */
    release: Version,
});
/** An installation a residency's space declares. */
export type ResidencyInstallation = schema.Infer<typeof ResidencyInstallation>;

/** The relations of a residency's servers: its cloud machines and its space. */
const SERVERS = ["machine", "space"] as const;

/** A jurisdiction Destack stores data in, served by its cloud machines and its space. */
export const residency = defineObject(
    {
        name: "residency",
        plural: "residencies",
        scope: "universe",
        key: "code",
        fields: {
            /** The code people and configurations name it by, such as eu. */
            code: field.string(ResidencyCode).immutable(),
            /** The name people read, such as European Union. */
            name: field.string(DisplayName),
            /** The origin its accounts' machines open their tunnels at, its relay's. */
            relay: field.string(schema.url()),
            /** The countries whose people are offered it first when signing up. */
            countries: field.json(schema.array(CountryCode)).default([]),
            /** The space hosting the residency's platform installations. */
            space: field.string(schema.identifier("space")).optional().guard({ read: "locate" }),
        },
        relations: {
            // the residency's cloud machines and its space
            machine: { subjects: [principal.machine], grantedBy: "operate" },
            space: { subjects: [principal.space], grantedBy: "operate" },
            // the residency's relay, the installation its space's stack declares, related by the universe
            relay: { subjects: [principal.installation], grantedBy: "operate" },
            // the installations its space's stack declares, related at its space's request
            installation: { subjects: [principal.installation], grantedBy: "operate" },
            // everyone choosing a residency while signing up
            public: { subjects: [anyone.all()] },
            // the contents of the accounts storing their data in it, related as each account is created
            resident: { subjects: ["account#contained"], grantedBy: "place" },
        },
        permissions: {
            read: relation("public"),
            server: union(relation("machine"), relation("space")),
            relaying: relation("relay"),
            // let the installations its space declares serve its accounts
            installed: relation("installation"),
            // let its space relate the installations its stack declares
            install: relation("space"),
            // let its cloud machines and its space claim names in its accounts
            claim: union(relation("machine"), relation("space")),
            // let every signed-in caller read its space and the accounts selling to its buyers, as the universe's signed-in role grants
            locate: none(),
            // let the contents of its accounts act as its residents, such as their spaces' installations
            reside: relation("resident"),
            operate: none(),
            place: none(),
        },
        reserved: ["server", "relaying", "claim", "installed", "install", "reside", "place"],
    },
    (method) => ({
        /** Store a residency the universe declares, shown to everyone. */
        create: method.create("operate", {
            fields: ["code", "name", "relay", "countries"],
            visibility: "package",
            isPredicted: false,
        }),

        /** Rename a residency, or give it its space. */
        update: method.update("operate", {
            fields: ["name", "relay", "countries", "space"],
            visibility: "package",
        }),

        /** Relate an installation its space declares, its release asking for what that gives. */
        touch: method.mutation({ permission: "install", input: ResidencyInstallation }),

        /** Unrelate an installation its space no longer declares. */
        revoke: method.mutation({
            permission: "install",
            input: ResidencyInstallation.pick({ installation: true, name: true }),
        }),

        /** Locate the space serving the residency an account stores its data in. */
        locate: method.query({
            permission: "locate",
            target: "scope",
            input: schema.object({
                /** The account. */
                account: schema.identifier("account"),
            }),
            output: schema.identifier("space"),
        }),

        /** Read the relay origin of the residency an account stores its data in. */
        relay: method.query({
            permission: "locate",
            target: "scope",
            input: schema.object({
                /** The account. */
                account: schema.identifier("account"),
            }),
            output: schema.url(),
        }),

        /** List the residency's enabled shared cloud machines and URLs, in identity order. */
        machines: method.query({
            permission: "locate",
            output: schema.array(ServingMachine),
        }),

        /** Find the account selling a handle's packages to the buyers of the asker's residency. */
        publisher: method.query({
            permission: "locate",
            target: "scope",
            input: schema.object({
                /** The handle. */
                handle: AccountHandle,
            }),
            output: schema.identifier("account").nullable(),
        }),
    }),
);
/** A jurisdiction Destack stores data in. */
export type Residency = Select<typeof residency.table>;

/** A residency as the universe declares it: its code, name, relay and the countries offered it. */
export interface ResidencyDefinition {
    /** The code, the row's key. */
    readonly code: ResidencyCode;
    /** The name people read. */
    readonly name: string;
    /** The origin its accounts' machines open their tunnels at. */
    readonly relay: string;
    /** The countries whose people are offered it first when signing up. */
    readonly countries: readonly string[];
}

/** The rows of residencies. */
export const Residency = {
    /** Read the principals serving a residency as a subject set. */
    servers(id: Identifier<"residency">): Subject {
        return { ...residency.reference(Scope.universe.id, id), relation: "server" };
    },

    /** List the universe permissions the installation relation gives an installation. */
    installedPermissions(): PermissionReference[] {
        const { packageId } = residency.typeReference;

        return [
            { packageId, type: "account", name: "serve" },
            { packageId, type: "account", name: "entitle" },
            { packageId, type: "residency", name: "locate" },
        ];
    },

    /** Read the installations a residency's space declares, which serve its accounts, as a set. */
    installations(id: Identifier<"residency">): Subject {
        return { ...residency.reference(Scope.universe.id, id), relation: "installed" };
    },

    /** Read a residency's relay as a subject set, which publishes the tunnels it accepts. */
    relays(id: Identifier<"residency">): Subject {
        return { ...residency.reference(Scope.universe.id, id), relation: "relaying" };
    },

    /** Read the principals claiming names in a residency's accounts, as a set. */
    claimants(id: Identifier<"residency">): Subject {
        return { ...residency.reference(Scope.universe.id, id), relation: "claim" };
    },

    /** List every residency, by code. */
    list(database: DatabaseConnection): Promise<Residency[]> {
        return Snapshot.live(database)
            .read(residency.table)
            .findMany({ orderBy: { code: "asc" } });
    },

    /** Read the residency a cloud machine or residency space serves, if any. */
    async of(database: DatabaseConnection, server: Subject): Promise<ResidencyCode | undefined> {
        // find the residency the server's relationship relates it to as a server, none for none
        const relationships = await Relationship.readBySubject(Snapshot.live(database), [server]);
        const found = relationships.find(
            (row) =>
                row.objectPackageId === residency.typeReference.packageId &&
                row.objectType === residency.typeReference.type &&
                SERVERS.some((kind) => kind === row.relation),
        );
        if (found === undefined) {
            return undefined;
        }

        // read the code of the residency it serves
        const served = await Snapshot.live(database)
            .read(residency.table)
            .findFirst({ where: { id: residency.identifier(found.objectId) } });

        return present(served, `residency ${found.objectId}`).code;
    },

    /** Read a residency's space by its identifier or code, and refuse a residency without one. */
    async space(reader: Pick<Snapshot, "read">, value: string): Promise<Identifier<"space">> {
        return Residency.requireSpace(await Residency.require(reader, value));
    },

    /** Read a residency row's space, refusing a residency the platform does not serve yet. */
    requireSpace(row: {
        readonly code: string;
        readonly space?: Identifier<"space"> | null;
    }): Identifier<"space"> {
        if (row.space === null || row.space === undefined) {
            throw AccountError.of(
                "RESIDENCY_SPACE_NOT_FOUND",
                `space of residency '${row.code}' not found`,
            );
        }

        return row.space;
    },

    /** Read a residency by its identifier or code, and refuse an unknown one. */
    async require(reader: Pick<Snapshot, "read">, value: string): Promise<Residency> {
        // read the residency and refuse an unknown one
        const row = await reader
            .read(residency.table)
            .findFirst({ where: { [residency.identify(value)]: value } });
        if (row === undefined) {
            throw AccountError.of("RESIDENCY_NOT_FOUND", `residency '${value}' not found`);
        }

        return row;
    },

    /** Apply the residencies the universe declares: create new ones and rename others. */
    async apply(
        objects: Pick<ObjectStore, "database" | "as" | "mutate" | "environment">,
        definitions: readonly ResidencyDefinition[],
    ): Promise<void> {
        // read the stored residencies
        const rows = await Snapshot.live(objects.database).read(residency.table).findMany();

        // create the missing ones and update the changed ones
        const calls = definitions.flatMap((definition) => {
            // find its row
            const row = rows.find((entry) => entry.code === definition.code);

            // create a missing residency
            const { code, name, relay } = definition;
            const countries = [...definition.countries];
            if (row === undefined) {
                return [
                    residency.call("create", {
                        ...residency.in(Scope.universe.id),
                        code,
                        name,
                        relay,
                        countries,
                    }),
                ];
            }
            // update one whose name, relay or countries changed
            else if (
                row.name !== name ||
                row.relay !== relay ||
                row.countries.join() !== countries.join()
            ) {
                return [
                    residency.call("update", { ...residency.target(row), name, relay, countries }),
                ];
            }

            return [];
        });
        if (calls.length > 0) {
            await objects.mutate(objects.as(residency), {
                id: RequestId.create(objects.environment),
                calls,
            });
        }
    },
};
