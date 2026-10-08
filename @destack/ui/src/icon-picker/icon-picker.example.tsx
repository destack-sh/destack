import { defineExample } from "@destack/package/declare";
import {
    IconPicker,
    IconPickerContent,
    IconPickerFooter,
    IconPickerSearch,
} from "./icon-picker.tsx";

/** A project's icon picked from the icon set by category, with a search, the active icon's name and a note while nothing matches. */
export const iconPickerProjectIcon = defineExample({
    of: IconPicker,
    name: "project-icon",
    description:
        "a project's icon picked from the icon set by category, with a search, the active icon's name and a note while nothing matches",
    render: () => (
        <IconPicker onPick={() => undefined}>
            <IconPickerSearch />
            <IconPickerContent />
            <IconPickerFooter />
        </IconPicker>
    ),
});

/** A picker whose search finds no icon, showing its note. */
export const iconPickerEmpty = defineExample({
    of: IconPicker,
    name: "empty",
    description: "a picker whose search finds no icon, showing its note",
    render: () => (
        <IconPicker defaultSearch="qqqqq" onPick={() => undefined}>
            <IconPickerSearch />
            <IconPickerContent />
        </IconPicker>
    ),
});
