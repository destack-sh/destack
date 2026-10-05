import type { ResidencyDefinition } from "@destack/account/object";

/** A region serving a residency: its code and display name. */
export interface PlatformRegion {
    /** The short code, such as eu-central. */
    readonly code: string;
    /** The display name. */
    readonly name: string;
}

/** A residency the platform serves with its regions and Durable Object location. */
export interface PlatformResidency extends ResidencyDefinition {
    /** The Durable Object jurisdiction, empty for none. */
    readonly jurisdiction: "" | "eu";
    /** The Durable Object location hint near the residency's database. */
    readonly location: string;
    /** The regions serving the residency. */
    readonly regions: readonly PlatformRegion[];
}

/** The residencies every universe keeps, each with its regions. */
export const RESIDENCIES: readonly PlatformResidency[] = [
    {
        code: "eu",
        name: "European Union",
        jurisdiction: "eu",
        location: "weur",
        regions: [{ code: "eu-central", name: "Europe" }],
    },
    {
        code: "us",
        name: "United States",
        jurisdiction: "",
        location: "enam",
        regions: [{ code: "us-east", name: "United States East" }],
    },
];
