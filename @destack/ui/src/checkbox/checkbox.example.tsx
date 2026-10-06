import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { Checkbox } from "./checkbox.tsx";

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
