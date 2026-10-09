import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Switcher } from "./index.ts";

test("hand a switcher's threshold to its elements' sizes, 30rem apart by step 4 by default", () => {
    const { container } = render(() => (
        <>
            <Switcher />
            <Switcher threshold="40rem" space="2" />
        </>
    ));
    const switchers = [...container.querySelectorAll("[data-slot=switcher]")];

    // each switcher carries its threshold for its elements' sizes to read
    expect(switchers.map((switcher) => switcher.getAttribute("style"))).toEqual([
        "--x---switcher-threshold: 30rem; --x-gap: var(--destack-space-4);",
        "--x---switcher-threshold: 40rem; --x-gap: var(--destack-space-2);",
    ]);
});

test("keep at most four elements in a switcher's row unless its limit says otherwise", () => {
    const { container } = render(() => (
        <>
            <Switcher />
            <Switcher limit={2} />
        </>
    ));
    const limits = [...container.querySelectorAll("[data-slot=switcher]")].map((switcher) =>
        switcher.getAttribute("data-limit"),
    );

    expect(limits).toEqual(["4", "2"]);
});
