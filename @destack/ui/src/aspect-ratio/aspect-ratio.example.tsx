import { defineExample } from "@destack/package/declare";
import { AspectRatio } from "./aspect-ratio.tsx";

/** A note's cover photo at 16 by 9 whatever the width. */
export const aspectRatioCoverPhoto = defineExample({
    of: AspectRatio,
    name: "cover-photo",
    description: "a note's cover photo at 16 by 9 whatever the width",
    render: () => (
        <AspectRatio ratio={16 / 9}>
            <img src="/photos/alfama.jpg" alt="Alfama at dusk" width="100%" height="100%" />
        </AspectRatio>
    ),
});
