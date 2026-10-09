import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Grid } from "./index.ts";

test("fit as many columns as their minimum allows, 16rem apart by step 4 by default", () => {
    const { container } = render(() => (
        <>
            <Grid />
            <Grid min="12rem" space="2" />
        </>
    ));
    const grids = [...container.querySelectorAll("[data-slot=grid]")];

    // each grid carries its columns and gap for its styles to read
    expect(grids.map((grid) => grid.getAttribute("style"))).toEqual([
        "--x-gridTemplateColumns: repeat(auto-fit, minmax(min(16rem, 100%), 1fr)); --x-gap: var(--destack-space-4);",
        "--x-gridTemplateColumns: repeat(auto-fit, minmax(min(12rem, 100%), 1fr)); --x-gap: var(--destack-space-2);",
    ]);
});
