import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Label } from "../label/index.ts";
import { Checkbox } from "./checkbox.tsx";
import { CheckboxGroup } from "./group.tsx";

/** A checkbox that accepts the terms. */
export const checkboxDefault = defineExample({
    of: Checkbox,
    name: "default",
    description: t`The box that accepts the terms.`,
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
export const checkboxChecked = defineExample({
    of: Checkbox,
    name: "checked",
    description: t`The terms checkbox checked.`,
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

/** The terms checkbox marked invalid, as a form shows it before the terms are accepted. */
export const checkboxInvalid = defineExample({
    of: Checkbox,
    name: "invalid",
    description: t`The terms checkbox marked invalid, as a form shows it before the terms are accepted.`,
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
export const checkboxGroupDefault = defineExample({
    of: CheckboxGroup,
    name: "default",
    description: t`Channels a person is told by, with a box for them all and one that stays.`,
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
export const checkboxGroupReadOnly = defineExample({
    of: CheckboxGroup,
    name: "readOnly",
    description: t`Boxes a person reads but does not change.`,
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
