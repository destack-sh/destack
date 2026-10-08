import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { Switch, SwitchThumb } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List the checked state each switch of a container reports, in order. */
function checked(container: Element): string[] {
    return [...container.querySelectorAll("[role=switch]")].map(
        (control) => control.getAttribute("aria-checked") ?? "",
    );
}

test("flip a switch and its thumb on click, submitting its state under its name", () => {
    const changes: boolean[] = [];
    const container = draw(() => (
        <form>
            <Switch name="notifications" onCheckedChange={(on) => changes.push(on)}>
                <SwitchThumb />
            </Switch>
        </form>
    ));
    container.querySelector<HTMLElement>("[role=switch]")?.click();
    flush();
    const submitted = container.querySelector("input");

    expect(markup(container).replace(/<input[^>]*>/u, "")).toBe(
        '<form><button type="button" role="switch" aria-checked="true" data-slot="switch" data-state="checked"><span data-slot="switch-thumb" data-state="checked"></span></button></form>',
    );
    expect([changes, submitted?.name, submitted?.checked]).toEqual([[true], "notifications", true]);
});

test("hold a controlled switch at its owner's state until the owner changes it", () => {
    const [isOn, setOn] = createSignal(true);
    const container = draw(() => (
        <>
            <Switch aria-label="Fixed" checked />
            <Switch aria-label="Followed" checked={isOn()} onCheckedChange={setOn} />
        </>
    ));
    for (const control of container.querySelectorAll<HTMLElement>("[role=switch]")) {
        control.click();
    }
    flush();
    expect([checked(container), isOn()]).toEqual([["true", "false"], false]);
});

test("return a switch to its first state when its form resets, and ignore clicks while disabled", () => {
    const container = draw(() => (
        <form>
            <Switch name="pinned" defaultChecked />
            <Switch name="locked" disabled />
        </form>
    ));
    for (const control of container.querySelectorAll<HTMLElement>("[role=switch]")) {
        control.click();
    }
    flush();
    const flipped = checked(container);
    container.querySelector("form")?.reset();
    flush();

    expect([flipped, checked(container)]).toEqual([
        ["false", "false"],
        ["true", "false"],
    ]);
});
