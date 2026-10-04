import type { JSX } from "@solidjs/web";
import { Field, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "./popover.tsx";

/** Show a popover that edits the size of an image in a note. */
export function PopoverExample(): JSX.Element {
    return (
        <Popover>
            <PopoverTrigger variant="outline">Size</PopoverTrigger>
            <PopoverContent align="start">
                <Field orientation="horizontal">
                    <FieldLabel>Width</FieldLabel>
                    <Input type="number" name="width" value="640" />
                </Field>
            </PopoverContent>
        </Popover>
    );
}
