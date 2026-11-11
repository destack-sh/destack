import { expect, refusal, test } from "@destack/test";
import { ResourceRecord } from "./provider.ts";

test("read a provisioned resource's reference, refusing an unprovisioned one", async () => {
    expect([
        ResourceRecord.reference({ reference: "bucket-1" }),
        await refusal(() => ResourceRecord.reference({ reference: null })),
    ]).toEqual([
        "bucket-1",
        [
            "SERVICE_UNAVAILABLE",
            "RESOURCE_UNAVAILABLE",
            "resource is unavailable: it is not provisioned",
        ],
    ]);
});
