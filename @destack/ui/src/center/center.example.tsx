import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color } from "@destack/theme/tokens.stylex";
import { Button } from "../button/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { Stack } from "../stack/index.ts";
import { Center } from "./center.tsx";

/** The styles of the centered text. */
const styles = style.create({
    title: {
        margin: 0,
    },
    note: {
        margin: 0,
        color: color.mutedForeground,
    },
});

/** An article centered at a readable line length. */
export const centerArticle = defineExample({
    of: Center,
    name: "article",
    description: "an article centered at a readable line length",
    render: () => (
        <Center render={(attributes) => <article {...attributes} />}>
            <Stack>
                <h1 {...style.attrs(text["2xl"], styles.title)}>Introducing Destack</h1>
                <p {...style.attrs(text.base, styles.note)}>
                    Software should be open, hackable and remixable: a stack you can own.
                </p>
            </Stack>
        </Center>
    ),
});

/** A sign-in form centered at its own width. */
export const centerForm = defineExample({
    of: Center,
    name: "form",
    description: "a sign-in form centered at its own width",
    properties: {
        intrinsic: true,
        max: "24rem",
    },
    render: (properties) => (
        <Center {...properties}>
            <Stack>
                <h1 {...style.attrs(text.xl, styles.title)}>Sign in</h1>
                <p {...style.attrs(text.sm, styles.note)}>Use the passkey on this device.</p>
                <Field>
                    <FieldLabel>Email</FieldLabel>
                    <Input type="email" name="email" placeholder="ada@example.com" />
                </Field>
                <Button type="submit">Continue with a passkey</Button>
            </Stack>
        </Center>
    ),
});
