import { expect, test } from "@destack/test";
import { BloomFilter } from "./catalog.ts";

test("hold every identifier a filter was built of, and few others", () => {
    // build a filter of ten thousand identifiers and probe ten thousand others
    const held = Array.from({ length: 10_000 }, (_, index) => `event-${String(index)}`);
    const others = Array.from({ length: 10_000 }, (_, index) => `other-${String(index)}`);
    const filter = BloomFilter.build(held);
    const falsePositives = others.filter((id) => BloomFilter.mayHold(filter, id)).length;

    // no false negatives, about one false positive in a hundred at ten bits each
    expect({
        bytes: filter.byteLength,
        isEveryHeld: held.every((id) => BloomFilter.mayHold(filter, id)),
        isNearOnePercent: falsePositives > 50 && falsePositives < 200,
    }).toEqual({ bytes: 12_500, isEveryHeld: true, isNearOnePercent: true });
});
