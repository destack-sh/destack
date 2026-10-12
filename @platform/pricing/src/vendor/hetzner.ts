import type { MachineQuote, MachineType, MachineVendor } from "./vendor.ts";

/** The page publishing the prices since Hetzner's adjustment of 15 June 2026. */
const PRICES = "https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/";

/** The day the prices were read at their source. */
const OBSERVED_AT = "2026-10-12";

/** The months a dedicated server's setup fee is spread over: a year, the shortest a server runs. */
const SETUP_MONTHS = 12;

/** A Hetzner server in Germany or Finland, in US dollars without VAT. */
export interface HetznerServer {
    /** The plan, such as CCX13 or AX102-1. */
    readonly name: string;
    /** A cloud server sold whole, or a dedicated server carved into machines. */
    readonly kind: "cloud" | "dedicated";
    /** The dedicated vCPUs, each one hardware thread. */
    readonly vcpus: number;
    /** The memory in GB. */
    readonly memory: number;
    /** The usable disk in GB, one drive of a dedicated server's RAID 1 pair. */
    readonly disk: number;
    /** The monthly price in US cents. */
    readonly monthly: number;
    /** The setup fee in US cents. */
    readonly setup: number;
    /** The page publishing the server's resources. */
    readonly source: string;
}

/** Hetzner's servers with dedicated vCPUs in Germany and Finland, cloud then dedicated. */
export const HETZNER_SERVERS: readonly HetznerServer[] = [
    // general purpose cloud servers, CCX
    ...[
        { name: "CCX13", vcpus: 2, memory: 8, disk: 80, monthly: 5049 },
        { name: "CCX23", vcpus: 4, memory: 16, disk: 160, monthly: 10_149 },
        { name: "CCX33", vcpus: 8, memory: 32, disk: 240, monthly: 16_299 },
        { name: "CCX43", vcpus: 16, memory: 64, disk: 360, monthly: 32_549 },
        { name: "CCX53", vcpus: 32, memory: 128, disk: 600, monthly: 62_949 },
        { name: "CCX63", vcpus: 48, memory: 192, disk: 960, monthly: 100_699 },
    ].map((server) => ({
        ...server,
        kind: "cloud" as const,
        setup: 0,
        source: "https://www.hetzner.com/cloud/general-purpose/",
    })),

    // dedicated servers: Ryzen 7 PRO 8700GE with 8 cores, Ryzen 9 7950X3D with 16, two threads each
    {
        name: "AX42-1",
        kind: "dedicated",
        vcpus: 16,
        memory: 64,
        disk: 512,
        monthly: 11_710,
        setup: 5900,
        source: "https://www.hetzner.com/dedicated-rootserver/ax42/",
    },
    {
        name: "AX102-1",
        kind: "dedicated",
        vcpus: 32,
        memory: 128,
        disk: 1920,
        monthly: 30_210,
        setup: 14_900,
        source: "https://www.hetzner.com/dedicated-rootserver/ax102/",
    },
];

/** Hetzner, the host of Destack's dedicated machines. */
export const hetzner: MachineVendor = {
    /** Quote the cheapest share of a dedicated server, or cloud server, with the resources. */
    quote(wanted: MachineType): MachineQuote | undefined {
        const quotes = HETZNER_SERVERS.flatMap((server) => {
            // take the share of the server the resources fill, its tightest resource deciding
            const share = Math.max(
                wanted.vcpus / server.vcpus,
                wanted.memory / server.memory,
                wanted.disk / server.disk,
            );
            if (share > 1) {
                return [];
            }

            // sell a cloud server whole, and a dedicated server by share with its setup spread over a year
            const monthly =
                server.kind === "cloud"
                    ? server.monthly
                    : share * (server.monthly + server.setup / SETUP_MONTHS);

            return [{ name: server.name, monthly, source: PRICES, observedAt: OBSERVED_AT }];
        });

        return quotes.toSorted((left, right) => left.monthly - right.monthly)[0];
    },
};
