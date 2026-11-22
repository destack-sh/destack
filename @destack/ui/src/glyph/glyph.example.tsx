import { defineExample } from "@destack/package/declare";
import { PICTURES } from "../example/picture.ts";
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
            <Glyph glyph={{ source: PICTURES.ada }} label="Ada's notes" />
        </span>
    ),
});

/** The icons of three pages at the extra large size. */
export const glyphPageIconsLarge = defineExample({
    of: Glyph,
    name: "page-icons-large",
    description: "the icons of three pages at the extra large size",
    properties: {
        size: "xl",
    },
    render: (properties) => (
        <span style={{ display: "flex", gap: "0.5rem" }}>
            <Glyph {...properties} glyph={{ emoji: "📘" }} label="Handbook" />
            <Glyph {...properties} glyph={{ icon: "rocket-launch" }} label="Launch" />
            <Glyph {...properties} glyph={{ source: PICTURES.ada }} label="Ada's notes" />
        </span>
    ),
});
