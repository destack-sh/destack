import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Kbd } from "../kbd/index.ts";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
    InputGroupText,
} from "./input-group.tsx";

/** A note search with a leading icon, its shortcut and a clear button. */
export const inputGroupNoteSearch = defineExample({
    of: InputGroup,
    name: "noteSearch",
    description: t`A note search with a leading icon, its shortcut and a clear button.`,
    render: () => (
        <InputGroup>
            <InputGroupInput type="search" placeholder="Search notes" aria-label="Search notes" />
            <InputGroupAddon>
                <InputGroupText>
                    <Icon name="magnifying-glass" />
                </InputGroupText>
            </InputGroupAddon>
            <InputGroupAddon align="inline-end">
                <Kbd>⌘K</Kbd>
                <InputGroupButton isIconOnly size="sm" aria-label="Clear search">
                    <Icon name="x" />
                </InputGroupButton>
            </InputGroupAddon>
        </InputGroup>
    ),
});

/** The note search marked invalid. */
export const inputGroupNoteSearchInvalid = defineExample({
    of: InputGroup,
    name: "noteSearchInvalid",
    description: t`The note search marked invalid.`,
    render: () => (
        <InputGroup>
            <InputGroupInput
                type="search"
                placeholder="Search notes"
                aria-label="Search notes"
                aria-invalid="true"
            />
            <InputGroupAddon>
                <InputGroupText>
                    <Icon name="magnifying-glass" />
                </InputGroupText>
            </InputGroupAddon>
        </InputGroup>
    ),
});

/** A note's link with its address beside the field. */
export const inputGroupAddonDefault = defineExample({
    of: InputGroupAddon,
    name: "default",
    description: t`A note's link with its address beside the field.`,
    render: (properties) => (
        <InputGroup>
            <InputGroupInput defaultValue="groceries" aria-label="Link" />
            <InputGroupAddon {...properties}>
                <InputGroupText>notes.example/</InputGroupText>
            </InputGroupAddon>
        </InputGroup>
    ),
});

/** A note's title typed into its group, beside its character limit. */
export const inputGroupInputDefault = defineExample({
    of: InputGroupInput,
    name: "default",
    description: t`A note's title typed into its group, beside its character limit.`,
    properties: { placeholder: "Untitled" },
    render: (properties) => (
        <InputGroup>
            <InputGroupInput aria-label="Title" {...properties} />
            <InputGroupAddon align="inline-end">
                <InputGroupText>40</InputGroupText>
            </InputGroupAddon>
        </InputGroup>
    ),
});
