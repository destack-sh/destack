import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";

import { PerlinNoise } from "./perlin-noise.tsx";

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** The perlin noise in the default look. */
export const perlinNoiseDefault = defineExample({
    of: PerlinNoise,
    name: "default",
    description: t`Perlin noise in the default look.`,
    render: (properties) => <PerlinNoise {...properties} xstyle={styles.canvas} />,
});

/** The perlin noise in the pixel water look. */
export const perlinNoisePixelWater = defineExample({
    of: PerlinNoise,
    name: "pixelWater",
    description: t`Perlin noise in the pixel water look.`,
    properties: {
        scale: 1 / 0.2,
        speed: 0.4,
        frame: 0,
        colorBack: "#2d69d4",
        colorFront: "#d1eefc",
        proportion: 0.42,
        softness: 0,
        octaveCount: 2,
        persistence: 0.55,
        lacunarity: 1.8,
    },
    render: (properties) => <PerlinNoise {...properties} xstyle={styles.canvas} />,
});

/** The perlin noise in the moss look. */
export const perlinNoiseMoss = defineExample({
    of: PerlinNoise,
    name: "moss",
    description: t`Perlin noise in the moss look.`,
    properties: {
        scale: 1 / 0.15,
        speed: 0.02,
        frame: 0,
        colorBack: "#05ff4a",
        colorFront: "#262626",
        proportion: 0.65,
        softness: 0.35,
        octaveCount: 6,
        persistence: 1,
        lacunarity: 2.55,
    },
    render: (properties) => <PerlinNoise {...properties} xstyle={styles.canvas} />,
});

/** The perlin noise in the worms look. */
export const perlinNoiseWorms = defineExample({
    of: PerlinNoise,
    name: "worms",
    description: t`Perlin noise in the worms look.`,
    properties: {
        scale: 0.9,
        speed: 0,
        frame: 0,
        colorBack: "#ffffff00",
        colorFront: "#595959",
        proportion: 0.5,
        softness: 0,
        octaveCount: 1,
        persistence: 1,
        lacunarity: 1.5,
    },
    render: (properties) => <PerlinNoise {...properties} xstyle={styles.canvas} />,
});
