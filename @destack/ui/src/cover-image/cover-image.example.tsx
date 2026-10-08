import { defineExample } from "@destack/package/declare";
import { CoverImage } from "./cover-image.tsx";

/** A page's cover centred a third of the way down, repositionable by dragging or the arrow keys. */
export const coverImagePageCover = defineExample({
    of: CoverImage,
    name: "page-cover",
    description:
        "a page's cover centred a third of the way down, repositionable by dragging or the arrow keys",
    render: () => (
        <CoverImage
            source="/covers/mountains.jpg"
            defaultPosition={0.33}
            label="Reposition the cover"
            onPositionChange={() => undefined}
        />
    ),
});
