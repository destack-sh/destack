import { expect, test } from "@destack/test";
import { attributes, attrs, create } from "@destack/style";

/** A dynamic style whose property StyleX names in camel case. */
const styles = create({
    row: (row: number) => ({ gridRow: row }),
});

test("set the dynamic variables under the names the compiled classes read", () => {
    expect(attrs(styles.row(3))).toEqual({ class: "x33j1x8", style: "--x-gridRow:3" });
});

test("apply a caller's inline style after the dynamic variables", () => {
    expect(attributes(styles.row(3), { "pointer-events": "none" })).toEqual({
        class: "x33j1x8",
        style: "--x-gridRow:3;pointer-events:none",
    });
});

test("give no attributes for no styles and no inline style", () => {
    expect(attributes([null, false], undefined)).toEqual({});
});

test("keep a caller's inline style written as CSS text", () => {
    expect(attributes(null, "color:red")).toEqual({ style: "color:red" });
});
