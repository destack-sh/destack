import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Stack } from "./index.ts";

test("stack elements one spacing step apart, step 4 by default, in the element a caller renders", () => {
    const { container } = render(() => (
        <>
            <Stack />
            <Stack space="2" render={(attributes) => <ul {...attributes} />} />
        </>
    ));
    const stacks = [...container.querySelectorAll("[data-slot=stack]")];

    // each stack carries its space for its styles to read
    expect(stacks.map((stack) => [stack.tagName, stack.getAttribute("style")])).toEqual([
        ["DIV", "--x---stack-space: var(--destack-space-4);"],
        ["UL", "--x---stack-space: var(--destack-space-2);"],
    ]);
});

test("mark a recursive stack and the element it splits after", () => {
    const { container } = render(() => <Stack recursive splitAfter={2} />);
    const stack = container.querySelector("[data-slot=stack]");

    expect([
        stack?.getAttribute("data-recursive"),
        stack?.getAttribute("data-split-after"),
    ]).toEqual(["", "2"]);
});
