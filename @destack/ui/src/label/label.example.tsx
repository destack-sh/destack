import { defineExample } from "@destack/package/declare";
import { Checkbox } from "../checkbox/index.ts";
import { Input } from "../input/index.ts";
import { Stack } from "../stack/index.ts";
import { Label } from "./label.tsx";

/** A label next to the checkbox it names. */
export const labelTermsCheckbox = defineExample({
    of: Label,
    name: "terms-checkbox",
    description: "a label next to the checkbox it names",
    render: () => (
        <Label>
            <Checkbox name="terms" />
            Accept the terms
        </Label>
    ),
});

/** A label above the field it names. */
export const labelNameField = defineExample({
    of: Label,
    name: "name-field",
    description: "a label above the field it names",
    render: () => (
        <Stack space="2">
            <Label for="notebook-name">Notebook name</Label>
            <Input id="notebook-name" name="name" placeholder="Trips" />
        </Stack>
    ),
});
