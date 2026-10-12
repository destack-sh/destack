import type { MachineQuote, MachineType, MachineVendor } from "./vendor.ts";

/** The page publishing Render's service instance types and disk price. */
const SOURCE = "https://render.com/pricing";

/** The day the prices were read at their source. */
const OBSERVED_AT = "2026-10-12";

/** A persistent disk's price, $0.25 per GB-month, in US cents. */
const DISK_GIGABYTE_MONTH = 25;

/** A Render instance type for web services, private services and workers. */
export interface RenderInstanceType {
    /** The instance type, such as 2c-4g. */
    readonly name: string;
    /** The CPUs. */
    readonly vcpus: number;
    /** The memory in GB. */
    readonly memory: number;
    /** The monthly price in US cents. */
    readonly monthly: number;
}

/** Render's paid instance types with at least one CPU. */
export const RENDER_INSTANCE_TYPES: readonly RenderInstanceType[] = [
    { name: "1c-2g", vcpus: 1, memory: 2, monthly: 2500 },
    { name: "2c-4g", vcpus: 2, memory: 4, monthly: 8500 },
    { name: "2c-8g", vcpus: 2, memory: 8, monthly: 13_500 },
    { name: "2c-16g", vcpus: 2, memory: 16, monthly: 20_000 },
    { name: "4c-8g", vcpus: 4, memory: 8, monthly: 17_500 },
    { name: "4c-16g", vcpus: 4, memory: 16, monthly: 22_500 },
    { name: "4c-32g", vcpus: 4, memory: 32, monthly: 35_000 },
    { name: "8c-16g", vcpus: 8, memory: 16, monthly: 30_000 },
    { name: "8c-32g", vcpus: 8, memory: 32, monthly: 45_000 },
    { name: "8c-64g", vcpus: 8, memory: 64, monthly: 100_000 },
    { name: "12c-24g", vcpus: 12, memory: 24, monthly: 45_000 },
    { name: "12c-48g", vcpus: 12, memory: 48, monthly: 80_000 },
    { name: "12c-96g", vcpus: 12, memory: 96, monthly: 150_000 },
];

/** Render, a platform Destack's dedicated machines compare with. */
export const render: MachineVendor = {
    /** Quote the cheapest instance type with the resources and a disk of their size. */
    quote(wanted: MachineType): MachineQuote | undefined {
        const fitting = RENDER_INSTANCE_TYPES.filter(
            (type) => type.vcpus >= wanted.vcpus && type.memory >= wanted.memory,
        );
        const [cheapest] = fitting.toSorted((left, right) => left.monthly - right.monthly);

        return cheapest === undefined
            ? undefined
            : {
                  name: `${cheapest.name} with a ${wanted.disk} GB disk`,
                  monthly: cheapest.monthly + wanted.disk * DISK_GIGABYTE_MONTH,
                  source: SOURCE,
                  observedAt: OBSERVED_AT,
              };
    },
};
