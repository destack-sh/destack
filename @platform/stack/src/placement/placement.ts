import { accountDatabase } from "@destack/account/stack";
import { accountWorkload } from "@destack/account/workload";
import type { Database } from "@destack/db";
import { forgeDatabase } from "@destack/forge/stack";
import { forgeWorkload } from "@destack/forge/workload";
import { relayDatabase } from "@destack/relay/stack";
import { relayWorkload } from "@destack/relay/workload";
import type { Workload } from "@destack/service/workload";
import { type PlatformResidency, RESIDENCIES } from "../universe/residency.ts";

/** A platform package a universe places: its workload and its database. */
export interface Platform {
    /** The package's workload, declaring where a universe places it. */
    readonly workload: Workload;
    /** The database the workload serves over, in a schema of its own. */
    readonly database: Database;
}

/** A process running placed workloads. */
export interface Process {
    /** The process's name, such as `account` or `forge-eu`. */
    readonly name: string;
    /** The residency a process of the residency tier serves, absent for a process serving every residency. */
    readonly residency?: PlatformResidency;
    /** The platform packages whose workloads the process runs. */
    readonly packages: readonly Platform[];
}

/** The platform packages every universe places. */
export const PLATFORM: readonly Platform[] = [
    { workload: accountWorkload, database: accountDatabase },
    { workload: forgeWorkload, database: forgeDatabase },
    { workload: relayWorkload, database: relayDatabase },
];

/** The processes a universe runs its platform workloads in. */
export class Placement {
    /** The processes, in placement order. */
    readonly processes: readonly Process[];

    /** Keep the processes of a placement. */
    private constructor(processes: readonly Process[]) {
        this.processes = processes;
    }

    /** Place every workload in one process, as the development universe runs. */
    static single(name: string, platform: readonly Platform[]): Placement {
        return new Placement([{ name, packages: platform }]);
    }

    /** Place each workload in processes of its own: one in the universe tier, one per residency in the residency tier. */
    static spread(platform: readonly Platform[]): Placement {
        return new Placement(
            platform.flatMap((entry): Process[] => {
                // place a workload once in the universe, or once in each residency
                const { workload } = entry;
                const placement = workload.placement ?? [];
                if (placement.includes("universe")) {
                    return [{ name: workload.name, packages: [entry] }];
                } else if (placement.includes("residency")) {
                    return RESIDENCIES.map((residency) => ({
                        name: `${workload.name}-${residency.code}`,
                        residency,
                        packages: [entry],
                    }));
                }

                // refuse a workload the cells running it place in the space or host tier
                throw new TypeError(`${workload.name} is placed by the cells running it`);
            }),
        );
    }
}
