import type { MachineSize } from "@destack/account/object";
import { CREDITS, type Sku } from "@destack/finance";
import { defineSku } from "@destack/finance/declare";
import { machineHours } from "@destack/observability/meter";
import { hetzner } from "../vendor/hetzner.ts";
import type { MachineType, MachineVendor } from "../vendor/vendor.ts";

/** The page publishing Destack's machine prices. */
const SOURCE = "https://destack.sh/pricing";

/** The day the machine prices were set. */
const OBSERVED_AT = "2026-10-20";

/** A dedicated machine size: its resources, its credit cents per awake hour and its monthly cap. */
export type Machine = MachineType & { readonly hourly: number; readonly monthly: number };

/** Destack's dedicated machines by size, each capped near twice its cost on its host. */
export const MACHINES: Readonly<Record<MachineSize, Machine>> = {
    small: { vcpus: 2, memory: 4, disk: 80, hourly: 8, monthly: 4000 },
    medium: { vcpus: 4, memory: 8, disk: 160, hourly: 16, monthly: 8000 },
    large: { vcpus: 8, memory: 16, disk: 320, hourly: 32, monthly: 16_000 },
    xlarge: { vcpus: 16, memory: 32, disk: 640, hourly: 64, monthly: 32_000 },
};

/** The vendor hosting the dedicated machines, whose cost each cap doubles. */
export const MACHINE_HOST: MachineVendor = hetzner;

/** A small machine's awake hours, 0.08 credits each. */
export const smallMachineUnit = defineSku({
    name: "destack.machine.small",
    description:
        "Hours a small machine with two vCPUs, 4 GB of memory and 80 GB of disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineHours.reference,
    pricingUnit: "hour",
    unitSize: 1,
    currency: CREDITS,
    unitAmount: String(MACHINES.small.hourly),
    maximumQuantity: MACHINES.small.monthly / MACHINES.small.hourly,
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** A medium machine's awake hours, 0.16 credits each. */
export const mediumMachineUnit = defineSku({
    name: "destack.machine.medium",
    description:
        "Hours a medium machine with four vCPUs, 8 GB of memory and 160 GB of disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineHours.reference,
    pricingUnit: "hour",
    unitSize: 1,
    currency: CREDITS,
    unitAmount: String(MACHINES.medium.hourly),
    maximumQuantity: MACHINES.medium.monthly / MACHINES.medium.hourly,
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** A large machine's awake hours, 0.32 credits each. */
export const largeMachineUnit = defineSku({
    name: "destack.machine.large",
    description:
        "Hours a large machine with eight vCPUs, 16 GB of memory and 320 GB of disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineHours.reference,
    pricingUnit: "hour",
    unitSize: 1,
    currency: CREDITS,
    unitAmount: String(MACHINES.large.hourly),
    maximumQuantity: MACHINES.large.monthly / MACHINES.large.hourly,
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** An extra large machine's awake hours, 0.64 credits each. */
export const xlargeMachineUnit = defineSku({
    name: "destack.machine.xlarge",
    description:
        "Hours an extra large machine with sixteen vCPUs, 32 GB of memory and 640 GB of disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineHours.reference,
    pricingUnit: "hour",
    unitSize: 1,
    currency: CREDITS,
    unitAmount: String(MACHINES.xlarge.hourly),
    maximumQuantity: MACHINES.xlarge.monthly / MACHINES.xlarge.hourly,
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** The units dedicated machines bill their awake hours by, one per size. */
export const MACHINE_SKUS: Readonly<Record<MachineSize, Sku>> = {
    small: smallMachineUnit,
    medium: mediumMachineUnit,
    large: largeMachineUnit,
    xlarge: xlargeMachineUnit,
};
