import type { MachineSize } from "@destack/account/object";
import { CREDITS, type Sku } from "@destack/finance/declare";
import { defineSku } from "@destack/finance/declare";
import { machineTime } from "@destack/telemetry/meter";
import { hetzner } from "../vendor/hetzner.ts";
import type { MachineType, MachineVendor } from "../vendor/vendor.ts";

/** The page publishing Destack's machine prices. */
const SOURCE = "https://destack.sh/pricing";

/** The day the machine prices were set. */
const OBSERVED_AT = "2026-10-20";

/** The seconds in an hour, the unit machine prices are set per. */
const SECONDS_PER_HOUR = 3600;

/** A dedicated machine size: resources, disk cleared on sleep, and credit cents an hour. */
export type Machine = MachineType & { readonly hourly: number };

/** Destack's dedicated machines by size, 500 awake hours costing twice its share on its host. */
export const MACHINES: Readonly<Record<MachineSize, Machine>> = {
    small: { vcpus: 2, memory: 4, disk: 80, hourly: 8 },
    medium: { vcpus: 4, memory: 8, disk: 160, hourly: 16 },
    large: { vcpus: 8, memory: 16, disk: 320, hourly: 32 },
    xlarge: { vcpus: 16, memory: 32, disk: 640, hourly: 64 },
};

/** The vendor hosting the dedicated machines, whose cost each hour's price is set against. */
export const MACHINE_HOST: MachineVendor = hetzner;

/** A small machine's awake seconds, 0.08 credits an hour. */
export const smallMachineUnit = defineSku({
    name: "destack.machine.small",
    description:
        "Seconds a small machine with two vCPUs, 4 GB of memory and 80 GB of local disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineTime.reference,
    attributes: {},
    pricingUnit: "hour",
    unitSize: SECONDS_PER_HOUR,
    currency: CREDITS,
    unitAmount: String(MACHINES.small.hourly),
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** A medium machine's awake seconds, 0.16 credits an hour. */
export const mediumMachineUnit = defineSku({
    name: "destack.machine.medium",
    description:
        "Seconds a medium machine with four vCPUs, 8 GB of memory and 160 GB of local disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineTime.reference,
    attributes: {},
    pricingUnit: "hour",
    unitSize: SECONDS_PER_HOUR,
    currency: CREDITS,
    unitAmount: String(MACHINES.medium.hourly),
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** A large machine's awake seconds, 0.32 credits an hour. */
export const largeMachineUnit = defineSku({
    name: "destack.machine.large",
    description:
        "Seconds a large machine with eight vCPUs, 16 GB of memory and 320 GB of local disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineTime.reference,
    attributes: {},
    pricingUnit: "hour",
    unitSize: SECONDS_PER_HOUR,
    currency: CREDITS,
    unitAmount: String(MACHINES.large.hourly),
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** An extra large machine's awake seconds, 0.64 credits an hour. */
export const xlargeMachineUnit = defineSku({
    name: "destack.machine.xlarge",
    description:
        "Seconds an extra large machine with sixteen vCPUs, 32 GB of memory and 640 GB of local disk stays awake.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    tenancy: "dedicated",
    meter: machineTime.reference,
    attributes: {},
    pricingUnit: "hour",
    unitSize: SECONDS_PER_HOUR,
    currency: CREDITS,
    unitAmount: String(MACHINES.xlarge.hourly),
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** The units dedicated machines bill their awake seconds by, one per size. */
export const MACHINE_SKUS: Readonly<Record<MachineSize, Sku>> = {
    small: smallMachineUnit,
    medium: mediumMachineUnit,
    large: largeMachineUnit,
    xlarge: xlargeMachineUnit,
};
