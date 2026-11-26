import linkIcon from "@destack/icon/phosphor/link";
import textBIcon from "@destack/icon/phosphor/text-b";
import textItalicIcon from "@destack/icon/phosphor/text-italic";
import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { Button } from "../button/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { Stack } from "../stack/index.ts";
import { Input } from "../input/index.ts";
import type { AnchorBox } from "../position/index.ts";
import { Toolbar, ToolbarButton } from "../toolbar/index.ts";
import { Icon } from "../icon/index.ts";
import {
    Popover,
    PopoverClose,
    PopoverContent,
    PopoverControl,
    PopoverDescription,
    PopoverHeader,
    PopoverTitle,
    PopoverTrigger,
    PopoverViewport,
} from "./popover.tsx";

/** The people of a list. */
const PEOPLE = [
    { name: "Ada", role: "Writes the plans" },
    { name: "Ben", role: "Books the trains and finds the rooms" },
    { name: "Cleo", role: "Takes the photos" },
];

/** A popover that edits the size of an image in a note. */
export const popoverImageSize = defineExample({
    of: Popover,
    name: "image-size",
    description: "a popover that edits the size of an image in a note",
    render: () => (
        <Popover>
            <PopoverTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Size
            </PopoverTrigger>
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
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <Popover {...properties}>
            <PopoverTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Size
            </PopoverTrigger>
            <PopoverContent align="start">
                <Field orientation="horizontal">
                    <FieldLabel>Width</FieldLabel>
                    <Input type="number" name="width" value="640" />
                </Field>
            </PopoverContent>
        </Popover>
    ),
});

/** A modal popover that renames a note, the page behind it inert while it is open. */
export const popoverRenameModal = defineExample({
    of: Popover,
    name: "rename-modal",
    description: "a modal popover that renames a note, the page behind it inert while it is open",
    properties: {
        modal: true,
        defaultOpen: true,
    },
    render: (properties) => (
        <Popover {...properties}>
            <PopoverTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Rename
            </PopoverTrigger>
            <PopoverContent align="start">
                <Stack>
                    <Field>
                        <FieldLabel>Title</FieldLabel>
                        <Input name="title" value="Groceries" />
                    </Field>
                    <PopoverClose render={(attributes) => <Button {...attributes} />}>
                        Save
                    </PopoverClose>
                </Stack>
            </PopoverContent>
        </Popover>
    ),
});

/** A popover titled and described for assistive technology, with a button that closes it. */
export const popoverLinkSharing = defineExample({
    of: Popover,
    name: "link-sharing",
    description:
        "a popover titled and described for assistive technology, with a button that closes it",
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <Popover {...properties}>
            <PopoverTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Share
            </PopoverTrigger>
            <PopoverContent align="end">
                <PopoverHeader>
                    <PopoverTitle>Link sharing</PopoverTitle>
                    <PopoverDescription>
                        Anyone with the link can view this note.
                    </PopoverDescription>
                </PopoverHeader>
                <PopoverClose render={(attributes) => <Button {...attributes} variant="outline" />}>
                    Done
                </PopoverClose>
            </PopoverContent>
        </Popover>
    ),
});

/** The people of a list, who share one popover that glides from one to the next. */
export const popoverPeople = defineExample({
    of: Popover,
    name: "people",
    description: "the people of a list, who share one popover that glides from one to the next",
    render: () => {
        const card = new PopoverControl<(typeof PEOPLE)[number]>();

        return (
            <>
                <Stack space="2">
                    <For each={PEOPLE}>
                        {(person) => (
                            <PopoverTrigger
                                control={card}
                                payload={person}
                                render={(attributes) => (
                                    <Button {...attributes} variant="ghost">
                                        {person.name}
                                    </Button>
                                )}
                            />
                        )}
                    </For>
                </Stack>
                <Popover control={card}>
                    {(person) => (
                        <PopoverContent side="right" align="start">
                            <PopoverViewport>
                                <PopoverHeader>
                                    <PopoverTitle>{person()?.name}</PopoverTitle>
                                    <PopoverDescription>{person()?.role}</PopoverDescription>
                                </PopoverHeader>
                            </PopoverViewport>
                        </PopoverContent>
                    )}
                </Popover>
            </>
        );
    },
});

/** The toolbar of a text's selection, beside the words a person selects. */
export const popoverSelection = defineExample({
    of: Popover,
    name: "selection",
    description: "the toolbar of a text's selection, beside the words a person selects",
    layout: "padded",
    render: () => {
        // follow the words selected, once the press that selects them ends
        const [box, setBox] = createSignal<AnchorBox>();
        const select = (event: { readonly currentTarget: HTMLElement }) => {
            const selection = event.currentTarget.ownerDocument.getSelection();
            const isEmpty = selection === null || selection.isCollapsed;
            setBox(isEmpty ? undefined : selection.getRangeAt(0).getBoundingClientRect());
        };

        return (
            <>
                <p onClick={select} onKeyUp={select}>
                    Take the early tram to the lake, then walk up to the pass before noon.
                </p>
                <Popover
                    open={box() !== undefined}
                    onOpenChange={(isOpen) => {
                        if (!isOpen) {
                            setBox(undefined);
                        }
                    }}
                >
                    <PopoverContent anchor={box()} side="top" width="fit">
                        <Toolbar aria-label="Format">
                            <ToolbarButton size="icon-sm" variant="ghost" aria-label="Bold">
                                <Icon icon={textBIcon} />
                            </ToolbarButton>
                            <ToolbarButton size="icon-sm" variant="ghost" aria-label="Italic">
                                <Icon icon={textItalicIcon} />
                            </ToolbarButton>
                            <ToolbarButton size="icon-sm" variant="ghost" aria-label="Link">
                                <Icon icon={linkIcon} />
                            </ToolbarButton>
                        </Toolbar>
                    </PopoverContent>
                </Popover>
            </>
        );
    },
});
