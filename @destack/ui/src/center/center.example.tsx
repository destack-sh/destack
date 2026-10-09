import { defineExample } from "@destack/package/declare";
import { Center } from "./center.tsx";

/** An article centered at a readable line length. */
export const centerArticle = defineExample({
    of: Center,
    name: "article",
    description: "an article centered at a readable line length",
    render: () => (
        <Center render={(attributes) => <article {...attributes} />}>
            <h1>Introducing Destack</h1>
            <p>Software should be open, hackable and remixable: a stack you can own.</p>
        </Center>
    ),
});

/** A sign-in form centered at its own width. */
export const centerForm = defineExample({
    of: Center,
    name: "form",
    description: "a sign-in form centered at its own width",
    render: () => (
        <Center intrinsic max="24rem">
            <h1>Sign in</h1>
            <p>Use the passkey on this device.</p>
        </Center>
    ),
});
