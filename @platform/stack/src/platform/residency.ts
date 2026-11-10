import type { ResidencyCode } from "@destack/account/object";
import type { Jurisdiction } from "@destack/db/cloudflare";
import { type Identifier, schema } from "@destack/schema";

/** A residency Destack serves. */
export interface PlatformResidency {
    /** The code people choose, such as eu. */
    readonly code: ResidencyCode;
    /** The display name. */
    readonly name: string;
    /** The jurisdiction keeping its objects and files. */
    readonly jurisdiction: Jurisdiction;
    /** The countries whose people are offered it first when signing up, ISO 3166-1 alpha-2. */
    readonly countries: readonly string[];
    /** The Durable Object location hint near its data, such as weur. */
    readonly location: string;
    /** The cloud machine serving it: its identifier and its name. */
    readonly machine: {
        readonly id: Identifier<"machine">;
        readonly name: string;
    };
    /** The export of its home's stack in production, with the edge's routes. */
    readonly stack: "RESIDENCY_STACK_EU" | "RESIDENCY_STACK_US";
}

/** The residencies Destack serves, each with its cloud machine. */
export const RESIDENCIES: readonly PlatformResidency[] = [
    {
        code: "eu",
        name: "European Union",
        jurisdiction: "eu",
        // the European Union and the European Economic Area, with Switzerland and the United Kingdom
        countries: [
            "AT",
            "BE",
            "BG",
            "CH",
            "CY",
            "CZ",
            "DE",
            "DK",
            "EE",
            "ES",
            "FI",
            "FR",
            "GB",
            "GR",
            "HR",
            "HU",
            "IE",
            "IS",
            "IT",
            "LI",
            "LT",
            "LU",
            "LV",
            "MT",
            "NL",
            "NO",
            "PL",
            "PT",
            "RO",
            "SE",
            "SI",
            "SK",
        ],
        location: "weur",
        machine: {
            id: schema.identifier("machine").parse("machine-01a11e60-7834-77f0-ad65-3470b05f0b39"),
            name: "eu-central",
        },
        stack: "RESIDENCY_STACK_EU",
    },
    {
        code: "us",
        name: "United States",
        jurisdiction: "us",
        countries: ["US"],
        location: "enam",
        machine: {
            id: schema.identifier("machine").parse("machine-01a11e60-7834-77f0-ad65-3a267795a630"),
            name: "us-east",
        },
        stack: "RESIDENCY_STACK_US",
    },
];
