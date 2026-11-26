import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { SAMPLE_IMAGE } from "../shader/sample.ts";
import { Water } from "./water.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The water in the default look. */
export const waterDefault = defineExample({
    of: Water,
    name: "default",
    description: t`Water in the default look.`,
    render: (properties) => <Water {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />,
});

/** The water in the abstract look. */
export const waterAbstract = defineExample({
    of: Water,
    name: "abstract",
    description: t`Water in the abstract look.`,
    properties: {
        fit: "cover",
        scale: 3,
        speed: 1,
        frame: 0,
        colorBack: "#909090",
        colorHighlight: "#ffffff",
        highlights: 0,
        layering: 0,
        edges: 1,
        waves: 1,
        caustic: 0.4,
        size: 0.15,
    },
    render: (properties) => <Water {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />,
});

/** The water in the streaming look. */
export const waterStreaming = defineExample({
    of: Water,
    name: "streaming",
    description: t`Water in the streaming look.`,
    properties: {
        fit: "contain",
        scale: 0.4,
        speed: 2,
        frame: 0,
        colorBack: "#909090",
        colorHighlight: "#ffffff",
        highlights: 0,
        layering: 0,
        edges: 0,
        waves: 0.5,
        caustic: 0,
        size: 0.5,
    },
    render: (properties) => <Water {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />,
});

/** The water in the slow mo look. */
export const waterSlowMo = defineExample({
    of: Water,
    name: "slowMo",
    description: t`Water in the slow mo look.`,
    properties: {
        fit: "cover",
        scale: 1,
        speed: 0.1,
        frame: 0,
        colorBack: "#909090",
        colorHighlight: "#ffffff",
        highlights: 0.4,
        layering: 0,
        edges: 0,
        waves: 0,
        caustic: 0.2,
        size: 0.7,
    },
    render: (properties) => <Water {...properties} image={SAMPLE_IMAGE} xstyle={styles.canvas} />,
});
