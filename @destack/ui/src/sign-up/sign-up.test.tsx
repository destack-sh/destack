import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { createSignal, flush } from "@destack/view";
import { type HandleAvailability, SignUpHandle, SignUpResidency } from "./index.ts";

test("check a typed handle, showing its profile address and why it is not free, and fold the residency into a line", () => {
    // render a sign-up whose handle is taken and whose residency the edge suggested
    const [availability, setAvailability] = createSignal<HandleAvailability>("taken");
    const [residency, setResidency] = createSignal("eu");
    const typed: string[] = [];
    const { container } = render(() => (
        <div>
            <SignUpHandle
                value="ada"
                availability={availability()}
                onInput={(handle) => typed.push(handle)}
            />
            <SignUpResidency
                residencies={[
                    { code: "eu", name: "European Union" },
                    { code: "us", name: "United States" },
                ]}
                value={residency()}
                onChange={setResidency}
            />
        </div>
    ));
    const taken = container.querySelector("[data-slot=field-error]")?.textContent;
    const line = [...container.querySelectorAll("p")].at(-1)?.textContent;

    // type a handle, free it, and change the residency through its choices
    const input = container.querySelector("input");
    if (input !== null) {
        input.value = " Bea ";
        input.dispatchEvent(new Event("input", { bubbles: true }));
    }
    setAvailability("available");
    [...container.querySelectorAll("button")].at(-1)?.click();
    flush();
    const choices = [...container.querySelectorAll("fieldset button")].map(
        (button) => button.textContent,
    );
    [...container.querySelectorAll("fieldset button")]
        .at(-1)
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    flush();

    expect({
        address: container.querySelector("[data-slot=field-description]")?.textContent,
        taken,
        free: container.querySelector("[data-slot=field-error]")?.textContent ?? null,
        line,
        choices,
        typed,
        chosen: [residency(), [...container.querySelectorAll("p")].at(-1)?.textContent],
    }).toEqual({
        address: "destack.app/@\u2068ada\u2069",
        taken: "This handle is taken.",
        free: null,
        line: "Your data lives in the \u2068European Union\u2069 · change",
        choices: ["European Union", "United States"],
        typed: ["bea"],
        chosen: ["us", "Your data lives in the \u2068United States\u2069 · change"],
    });
});
