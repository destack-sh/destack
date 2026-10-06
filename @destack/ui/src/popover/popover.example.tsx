import { defineExample } from "@destack/package/declare";
import { Field, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "./popover.tsx";

/** A popover that edits the size of an image in a note. */
export const popoverImageSize = defineExample({
    of: Popover,
    name: "image-size",
    description: "a popover that edits the size of an image in a note",
    render: () => (
        <Popover>
            <PopoverTrigger variant="outline">Size</PopoverTrigger>
            <PopoverContent align="start">
                <Field orientation="horizontal">
                    <FieldLabel>Width</FieldLabel>
                    <Input type="number" name="width" value="640" />
                </Field>
            </PopoverContent>
        </Popover>
    ),
});

/** The image size popover open beside its trigger. */
export const popoverImageSizeOpen = defineExample({
    of: Popover,
    name: "image-size-open",
    description: "the image size popover open beside its trigger",
    render: () => (
        <Popover defaultOpen>
            <PopoverTrigger variant="outline">Size</PopoverTrigger>
            <PopoverContent align="start">
                <Field orientation="horizontal">
                    <FieldLabel>Width</FieldLabel>
                    <Input type="number" name="width" value="640" />
                </Field>
            </PopoverContent>
        </Popover>
    ),
});
