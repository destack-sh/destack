import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { Field, FieldLabel } from "../field/index.ts";
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from "./index.ts";

test("focus the group's input on a click on an addon, but not on a click on its button", () => {
    const pressed: string[] = [];
    const container = draw(() => (
        <InputGroup>
            <InputGroupInput aria-label="Search notes" />
            <InputGroupAddon>
                <span>Find</span>
            </InputGroupAddon>
            <InputGroupAddon align="inline-end">
                <InputGroupButton onClick={() => pressed.push("clear")}>Clear</InputGroupButton>
            </InputGroupAddon>
        </InputGroup>
    ));
    container.querySelector("span")?.click();
    const afterText = document.activeElement?.getAttribute("data-slot");
    if (document.activeElement instanceof HTMLElement) {
        document.activeElement.blur();
    }
    container.querySelector("button")?.click();
    expect([afterText, document.activeElement?.tagName, pressed]).toEqual([
        "input-group-control",
        "BODY",
        ["clear"],
    ]);
});

test("label a group's input by its field", () => {
    const container = draw(() => (
        <Field>
            <FieldLabel>Search</FieldLabel>
            <InputGroup>
                <InputGroupInput />
            </InputGroup>
        </Field>
    ));
    const label = container.querySelector("label");
    const input = container.querySelector("input");
    expect([label?.htmlFor, input?.getAttribute("data-slot")]).toEqual([
        input?.id,
        "input-group-control",
    ]);
});

test("give a group's button the ghost variant and extra small size and keep it from submitting", () => {
    const container = draw(() => <InputGroupButton>Clear</InputGroupButton>);
    const button = container.querySelector("button");
    expect([button?.type, button?.dataset["variant"], button?.dataset["size"]]).toEqual([
        "button",
        "ghost",
        "xs",
    ]);
});
