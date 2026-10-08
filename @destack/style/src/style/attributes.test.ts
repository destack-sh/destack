import { expect, test } from "vitest";

import { attributes, attrs, create } from "@destack/style";

/** A dynamic style whose property StyleX names in camel case. */
const styles = create({
    row: (row: number) => ({ gridRow: row }),
});

test("set the dynamic variables under the names the compiled classes read", () => {
    expect(attrs(styles.row(3)).style).toBe("--x-gridRow:3");
});

test("apply a caller's inline style after the dynamic variables", () => {
    expect(attributes(styles.row(3), { "pointer-events": "none" }).style).toBe(
        "--x-gridRow:3;pointer-events:none",
    );
});
