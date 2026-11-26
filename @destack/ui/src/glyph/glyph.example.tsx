import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { PICTURES } from "../example/picture.ts";
import { Glyph } from "./glyph.tsx";

/** The icons of three pages: an emoji, a named icon and an uploaded image. */
export const glyphPageIcons = defineExample({
    of: Glyph,
    name: "pageIcons",
    description: t`The icons of three pages: an emoji, a named icon and an uploaded image.`,
    render: (properties) => (
        <span style={{ display: "flex", gap: "0.5rem" }}>
            <Glyph {...properties} glyph={{ emoji: "📘" }} label="Handbook" />
            <Glyph glyph={{ icon: "rocket-launch" }} label="Launch" />
            <Glyph glyph={{ source: PICTURES.ada }} label="Ada's notes" />
        </span>
    ),
});
