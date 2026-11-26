import { anyone, none, principal, relation } from "@destack/access";
import type { Select } from "@destack/db";
import type { DatabaseConnection, Snapshot } from "@destack/db/database";
import { defineObject, field } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { RequestId, Scope, Subject } from "@destack/sync";
import { type Identifier, schema } from "@destack/schema";
import { DisplayName } from "./display-name.ts";
import { AccountError } from "../error/error.ts";
import { argument, plural, t } from "@destack/locale";

/** The name the platform installs its merchant under. */
export const REGION_MERCHANT = "finance";

/** A region's code, a lowercase DNS label such as eu or us. */
export const RegionCode = schema.string().regex(/^[a-z]{2,16}$/u);
/** A region's code. */
export type RegionCode = schema.Infer<typeof RegionCode>;

/** A country by its ISO 3166-1 alpha-2 code, such as DE. */
export const CountryCode = schema.string().regex(/^[A-Z]{2}$/u);

/** A jurisdiction Destack stores data in. */
export const region = defineObject(
    {
        name: "region",
        plural: "regions",
        title: t`${plural(argument("count"), { one: "Region", other: "Regions" })}`,
        scope: "universe",
        key: "code",
        fields: {
            /** The code configurations and people call it by, such as eu. */
            code: field.string(RegionCode).immutable(),
            /** The name people read, such as European Union. */
            name: field.string(DisplayName),
            /** The origin its accounts' machines open their tunnels at, its relay's. */
            relay: field.string(schema.url()),
            /** The countries whose people are offered it first when signing up. */
            countries: field.json(schema.array(CountryCode)).default([]),
        },
        access: {
            relations: {
                // the installation selling to the region's accounts, and the space it sells from
                merchant: {
                    subjects: [principal.installation, principal.space],
                    grantedBy: "operate",
                },
                // everyone choosing a region while signing up
                public: { subjects: [anyone.all()] },
            },
            permissions: {
                read: relation("public"),
                sell: relation("merchant"),
                operate: none(),
            },
            reserved: ["sell"],
        },
    },
    (method) => ({
        /** Store a region the universe declares, shown to everyone. */
        create: method.create("operate", {
            fields: ["code", "name", "relay", "countries"],
            visibility: "package",
            isPredicted: false,
        }),

        /** Rename a region, or move its relay. */
        update: method.update("operate", {
            fields: ["name", "relay", "countries"],
            visibility: "package",
        }),

        /** Read the relay of an account's region. */
        relay: method.query({
            title: t.context("action")`Relay`,
            permission: "read",
            target: "scope",
            input: schema.object({
                /** The account. */
                account: schema.identifier("account"),
            }),
            output: schema.object({
                /** The origin its accounts' machines open their tunnels at. */
                url: schema.url(),
            }),
        }),
    }),
);
/** A jurisdiction Destack stores data in. */
export type Region = Select<typeof region.table>;

/** A region as the universe declares it: its code, name, relay and the countries offered it. */
export const RegionDefinition = schema.object({
    /** The code, the row's key. */
    code: RegionCode,
    /** The name people read. */
    name: DisplayName,
    /** The origin its accounts' machines open their tunnels at. */
    relay: schema.url(),
    /** The countries whose people are offered it first when signing up. */
    countries: schema.array(CountryCode),
});
/** A region as the universe declares it. */
export type RegionDefinition = schema.Infer<typeof RegionDefinition>;

/** The rows of regions. */
export const Region = {
    /** Read a region's merchant as a subject set: whoever sells to its accounts. */
    merchant(id: Identifier<"region">): Subject {
        return { ...region.reference(Scope.universe.id, id), relation: "sell" };
    },

    /** List every region, by code. */
    list(database: DatabaseConnection): Promise<Region[]> {
        return database
            .snapshot()
            .read(region.table)
            .findMany({ orderBy: { code: "asc" } });
    },

    /** Read a region by its identifier or code, and refuse an unknown one. */
    async require(reader: Pick<Snapshot, "read">, value: string): Promise<Region> {
        // read the region and refuse an unknown one
        const row = await reader
            .read(region.table)
            .findFirst({ where: { [region.identify(value)]: value } });
        if (row === undefined) {
            throw AccountError.of("REGION_NOT_FOUND", `region '${value}' not found`);
        }

        return row;
    },

    /** Apply the regions the universe declares: create new ones and rename others. */
    async apply(
        objects: Pick<ObjectStore, "database" | "as" | "mutate" | "environment">,
        definitions: readonly RegionDefinition[],
    ): Promise<void> {
        // read the stored regions
        const rows = await objects.database.snapshot().read(region.table).findMany();

        // create the missing ones and update the changed ones
        const calls = definitions.flatMap((definition) => {
            // find its row
            const row = rows.find((entry) => entry.code === definition.code);

            // create a missing region
            const { code, name, relay, countries } = definition;
            if (row === undefined) {
                return [
                    region.call("create", {
                        ...region.in(Scope.universe.id),
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
                return [region.call("update", { ...region.target(row), name, relay, countries })];
            }
            // leave an unchanged one
            else {
                return [];
            }
        });
        if (calls.length > 0) {
            await objects.mutate(objects.as(region), {
                id: RequestId.create(objects.environment),
                calls,
            });
        }
    },
};
