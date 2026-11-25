import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { Checkbox, CheckboxIndicator } from "./checkbox.tsx";
import { CheckboxGroup } from "./group.tsx";

/** A checkbox that accepts the terms. */
export const checkboxAcceptTerms = defineExample({
    of: Checkbox,
    name: "accept-terms",
    description: "a checkbox that accepts the terms",
    properties: {
        name: "terms",
        required: true,
    },
    render: (properties) => (
        <Label>
            <Checkbox {...properties} /> Accept the terms
        </Label>
    ),
});

/** The terms checkbox checked. */
export const checkboxAcceptTermsChecked = defineExample({
    of: Checkbox,
    name: "accept-terms-checked",
    description: "the terms checkbox checked",
    properties: {
        name: "terms",
        required: true,
        checked: true,
    },
    render: (properties) => (
        <Label>
            <Checkbox {...properties} /> Accept the terms
        </Label>
    ),
});

/** The terms checkbox unavailable. */
export const checkboxAcceptTermsDisabled = defineExample({
    of: Checkbox,
    name: "accept-terms-disabled",
    description: "the terms checkbox unavailable",
    properties: {
        name: "terms",
        required: true,
        disabled: true,
    },
    render: (properties) => (
        <Label>
            <Checkbox {...properties} /> Accept the terms
        </Label>
    ),
});

/** A select-all box composed from its root and indicator, showing a partial selection. */
export const checkboxSelectAll = defineExample({
    of: Checkbox,
    name: "select-all",
    description:
        "a select-all box composed from its root and indicator, showing a partial selection",
    properties: {
        indeterminate: true,
    },
    render: (properties) => (
        <Label>
            <Checkbox {...properties}>
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
    properties: {
        name: "terms",
        required: true,
        "aria-invalid": "true",
    },
    render: (properties) => (
        <Label>
            <Checkbox {...properties} /> Accept the terms
        </Label>
    ),
});

/** Channels a person is told by, with a box for them all and one that stays. */
export const checkboxGroup = defineExample({
    of: CheckboxGroup,
    name: "group",
    description: "channels a person is told by, with a box for them all and one that stays",
    render: () => (
        <CheckboxGroup aria-label="Tell me by" name="channel" defaultValue={["mail", "app"]}>
            <Label>
                <Checkbox parent /> Everything
            </Label>
            <Label>
                <Checkbox value="app" disabled /> In the app
            </Label>
            <Label>
                <Checkbox value="mail" /> Mail
            </Label>
            <Label>
                <Checkbox value="push" /> Push
            </Label>
        </CheckboxGroup>
    ),
});

/** Boxes a person reads but does not change. */
export const checkboxReadOnly = defineExample({
    of: CheckboxGroup,
    name: "read-only",
    description: "boxes a person reads but does not change",
    render: () => (
        <CheckboxGroup aria-label="Tell me by" defaultValue={["mail"]} readOnly>
            <Label>
                <Checkbox value="app" /> In the app
            </Label>
            <Label>
                <Checkbox value="mail" /> Mail
            </Label>
        </CheckboxGroup>
    ),
});
