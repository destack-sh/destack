import { defineExample } from "@destack/package/declare";
import { Glyph } from "./glyph.tsx";

/** The icons of three pages: an emoji, a named icon and an uploaded image. */
export const glyphPageIcons = defineExample({
    of: Glyph,
    name: "page-icons",
    description: "the icons of three pages: an emoji, a named icon and an uploaded image",
    render: () => (
        <span style={{ display: "flex", gap: "0.5rem" }}>
            <Glyph glyph={{ emoji: "📘" }} label="Handbook" />
            <Glyph glyph={{ icon: "rocket-launch" }} label="Launch" />
            <Glyph glyph={{ source: "/people/ada.jpg" }} label="Ada's notes" />
        </span>
    ),
});
