import type { MachineQuote, MachineType, MachineVendor } from "./vendor.ts";

/** The page publishing Fly.io's Machine prices in Ashburn and Secaucus, its cheapest regions. */
const SOURCE = "https://fly.io/docs/about/pricing/";

/** The day the prices were read at their source. */
const OBSERVED_AT = "2026-10-12";

/** A performance CPU with its 2 GB of memory, $33.00 per 30 days, in US cents. */
const PERFORMANCE_CPU_MONTH = 3300;

/** The memory each performance CPU includes, in GB. */
const INCLUDED_MEMORY = 2;

/** The most memory a performance CPU takes, in GB, as the largest preset tier shows. */
const MAXIMUM_MEMORY = 8;

/** Memory beyond the included, $6.00 per GB per 30 days, in US cents. */
const MEMORY_GIGABYTE_MONTH = 600;

/** A volume's provisioned capacity, $0.15 per GB-month, in US cents. */
const VOLUME_GIGABYTE_MONTH = 15;

/** The CPUs of Fly.io's performance Machine presets, performance-1x to performance-16x. */
export const FLY_PERFORMANCE_CPUS: readonly number[] = [1, 2, 4, 6, 8, 10, 12, 14, 16];

/** Fly.io, a platform Destack's dedicated machines compare with. */
export const fly: MachineVendor = {
    /** Quote the smallest performance preset with the resources and a volume of their disk. */
    quote(wanted: MachineType): MachineQuote | undefined {
        // take the smallest preset with the CPUs and room for the memory
        const cpus = FLY_PERFORMANCE_CPUS.find(
            (count) => count >= wanted.vcpus && count * MAXIMUM_MEMORY >= wanted.memory,
        );
        if (cpus === undefined) {
            return undefined;
        }

        // price its CPUs, the memory beyond the included and the volume
        const memory = Math.max(0, wanted.memory - cpus * INCLUDED_MEMORY);
        const monthly =
            cpus * PERFORMANCE_CPU_MONTH +
            memory * MEMORY_GIGABYTE_MONTH +
            wanted.disk * VOLUME_GIGABYTE_MONTH;

        return {
            name: `performance-${cpus}x with ${wanted.memory} GB and a ${wanted.disk} GB volume`,
            monthly,
            source: SOURCE,
            observedAt: OBSERVED_AT,
        };
    },
};
