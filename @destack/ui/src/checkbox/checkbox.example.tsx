import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { Checkbox, CheckboxIndicator } from "./checkbox.tsx";

/** A checkbox that accepts the terms. */
export const checkboxAcceptTerms = defineExample({
    of: Checkbox,
    name: "accept-terms",
    description: "a checkbox that accepts the terms",
    render: () => (
        <Label>
            <Checkbox name="terms" required /> Accept the terms
        </Label>
    ),
});

/** The terms checkbox checked. */
export const checkboxAcceptTermsChecked = defineExample({
    of: Checkbox,
    name: "accept-terms-checked",
    description: "the terms checkbox checked",
    render: () => (
        <Label>
            <Checkbox name="terms" required checked /> Accept the terms
        </Label>
    ),
});

/** The terms checkbox unavailable. */
export const checkboxAcceptTermsDisabled = defineExample({
    of: Checkbox,
    name: "accept-terms-disabled",
    description: "the terms checkbox unavailable",
    render: () => (
        <Label>
            <Checkbox name="terms" required disabled /> Accept the terms
        </Label>
    ),
});

/** A select-all box composed from its root and indicator, showing a partial selection. */
export const checkboxSelectAll = defineExample({
    of: Checkbox,
    name: "select-all",
    description:
        "a select-all box composed from its root and indicator, showing a partial selection",
    render: () => (
        <Label>
            <Checkbox indeterminate>
                <CheckboxIndicator />
            </Checkbox>{" "}
            Select all
        </Label>
    ),
});

/** The terms checkbox marked invalid, as a form shows it before the terms are accepted. */
export const checkboxAcceptTermsInvalid = defineExample({
    of: Checkbox,
    name: "accept-terms-invalid",
    description:
        "the terms checkbox marked invalid, as a form shows it before the terms are accepted",
    render: () => (
        <Label>
            <Checkbox name="terms" required aria-invalid="true" /> Accept the terms
        </Label>
    ),
});
