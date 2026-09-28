import { expect, test } from "@destack/test";
import type { Row } from "@destack/db";
import { Window } from "./window.ts";
import { Random } from "../feed/test/random.ts";

/** The rows an arrangement holds at most. */
const LIMIT = 5;

/** The keys the random moves pick from, enough for several chunks. */
const KEYS = 1500;

/** Sort rows by score descending, then by key. */
function compare(left: Row, right: Row): number {
    return (
        (right.score as number) - (left.score as number) ||
        String(left.id).localeCompare(String(right.id))
    );
}

test("arrange rows as a sorted list does through random moves", () => {
    // move rows at random in a window and a sorted list alike
    const random = new Random(7);
    const window = new Window(compare, LIMIT, undefined, [], true);
    const reference: Row[] = [];
    const evictions: string[] = [];
    const expected: string[] = [];
    for (let step = 0; step < 3000; step += 1) {
        // take the row out of the list
        const key = `r${random.integer(KEYS)}`;
        const before = reference.slice(0, LIMIT).map((row) => row.id as string);
        const index = reference.findIndex((row) => row.id === key);
        if (index >= 0) {
            reference.splice(index, 1);
        }

        // remove the row from both
        if (random.chance(0.2)) {
            window.remove(key);
            continue;
        }

        // place it in both, noting the row it evicts
        const row = { id: key, score: random.integer(100) };
        const placed = window.place(key, row);
        const position = reference.findIndex((entry) => compare(entry, row) > 0);
        reference.splice(position < 0 ? reference.length : position, 0, row);
        const after = reference.slice(0, LIMIT).map((entry) => entry.id as string);
        expect(placed.isHeld).toBe(after.includes(key));
        if (placed.evicted !== undefined) {
            evictions.push(placed.evicted);
        }
        if (!before.includes(key) && after.includes(key) && before.length === LIMIT) {
            expected.push(before.find((held) => !after.includes(held))!);
        }
    }

    // hold the list's first rows and evict the displaced rows
    expect([window.held(), window.size, evictions]).toEqual([
        reference.slice(0, LIMIT).map((row) => row.id),
        reference.length,
        expected,
    ]);
});
