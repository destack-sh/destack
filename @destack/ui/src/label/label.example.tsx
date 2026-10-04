import type { JSX } from "@solidjs/web";
import { Label } from "./label.tsx";

/** Show a label next to the checkbox it names. */
export function LabelExample(): JSX.Element {
    return (
        <Label>
            <input type="checkbox" name="terms" />
            Accept the terms
        </Label>
    );
}
