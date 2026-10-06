import { defineExample } from "@destack/package/declare";
import { Label } from "./label.tsx";

/** A label next to the checkbox it names. */
export const labelTermsCheckbox = defineExample({
    of: Label,
    name: "terms-checkbox",
    description: "a label next to the checkbox it names",
    render: () => (
        <Label>
            <input type="checkbox" name="terms" />
            Accept the terms
        </Label>
    ),
});
