import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Cluster } from "./index.ts";

test("wrap elements in rows with a gap, justification and alignment, centered at the start by default", () => {
    const { container } = render(() => (
        <>
            <Cluster />
            <Cluster space="1" justify="between" align="baseline" />
        </>
    ));
    const clusters = [...container.querySelectorAll("[data-slot=cluster]")];

    // each cluster carries its layout for its styles to read
    expect(clusters.map((cluster) => cluster.getAttribute("style"))).toEqual([
        "--x-gap: var(--destack-space-3); --x-justifyContent: flex-start; --x-alignItems: center;",
        "--x-gap: var(--destack-space-1); --x-justifyContent: space-between; --x-alignItems: baseline;",
    ]);
});
