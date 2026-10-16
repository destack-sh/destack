import { anyone, none, principal, relation, Relationship, union } from "@destack/access";
import { type DatabaseConnection, type Select, Snapshot } from "@destack/db";
import { defineObject, field } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { RequestId } from "@destack/runtime/request";
import { defineSchema, type Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Scope, Subject } from "@destack/sync";
import { AccountHandle } from "../address/handle.ts";
import { DisplayName } from "./display-name.ts";

/** A residency's code, a lowercase DNS label such as eu or us. */
export const ResidencyCode = defineSchema(schema.string().regex(/^[a-z]{2,16}$/u));
/** A residency's code. */
export type ResidencyCode = schema.Infer<typeof ResidencyCode>;

/** A cloud machine serving a residency, with the URL it answers at. */
export const ServingMachine = defineSchema(
    schema.object({
        /** The cloud machine. */
        machine: schema.identifier("machine"),
        /** The URL it answers at. */
        url: schema.url(),
    }),
);
/** A cloud machine serving a residency, with the URL it answers at. */
export type ServingMachine = schema.Infer<typeof ServingMachine>;

/** A residency as an account finds it: its code, name and home. */
export const ResidencyFound = defineSchema(
    schema.object({
        /** The residency's code. */
        id: ResidencyCode,
        /** The name people read. */
        name: DisplayName,
        /** The space serving the residency's platform installations. */
        home: schema.identifier("space").nullable(),
    }),
);
/** A residency as an account finds it. */
export type ResidencyFound = schema.Infer<typeof ResidencyFound>;

/** The relations of a residency's servers: cloud machines, its home and the installations in it. */
const SERVERS = ["machine", "home", "installation"] as const;

/** A jurisdiction Destack stores data in, which the platform's installations in it serve. */
export const residency = defineObject(
    {
        name: "residency",
        plural: "residencies",
        scope: "universe",
        key: ResidencyCode,
        fields: {
            /** The name people read, such as European Union. */
            name: field.string(DisplayName),
            /** The space serving the residency's platform installations. */
            home: field.string(schema.identifier("space")).optional().guard({ read: "locate" }),
        },
        relations: {
            // the residency's cloud machines, its home and the platform installations in its home
            machine: { subjects: [principal.machine], grantedBy: "operate" },
            home: { subjects: [principal.space], grantedBy: "operate" },
            installation: {
                subjects: [principal.installation, principal.installation.all()],
                grantedBy: "operate",
            },
            // everyone choosing a residency while signing up
            public: { subjects: [anyone.all()] },
        },
        permissions: {
            read: relation("public"),
            server: union(relation("machine"), relation("home"), relation("installation")),
            // let its cloud machines and the installations in its home claim names in its accounts
            claim: union(relation("machine"), relation("installation")),
            locate: none(),
            operate: none(),
        },
        reserved: ["server", "claim"],
    },
    (method) => ({
        /** Store a residency the universe declares, shown to everyone. */
        create: method.create("operate", {
            fields: ["name"],
            isInternal: true,
            isPredicted: false,
        }),

        /** Rename a residency, or give it its home. */
        update: method.update("operate", { fields: ["name", "home"], isInternal: true }),

        /** Find the residency an account stores its data in. */
        find: method.query({
            permission: "locate",
            target: "scope",
            input: schema.object({
                /** The account. */
                account: schema.identifier("account"),
            }),
            output: ResidencyFound.nullable(),
        }),

        /** List the residency's enabled shared cloud machines and URLs, in identity order. */
        machines: method.query({
            permission: "locate",
            output: schema.array(ServingMachine),
        }),

        /** Find the account selling a handle's packages to the buyers of a residency's home. */
        publisher: method.query({
            permission: "locate",
            target: "scope",
            input: schema.object({
                /** The handle. */
                handle: AccountHandle,
                /** The residency's home. */
                home: schema.identifier("space"),
            }),
            output: schema.identifier("account").nullable(),
        }),
    }),
);
/** A jurisdiction Destack stores data in. */
export type Residency = Select<typeof residency.table>;

/** A residency as the universe declares it: its code and the name people read. */
export interface ResidencyDefinition {
    /** The code, the row's key. */
    readonly code: ResidencyCode;
    /** The name people read. */
    readonly name: string;
}

/** The rows of residencies. */
export const Residency = {
    /** Read the principals serving a residency as a subject set. */
    servers(code: ResidencyCode): Subject {
        return { ...residency.reference(Scope.universe.id, code), relation: "server" };
    },

    /** Read the platform installations claiming names in a residency's accounts, as a set. */
    claimants(code: ResidencyCode): Subject {
        return { ...residency.reference(Scope.universe.id, code), relation: "claim" };
    },

    /** List every residency by code. */
    async list(database: DatabaseConnection): Promise<Pick<Residency, "id" | "name">[]> {
        return database
            .select({ id: residency.table.id, name: residency.table.name })
            .from(residency.table)
            .orderBy(residency.table.id);
    },

    /** Read the residency a platform installation or cloud machine serves, if any. */
    async of(database: DatabaseConnection, server: Subject): Promise<ResidencyCode | undefined> {
        const relationships = await Relationship.readBySubject(Snapshot.live(database), [server]);
        const found = relationships.find(
            (row) =>
                row.objectPackageId === residency.typeReference.packageId &&
                row.objectType === residency.typeReference.type &&
                SERVERS.some((kind) => kind === row.relation),
        );

        return found === undefined ? undefined : ResidencyCode.parse(found.objectId);
    },

    /** Report whether a space is a residency's home, the platform's account's home in it. */
    async isHome(reader: Pick<Snapshot, "read">, spaceId: string): Promise<boolean> {
        const found = await reader.read(residency.table).findFirst({
            where: {
                home: schema.identifier("space").parse(spaceId),
            },
        });

        return found !== undefined;
    },

    /** Read a residency's home, and refuse a residency without one. */
    async home(
        residencies: {
            readonly get: (input: {
                readonly id: ResidencyCode;
            }) => Promise<{ readonly home?: Identifier<"space"> | null }>;
        },
        code: ResidencyCode,
    ): Promise<Identifier<"space">> {
        // refuse a home hidden from the reader
        const { home } = await residencies.get({ id: code });
        if (home === undefined) {
            throw new CallError("FORBIDDEN", { message: `the home of ${code} is hidden` });
        }
        // refuse a residency without a home
        else if (home === null) {
            throw new CallError("NOT_FOUND", {
                message: `no home serves the residency ${code}`,
            });
        }

        return home;
    },

    /** Read a residency, and refuse an unknown code. */
    async require(reader: Pick<Snapshot, "read">, code: string): Promise<Residency> {
        // read the residency and refuse an unknown code
        const row = await reader
            .read(residency.table)
            .findFirst({ where: { id: ResidencyCode.parse(code) } });
        if (row === undefined) {
            throw new CallError("NOT_FOUND", { message: `no residency ${code}` });
        }

        return row;
    },

    /** Apply the residencies the universe declares: create new ones and rename others. */
    async apply(
        objects: Pick<ObjectStore, "database" | "self" | "mutate">,
        definitions: readonly ResidencyDefinition[],
        now: number,
    ): Promise<void> {
        // read the stored residencies
        const rows = await Snapshot.live(objects.database).read(residency.table).findMany();

        // create the missing ones and rename the renamed ones
        const calls = definitions.flatMap((definition) => {
            // find its row
            const row = rows.find((entry) => entry.id === definition.code);
            const target = residency.target({ scope: Scope.universe.id, id: definition.code });

            // create a missing residency
            if (row === undefined) {
                return [residency.call("create", { ...target, name: definition.name })];
            }
            // rename a renamed one
            else if (row.name !== definition.name) {
                return [residency.call("update", { ...target, name: definition.name })];
            }

            return [];
        });
        if (calls.length > 0) {
            await objects.mutate(objects.self, {
                id: RequestId.create(now),
                calls,
            });
        }
    },
};
