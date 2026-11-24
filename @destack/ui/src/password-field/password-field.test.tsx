import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { render } from "@destack/view/test";
import { PasswordField } from "./index.ts";

test("show and hide the password", () => {
    const { container } = render(() => <PasswordField defaultValue="horse" />);
    const read = () => [
        container.querySelector("input")?.type,
        container.querySelector("button")?.getAttribute("aria-pressed"),
    ];
    const before = read();
    container.querySelector("button")?.click();
    flush();

    expect([before, read()]).toEqual([
        ["password", "false"],
        ["text", "true"],
    ]);
});
