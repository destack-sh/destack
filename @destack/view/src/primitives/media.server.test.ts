import { expect, test } from "@destack/test";
import { createBreakpoints, createMediaQuery } from "./media.ts";

test("answer media queries and breakpoints with their fallbacks on the server", () => {
    // name the last fallback breakpoint that matches as the key, here the narrower one
    const matches = createBreakpoints(
        { sm: "640px", lg: "1024px" },
        { fallbackState: { sm: true, lg: false } },
    );

    expect([
        createMediaQuery("(max-width: 767px)")(),
        createMediaQuery("(max-width: 767px)", true)(),
        { ...matches, key: matches.key },
    ]).toEqual([false, true, { sm: true, lg: false, key: "sm" }]);
});
