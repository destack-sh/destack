import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { omit } from "@destack/view";
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectSeparator,
    SelectTrigger,
    SelectValue,
} from "./select.tsx";

/** The sort order of a note list. */
export const selectDefault = defineExample({
    of: Select,
    name: "default",
    description: t`The sort order of a note list.`,
    properties: {
        name: "sort",
        "aria-label": "Sort by",
    },
    render: (properties) => (
        <Select {...properties}>
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                    <SelectLabel>Date</SelectLabel>
                    <SelectItem value="updated">Last edited</SelectItem>
                    <SelectItem value="created">Created</SelectItem>
                </SelectGroup>
                <SelectSeparator />
                <SelectItem value="title">Title</SelectItem>
            </SelectContent>
        </Select>
    ),
});

/** The sort order of a note list before one is chosen. */
export const selectEmpty = defineExample({
    of: Select,
    name: "empty",
    description: t`The sort order of a note list before one is chosen.`,
    properties: {
        name: "sort",
        "aria-label": "Sort by",
        placeholder: "Sort by…",
    },
    render: (properties) => (
        <Select {...properties}>
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectItem value="updated">Last edited</SelectItem>
                <SelectItem value="title">Title</SelectItem>
            </SelectContent>
        </Select>
    ),
});

/** The tags of a note, several chosen at once. */
export const selectMultiple = defineExample({
    of: Select,
    name: "multiple",
    description: t`The tags of a note, several chosen at once.`,
    render: (properties) => (
        <Select
            {...omit(properties, "multiple", "value", "defaultValue", "onValueChange")}
            name="tags"
            aria-label="Tags"
            multiple
            defaultValue={["travel", "family"]}
        >
            <SelectItem value="travel">Travel</SelectItem>
            <SelectItem value="food">Food</SelectItem>
            <SelectItem value="family">Family</SelectItem>
        </Select>
    ),
});

/** The sort order of a note list marked invalid. */
export const selectInvalid = defineExample({
    of: Select,
    name: "invalid",
    description: t`The sort order of a note list marked invalid.`,
    properties: {
        name: "sort",
        "aria-label": "Sort by",
        placeholder: "Sort by…",
        "aria-invalid": "true",
    },
    render: (properties) => (
        <Select {...properties}>
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectItem value="updated">Last edited</SelectItem>
                <SelectItem value="title">Title</SelectItem>
            </SelectContent>
        </Select>
    ),
});
