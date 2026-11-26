import { t } from "@destack/locale";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import type { JSX } from "@destack/view";
import { Button, IconButton } from "./button.tsx";

/** A save button as its defaults show it. */
export const buttonDefault = defineExample({
    of: Button,
    name: "default",
    description: t`A note's save button.`,
    render: renderSave,
});

/** A note's delete button: its icon beside its words, destructive. */
export const buttonDelete = defineExample({
    of: Button,
    name: "delete",
    description: t`A button that deletes a note.`,
    properties: { variant: "destructive" },
    render: renderDelete,
});

/** A note's toolbar: a primary save, a quiet cancel and an icon-only delete. */
export const buttonNoteToolbar = defineExample({
    of: Button,
    name: "noteToolbar",
    description: t`A note's toolbar: a primary save, a quiet cancel and an icon-only delete.`,
    render: () => (
        <div role="toolbar" aria-label="Note">
            <Button type="submit">Save</Button>
            <Button variant="ghost">Cancel</Button>
            <IconButton variant="destructive" aria-label="Delete note">
                <Icon name="trash" />
            </IconButton>
        </div>
    ),
});

/** Render a note's save button. */
function renderSave(properties: PropertiesOf<typeof Button>): JSX.Element {
    return <Button {...properties}>Save</Button>;
}

/** Render a note's delete button, its icon beside its words. */
function renderDelete(properties: PropertiesOf<typeof Button>): JSX.Element {
    return (
        <Button {...properties}>
            <Icon name="trash" />
            Delete
        </Button>
    );
}

/** A note's delete button showing only its icon. */
export const iconButtonDefault = defineExample({
    of: IconButton,
    name: "default",
    description: t`A note's delete button showing only its icon, named for assistive technology.`,
    render: (properties) => (
        <IconButton aria-label="Delete note" {...properties}>
            <Icon name="trash" />
        </IconButton>
    ),
});

/** A message's send button, round. */
export const iconButtonRound = defineExample({
    of: IconButton,
    name: "round",
    description: t`A message's send button, round.`,
    properties: { shape: "round" },
    render: (properties) => (
        <IconButton aria-label="Send" {...properties}>
            <Icon name="arrow-up" />
        </IconButton>
    ),
});
