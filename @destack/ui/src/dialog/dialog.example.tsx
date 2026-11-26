import { t } from "@destack/locale";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import { type JSX, omit } from "@destack/view";
import { Button } from "../button/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "./dialog.tsx";

/** A dialog that renames a note from a form. */
export const dialogDefault = defineExample({
    of: Dialog,
    name: "default",
    description: t`A dialog that renames a note from a form.`,
    render: (properties) => <RenameDialog {...properties} />,
});

/** The rename dialog open over the page. */
export const dialogContentDefault = defineExample({
    of: DialogContent,
    name: "default",
    description: t`The rename dialog open over the page.`,
    render: (properties) => <RenameDialog defaultOpen content={properties} />,
});

/** Render a dialog that renames a note, open from the start or on its trigger. */
function RenameDialog(
    properties: PropertiesOf<typeof Dialog> & {
        /** The properties of the dialog's content. */
        readonly content?: PropertiesOf<typeof DialogContent>;
    },
): JSX.Element {
    return (
        <Dialog {...omit(properties, "content")}>
            <DialogTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
                Rename
            </DialogTrigger>
            <DialogContent {...properties.content}>
                <DialogHeader>
                    <DialogTitle>Rename note</DialogTitle>
                    <DialogDescription>
                        The new title shows everywhere the note is shared.
                    </DialogDescription>
                </DialogHeader>
                <Field>
                    <FieldLabel>Title</FieldLabel>
                    <Input name="title" value="Groceries" />
                </Field>
                <DialogFooter>
                    <DialogClose
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
                        Cancel
                    </DialogClose>
                    <DialogClose render={(attributes) => <Button {...attributes} type="submit" />}>
                        Save
                    </DialogClose>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
