import { defineExample } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { Cover, CoverCenter } from "./cover.tsx";

/** A welcome screen with its heading centered and its actions at the bottom. */
export const coverWelcome = defineExample({
    of: Cover,
    name: "welcome",
    description: "a welcome screen with its heading centered and its actions at the bottom",
    render: () => (
        <Cover minHeight="32rem">
            <p>Destack</p>
            <CoverCenter render={(attributes) => <h1 {...attributes} />}>
                Take back your software
            </CoverCenter>
            <Button>Get started</Button>
        </Cover>
    ),
});
