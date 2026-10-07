import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import {
    Aurora,
    Blur,
    Chrome,
    Circle,
    CursorTrail,
    defineEffect,
    defineShader,
    Dither,
    Duotone,
    FilmGrain,
    IrisWipe,
    LinearGradient,
    RadialGradient,
    Star,
    transformColor,
    transformPosition,
    WaveDistortion,
    wgsl,
} from "../effect/index.ts";
import { Shader } from "./shader.tsx";

/** The size every example draws at. */
const styles = style.create({
    hero: { width: "100%", height: "16rem" },
});

/** An aurora in the theme's accent behind a page's hero, a texture drawing on its own. */
export const shaderAuroraHero = defineExample({
    of: Shader,
    name: "aurora-hero",
    description:
        "an aurora in the theme's accent behind a page's hero, a texture drawing on its own",
    render: () => (
        <Shader xstyle={styles.hero}>
            <Aurora colorA={color.primary} colorB={color.accent} colorC={color.background} />
        </Shader>
    ),
});

/** A radial glow rippled by a wave and grained like film, effects wrapping the layers below them. */
export const shaderRippledGlow = defineExample({
    of: Shader,
    name: "rippled-glow",
    description:
        "a radial glow rippled by a wave and grained like film, effects wrapping the layers below them",
    render: () => (
        <Shader xstyle={styles.hero}>
            <FilmGrain strength={0.2}>
                <WaveDistortion strength={0.3}>
                    <RadialGradient colorA={color.primary} colorB={color.background} />
                </WaveDistortion>
            </FilmGrain>
        </Shader>
    ),
});

/** A trail following the pointer over the background, an interactive effect. */
export const shaderCursorTrail = defineExample({
    of: Shader,
    name: "cursor-trail",
    description: "a trail following the pointer over the background, an interactive effect",
    render: () => (
        <Shader xstyle={styles.hero}>
            <CursorTrail colorA={color.primary} colorB={color.accent} />
        </Shader>
    ),
});

/** An aurora where the device draws no GPU effects, showing its fallback. */
export const shaderFallback = defineExample({
    of: Shader,
    name: "fallback",
    description: "an aurora where the device draws no GPU effects, showing its fallback",
    render: () => (
        <Shader xstyle={styles.hero} fallback={<p>The aurora needs WebGPU.</p>}>
            <Aurora colorA={color.primary} />
        </Shader>
    ),
});

/** A halo of rings, a custom effect painted from its own WGSL. */
const Halo = defineEffect(
    defineShader({
        name: "Halo",
        animatedTime: { speed: "speed" },
        props: {
            inner: { default: "#ffd166", transform: transformColor },
            outer: { default: "#0b132b", transform: transformColor },
            center: { default: { x: 0.5, y: 0.5 }, transform: transformPosition },
            radius: { default: 0.6 },
            bands: { default: 4 },
            speed: { default: 1 },
        },
        paint: wgsl`
            let d = length((uv - center) * vec2f(aspect, 1.0)) / radius;
            let wave = 0.5 + 0.5 * cos(d * bands * 6.2831853 - time * 2.0);
            return vec4f(mix(outer.rgb, inner.rgb, wave * (1.0 - smoothstep(0.7, 1.0, d))), 1.0);
        `,
    }),
);

/** A circle in the theme's accent softened by a blur, an effect of the blurs family. */
export const shaderBlurredCircle = defineExample({
    of: Shader,
    name: "blurred-circle",
    description: "a circle in the theme's accent softened by a blur, an effect of the blurs family",
    render: () => (
        <Shader xstyle={styles.hero}>
            <Blur intensity={24}>
                <Circle color={color.primary} />
            </Blur>
        </Shader>
    ),
});

/** A five-pointed star in the theme's accent, a shape. */
export const shaderStar = defineExample({
    of: Shader,
    name: "star",
    description: "a five-pointed star in the theme's accent, a shape",
    render: () => (
        <Shader xstyle={styles.hero}>
            <Star color={color.primary} sides={5} />
        </Shader>
    ),
});

/** A star finished in chrome, an effect of the shape effects family. */
export const shaderChromeStar = defineExample({
    of: Shader,
    name: "chrome-star",
    description: "a star finished in chrome, an effect of the shape effects family",
    render: () => (
        <Shader xstyle={styles.hero}>
            <Chrome>
                <Star sides={5} />
            </Chrome>
        </Shader>
    ),
});

/** A gradient revealed through an iris half open, a transition. */
export const shaderIrisWipe = defineExample({
    of: Shader,
    name: "iris-wipe",
    description: "a gradient revealed through an iris half open, a transition",
    render: () => (
        <Shader xstyle={styles.hero}>
            <IrisWipe progress={0.5}>
                <LinearGradient colorA={color.primary} colorB={color.accent} />
            </IrisWipe>
        </Shader>
    ),
});

/** An aurora recolored in two theme colors, an adjustment. */
export const shaderDuotoneAurora = defineExample({
    of: Shader,
    name: "duotone-aurora",
    description: "an aurora recolored in two theme colors, an adjustment",
    render: () => (
        <Shader xstyle={styles.hero}>
            <Duotone colorA={color.background} colorB={color.primary}>
                <Aurora />
            </Duotone>
        </Shader>
    ),
});

/** A radial glow dithered in two theme colors, an effect of the stylize family. */
export const shaderDitheredGlow = defineExample({
    of: Shader,
    name: "dithered-glow",
    description: "a radial glow dithered in two theme colors, an effect of the stylize family",
    render: () => (
        <Shader xstyle={styles.hero}>
            <Dither colorA={color.background} colorB={color.primary}>
                <RadialGradient colorA={color.primary} colorB={color.background} />
            </Dither>
        </Shader>
    ),
});

/** A halo of rings, a custom effect defined from WGSL, in tone-mapped linear Display P3. */
export const shaderCustomHalo = defineExample({
    of: Shader,
    name: "custom-halo",
    description:
        "a halo of rings, a custom effect defined from WGSL, in tone-mapped linear Display P3",
    render: () => (
        <Shader xstyle={styles.hero} colorSpace="p3-linear" toneMapping="aces">
            <Halo inner={color.primary} outer={color.background} bands={6} />
        </Shader>
    ),
});
