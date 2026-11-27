import hashIcon from "@destack/icon/phosphor/hash";
import paintBucketIcon from "@destack/icon/phosphor/paint-bucket";
import rulerIcon from "@destack/icon/phosphor/ruler";
import textIcon from "@destack/icon/phosphor/text-aa";
import { JsonValue, toJsonSchema } from "@destack/schema";
import { Dimension, Paint } from "@destack/style";
import { expect, test } from "@destack/test";
import { createSignal, type JSX } from "@destack/view";
import { render } from "@destack/view/test";
import { Icon } from "../icon/index.ts";
import { ValueIcon, type ValueIconProperties } from "./icon.tsx";

/** Draw an icon alone, as its markup. */
function drawn(icon: () => JSX.Element): string {
    return render(icon).container.innerHTML;
}

test("follow a value's type as it changes, from a number's icon to a text's", async () => {
    // draw a number's icon, then change the type to text, each once its module loads
    const [type, setType] = createSignal<ValueIconProperties["type"]>({ kind: "number" });
    const { container } = render(() => <ValueIcon type={type()} />);
    await expect.poll(() => container.innerHTML).toBe(drawn(() => <Icon icon={hashIcon} />));
    setType({ kind: "text" });

    await expect.poll(() => container.innerHTML).toBe(drawn(() => <Icon icon={textIcon} />));
});

test("show a named schema's icon in place of the JSON kind's, such as a paint's", async () => {
    // draw the icons of a paint and of a length, each once its module loads
    const named = [Paint, Dimension].map(
        (definition) =>
            render(() => (
                <ValueIcon
                    type={{ kind: "json", schema: JsonValue.object(toJsonSchema(definition)) }}
                />
            )).container,
    );
    const expected = [paintBucketIcon, rulerIcon].map((icon) => drawn(() => <Icon icon={icon} />));

    await expect.poll(() => named.map((container) => container.innerHTML)).toEqual(expected);
});

test("draw an icon given in place of the type's at once", () => {
    expect(drawn(() => <ValueIcon type={{ kind: "number" }} icon={rulerIcon} />)).toBe(
        drawn(() => <Icon icon={rulerIcon} />),
    );
});
