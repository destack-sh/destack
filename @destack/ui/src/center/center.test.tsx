import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Center } from "./index.ts";

test("center a column up to its maximum with gutters, and its elements too when intrinsic", () => {
    const { container } = render(() => (
        <>
            <Center />
            <Center max="24rem" gutters="2" intrinsic />
        </>
    ));
    const [plain, intrinsic] = [...container.querySelectorAll("[data-slot=center]")];

    // each column carries its width and gutters for its styles to read
    expect([plain, intrinsic].map((center) => center?.getAttribute("style"))).toEqual([
        "--x-maxInlineSize: 65ch; --x-paddingInline: var(--destack-space-4);",
        "--x-maxInlineSize: 24rem; --x-paddingInline: var(--destack-space-2);",
    ]);

    // only the intrinsic column adds the classes centering its elements
    expect(intrinsic?.classList.length).toBeGreaterThan(plain?.classList.length ?? 0);
});

test("center a column's text when asked", () => {
    const { container } = render(() => (
        <>
            <Center />
            <Center andText />
        </>
    ));
    const [plain, text] = [...container.querySelectorAll("[data-slot=center]")];

    // only the second column takes the extra text alignment class
    expect(text?.classList.length).toBe((plain?.classList.length ?? 0) + 1);
});
