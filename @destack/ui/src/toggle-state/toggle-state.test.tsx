import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { draw, markup } from "@destack/view/test";
import { isSubmitted, ToggleInput, ToggleState } from "./toggle-state.tsx";

test("keep an uncontrolled toggle's own state, report each change and reset to the default", () => {
    // start checked and uncheck, check and reset
    const reported: boolean[] = [];
    const toggle = new ToggleState({
        defaultChecked: true,
        onCheckedChange: (checked) => reported.push(checked),
    });
    const states = [toggle.isChecked()];
    toggle.set(false);
    flush();
    states.push(toggle.isChecked());
    toggle.reset();
    flush();
    states.push(toggle.isChecked());

    // the toggle follows its own changes and tells the owner of each
    expect({ states, reported }).toEqual({ states: [true, false, true], reported: [false, true] });
});

test("hold a controlled toggle at its owner's state and report the change it asks for", () => {
    // ask to check while the owner keeps the toggle unchecked, then let the owner check it
    const [isOwned, setOwned] = createSignal(false);
    const reported: boolean[] = [];
    const toggle = new ToggleState({
        get checked() {
            return isOwned();
        },
        onCheckedChange: (checked) => reported.push(checked),
    });
    toggle.set(true);
    flush();
    const held = toggle.isChecked();
    setOwned(true);
    flush();

    // the toggle reads the owner's state at each step
    expect({ held, followed: toggle.isChecked(), reported }).toEqual({
        held: false,
        followed: true,
        reported: [true],
    });
});

test("submit through a hidden input only with a name, a requirement or a form", () => {
    expect([
        isSubmitted({}),
        isSubmitted({ value: "yes" }),
        isSubmitted({ name: "terms" }),
        isSubmitted({ required: true }),
        isSubmitted({ form: "settings" }),
    ]).toEqual([false, false, true, true, true]);
});

test("reflect the toggle's state in the hidden input and restore it when its own form resets", () => {
    // render a checked input in one form and a radio in another
    const [isChecked, setChecked] = createSignal(true);
    const resets: string[] = [];
    const container = draw(() => (
        <>
            <form id="settings">
                <ToggleInput
                    name="terms"
                    checked={isChecked()}
                    disabled={false}
                    onReset={() => resets.push("terms")}
                />
            </form>
            <form id="other">
                <ToggleInput
                    type="radio"
                    name="plan"
                    value="pro"
                    required
                    checked={false}
                    disabled
                    onReset={() => resets.push("plan")}
                />
            </form>
        </>
    ));
    const [terms, plan] = container.querySelectorAll("input");
    const before = [terms?.checked, plan?.checked];
    setChecked(false);
    flush();
    const after = [terms?.checked, plan?.checked];
    container.querySelector<HTMLFormElement>("#settings")?.reset();

    // the inputs carry their state, and a reset reaches only its own form's input
    expect({ markup: markup(container), before, after, resets }).toEqual({
        markup:
            '<form id="settings"><input type="checkbox" aria-hidden="true" tabindex="-1" name="terms" value="on"></form>' +
            '<form id="other"><input type="radio" aria-hidden="true" tabindex="-1" name="plan" value="pro" required="" disabled=""></form>',
        before: [true, false],
        after: [false, false],
        resets: ["terms"],
    });
});

test("leave the hidden input agreeing with its control after its form resets", async () => {
    // keep one control at its checked default and hold another checked by its owner
    const own = new ToggleState({ defaultChecked: true });
    const held = new ToggleState({ checked: true, defaultChecked: false });
    const container = draw(() => (
        <form>
            <ToggleInput
                name="terms"
                checked={own.isChecked()}
                disabled={false}
                onReset={() => own.reset()}
            />
            <ToggleInput
                name="news"
                checked={held.isChecked()}
                disabled={false}
                onReset={() => held.reset()}
            />
        </form>
    ));
    container.querySelector("form")?.reset();
    flush();
    await new Promise((resolve) => {
        setTimeout(resolve);
    });

    // the browser restores each input to its markup, and each then follows its control again
    expect({
        controls: [own.isChecked(), held.isChecked()],
        inputs: [...container.querySelectorAll("input")].map((input) => input.checked),
    }).toEqual({ controls: [true, true], inputs: [true, true] });
});
