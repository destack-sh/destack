import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { flush } from "solid-js";
import { Field, FieldLabel } from "../field/index.ts";
import { InputOTP, InputOTPGroup, InputOTPSlot } from "./index.ts";

/** Render a four-digit code input labelled by its field, recording each change and completion. */
function drawCode(changes: string[], completions: string[]): HTMLElement {
    return draw(() => (
        <Field>
            <FieldLabel>Code</FieldLabel>
            <InputOTP
                maxLength={4}
                onValueChange={(value) => changes.push(value)}
                onComplete={(value) => completions.push(value)}
            >
                <InputOTPGroup>
                    <InputOTPSlot index={0} />
                    <InputOTPSlot index={1} />
                    <InputOTPSlot index={2} />
                    <InputOTPSlot index={3} />
                </InputOTPGroup>
            </InputOTP>
        </Field>
    ));
}

/** Type text into an input as a paste or autofill would. */
function enter(input: HTMLInputElement, text: string): void {
    input.value = text;
    input.setSelectionRange(text.length, text.length);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flush();
}

/** Read the character each slot shows. */
function slots(container: Element): string[] {
    return [...container.querySelectorAll("[data-slot=input-otp-slot]")].map(
        (slot) => slot.textContent ?? "",
    );
}

test("offer one-time code autofill on a native input its field labels", () => {
    const container = drawCode([], []);
    const input = container.querySelector("input");
    expect([
        input?.getAttribute("autocomplete"),
        input?.getAttribute("inputmode"),
        input?.getAttribute("maxlength"),
        container.querySelector("label")?.htmlFor === input?.id,
    ]).toEqual(["one-time-code", "numeric", "4", true]);
});

test("show each digit in its slot, dropping other characters, and report the complete code", () => {
    const changes: string[] = [];
    const completions: string[] = [];
    const container = drawCode(changes, completions);
    const input = container.querySelector("input");
    if (input !== null) {
        enter(input, "1a2");
        enter(input, "12 345");
    }
    expect([slots(container), changes, completions]).toEqual([
        ["1", "2", "3", "4"],
        ["12", "1234"],
        ["1234"],
    ]);
});

test("mark the slot holding the caret while the input has the focus", () => {
    const container = drawCode([], []);
    const input = container.querySelector("input");
    input?.focus();
    flush();
    if (input !== null) {
        enter(input, "7");
    }
    const active = [...container.querySelectorAll("[data-slot=input-otp-slot]")].map((slot) =>
        slot.getAttribute("data-active"),
    );
    expect(active).toEqual(["false", "true", "false", "false"]);
});
