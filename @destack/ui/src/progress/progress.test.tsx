import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Progress } from "./index.ts";

test("render a native progress bar with its value and maximum", () => {
    const container = draw(() => <Progress value={30} max={100} aria-label="Upload" />);
    expect(markup(container)).toBe(
        '<progress data-slot="progress" value="30" max="100" aria-label="Upload"></progress>',
    );
});
