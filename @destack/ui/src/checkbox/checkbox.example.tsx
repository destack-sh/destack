import type { JSX } from "@solidjs/web";
import { Label } from "../label/index.ts";
import { Checkbox } from "./checkbox.tsx";

/** Show a checkbox that accepts the terms. */
export function CheckboxExample(): JSX.Element {
    return (
        <Label>
            <Checkbox name="terms" required /> Accept the terms
        </Label>
    );
}
