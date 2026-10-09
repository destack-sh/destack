import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Placement } from "../src/index.ts";

/** The machines a residency's spaces spread over. */
const MACHINES = ["machine-a", "machine-b", "machine-c", "machine-d"];

/** The spaces placed in the scenario. */
const SPACES = Array.from({ length: 200 }, (_, index) => `space-${index}`);

test("pick each space's machine regardless of the machines' order, and move only the spaces of a machine that leaves", async () => {
    // pick every space's machine among the machines, and again among them reversed and without one
    const picked = await Promise.all(
        SPACES.map(async (space) => present(await Placement.pick(space, MACHINES), space)),
    );
    const reversed = await Promise.all(
        SPACES.map((space) => Placement.pick(space, MACHINES.toReversed())),
    );
    const remaining = MACHINES.filter((machine) => machine !== "machine-b");
    const after = await Promise.all(SPACES.map((space) => Placement.pick(space, remaining)));

    // keep every choice under another order, move exactly the spaces machine-b held, and use every machine
    const moved = SPACES.filter((_, index) => picked[index] !== after[index]);
    const held = SPACES.filter((_, index) => picked[index] === "machine-b");
    expect({
        isOrderless: reversed.every((machine, index) => machine === picked[index]),
        isMovingOnlyHeld:
            moved.length === held.length && moved.every((space) => held.includes(space)),
        used: [...new Set(picked)].toSorted((left, right) => left.localeCompare(right)),
        none: await Placement.pick("space-0", []),
    }).toEqual({ isOrderless: true, isMovingOnlyHeld: true, used: MACHINES, none: undefined });
});
