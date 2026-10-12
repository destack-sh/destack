import type { MachineQuote, MachineType, MachineVendor } from "./vendor.ts";

/** The page publishing Railway's resource prices. */
const SOURCE = "https://docs.railway.com/reference/pricing/plans";

/** The day the prices were read at their source. */
const OBSERVED_AT = "2026-10-12";

/** A vCPU, $20 per month, in US cents. */
const VCPU_MONTH = 2000;

/** Memory, $10 per GB per month, in US cents. */
const MEMORY_GIGABYTE_MONTH = 1000;

/** Volume storage, $0.15 per GB per month, in US cents. */
const VOLUME_GIGABYTE_MONTH = 15;

/** Railway, a platform Destack's dedicated machines compare with. */
export const railway: MachineVendor = {
    /** Quote a service using the resources all month with a volume of their disk. */
    quote(wanted: MachineType): MachineQuote {
        return {
            name: `${wanted.vcpus} vCPUs, ${wanted.memory} GB and a ${wanted.disk} GB volume`,
            monthly:
                wanted.vcpus * VCPU_MONTH +
                wanted.memory * MEMORY_GIGABYTE_MONTH +
                wanted.disk * VOLUME_GIGABYTE_MONTH,
            source: SOURCE,
            observedAt: OBSERVED_AT,
        };
    },
};
