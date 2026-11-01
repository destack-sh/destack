import type { Simulation } from "@destack/environment/simulation";
import type { LocalBucket } from "../bucket.ts";

/** How long a write leases its blob: BlobStore's half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** How long an unreferenced blob stays before deletion: BlobStore's hour of grace. */
const GRACE_MILLISECONDS = 60 * 60_000;

/** Collect a bucket's unreferenced blobs a grace after their writes' leases end. */
export async function collectLater(
    simulation: Pick<Simulation, "clock" | "jump">,
    bucket: Pick<LocalBucket, "collect">,
    leasedUntil: number = simulation.clock.now() + LEASE_MILLISECONDS,
): Promise<void> {
    // find them unreferenced once the leases end, then delete them once their grace ends
    await simulation.jump(leasedUntil + 1);
    await bucket.collect();
    await simulation.jump(simulation.clock.now() + GRACE_MILLISECONDS);
    await bucket.collect();
}
