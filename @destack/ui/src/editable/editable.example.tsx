import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { weight } from "@destack/theme/tokens.stylex";
import { Stack } from "../stack/index.ts";
import {
    Editable,
    EditableCancel,
    EditableInput,
    EditablePreview,
    EditableSubmit,
    EditableTrigger,
} from "./editable.tsx";

/** The styles of the examples. */
const styles = style.create({
    title: {
        margin: 0,
        fontWeight: weight.bold,
    },
});

/** A page's title and what it is about. */
export const editablePage = defineExample({
    of: Editable,
    name: "page",
    description: t`A page's title, put back where nothing is left, and what it is about, in lines.`,
    layout: "padded",
    render: (properties) => (
        <Stack space="3">
            <h1 {...style.attrs(text["3xl"], styles.title)}>
                <Editable
                    required
                    defaultValue="Spring in Lisbon"
                    placeholder="Untitled"
                    {...properties}
                />
            </h1>
            <Editable
                multiline
                defaultValue={"Two weeks in April.\nThe light is best before ten."}
                placeholder="What is it about?"
            />
        </Stack>
    ),
});

/** A name with buttons that change, keep and leave it. */
export const editableButtons = defineExample({
    of: Editable,
    name: "buttons",
    description: t`A name with buttons that change, keep and leave it, and an empty one.`,
    layout: "padded",
    render: (properties) => (
        <Stack space="4">
            <Editable activationMode="none" submitMode="enter" defaultValue="Ada Lovelace">
                <EditablePreview />
                <EditableInput aria-label="Name" />

                <EditableTrigger />
                <EditableSubmit />
                <EditableCancel />
            </Editable>
            <Editable placeholder="Add a nickname" {...properties} />
        </Stack>
    ),
});
