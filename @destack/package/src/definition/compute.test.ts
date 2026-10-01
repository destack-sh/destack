import { expect, test } from "@destack/test";
import { PackageError } from "../error/error.ts";
import { ComputeDefinition } from "./compute.ts";

test("merge compute overrides field by field, and refuse contradictory bounds", () => {
    // keep defaults beside each override
    expect(
        ComputeDefinition.merge(
            { requests: { cpu: 1 }, scaling: { maxInstances: 4 }, idleTimeout: 1000 },
            { requests: { memory: 512 }, scaling: { minInstances: 1 } },
        ),
    ).toEqual({
        requests: { cpu: 1, memory: 512 },
        limits: {},
        scaling: { minInstances: 1, maxInstances: 4 },
        idleTimeout: 1000,
    });

    // refuse more minimum instances than maximum, and requests above limits
    expect(() =>
        ComputeDefinition.merge({ scaling: { maxInstances: 1 } }, { scaling: { minInstances: 2 } }),
    ).toThrow(new PackageError("INVALID_DEFINITION", "minimum scaling exceeds maximum scaling"));
    expect(() =>
        ComputeDefinition.merge({ limits: { memory: 256 } }, { requests: { memory: 512 } }),
    ).toThrow(new PackageError("INVALID_DEFINITION", "memory request exceeds its limit"));
});
