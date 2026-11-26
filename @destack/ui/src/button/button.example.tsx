import { t } from "@destack/locale";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import type { JSX } from "@destack/view";
import { Button } from "./button.tsx";

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
export const buttonIconOnly = defineExample({
    of: Button,
    name: "iconOnly",
    description: t`A note's delete button showing only its icon, named for assistive technology.`,
    properties: { isIconOnly: true },
    render: (properties) => (
        <Button aria-label="Delete note" {...properties}>
            <Icon name="trash" />
        </Button>
    ),
});
