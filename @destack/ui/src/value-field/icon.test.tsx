import hashIcon from "@destack/icon/phosphor/hash";
import textIcon from "@destack/icon/phosphor/text-aa";
import { expect, test } from "@destack/test";
import { createSignal, flush, type JSX } from "@destack/view";
import { render } from "@destack/view/test";
import { Icon } from "../icon/index.ts";
import { ValueIcon, type ValueIconProperties } from "./icon.tsx";

/** Draw an icon alone, as its markup. */
function drawn(icon: () => JSX.Element): string {
    return render(icon).container.innerHTML;
}

test("follow a value's type as it changes, from a number's icon to a text's", () => {
    // draw a number's icon, then change the type to text
    const [type, setType] = createSignal<ValueIconProperties["type"]>({ kind: "number" });
    const { container } = render(() => <ValueIcon type={type()} />);
    const before = container.innerHTML;
    setType({ kind: "text" });
    flush();

    expect([before, container.innerHTML]).toEqual([
        drawn(() => <Icon icon={hashIcon} />),
        drawn(() => <Icon icon={textIcon} />),
    ]);
});
