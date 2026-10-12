import type { Sku } from "@destack/finance";
import type { PricingModel } from "../price/model.ts";
import { AWS_PRICING, AWS_SKUS } from "./aws.ts";
import { AZURE_PRICING, AZURE_SKUS } from "./azure.ts";
import { CLOUDFLARE_PRICING, CLOUDFLARE_SKUS } from "./cloudflare.ts";
import { GCP_PRICING, GCP_SKUS } from "./gcp.ts";

/** The providers whose list prices Destack keeps: the one it runs on, then the hyperscalers. */
export const VENDORS = ["cloudflare", "aws", "azure", "gcp"] as const;

/** A provider whose list prices Destack keeps. */
export type Vendor = (typeof VENDORS)[number];

/** The hyperscalers Destack's prices sit at or below, every vendor but the one it runs on. */
export const HYPERSCALERS = ["aws", "azure", "gcp"] as const satisfies readonly Vendor[];

/** Each vendor's list prices as SKUs, data the workloads and scripts read. */
export const VENDOR_SKUS: Readonly<Record<Vendor, readonly Sku[]>> = {
    cloudflare: CLOUDFLARE_SKUS,
    aws: AWS_SKUS,
    azure: AZURE_SKUS,
    gcp: GCP_SKUS,
};

/** Each vendor's pricing model, its SKUs per unit of each meter it prices. */
export const VENDOR_PRICING: Readonly<Record<Vendor, PricingModel>> = {
    cloudflare: CLOUDFLARE_PRICING,
    aws: AWS_PRICING,
    azure: AZURE_PRICING,
    gcp: GCP_PRICING,
};

/** A machine type: its dedicated vCPUs, and its memory and disk in GB. */
export interface MachineType {
    /** The dedicated vCPUs, each one hardware thread. */
    readonly vcpus: number;
    /** The memory in GB. */
    readonly memory: number;
    /** The disk in GB. */
    readonly disk: number;
}

/** A vendor's cheapest machine with at least some resources, at its monthly list price. */
export interface MachineQuote {
    /** What the vendor sells, such as an instance type with a volume. */
    readonly name: string;
    /** The monthly list price in US cents. */
    readonly monthly: number;
    /** The page publishing the price. */
    readonly source: string;
    /** The day the price was read at its source. */
    readonly observedAt: string;
}

/** A vendor quoting machines, absent where it sells none as large. */
export interface MachineVendor {
    /** Quote the cheapest machine with at least some resources. */
    quote(wanted: MachineType): MachineQuote | undefined;
}
