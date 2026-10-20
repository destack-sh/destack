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
    readonly quote: (wanted: MachineType) => MachineQuote | undefined;
}
