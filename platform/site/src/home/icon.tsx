import { createUniqueId } from "solid-js";

const RING_PATH =
    "M282.69 332.02C429.66 278.53 536.68 201.84 521.72 160.74C506.76 119.64 375.49 129.68 228.51 183.18C81.54 236.67 -25.48 313.36 -10.52 354.46C4.44 395.56 135.71 385.52 282.69 332.02Z";

type IconProps = {
    class?: string;
};

export function Icon(props: IconProps) {
    const id = createUniqueId();
    const backMask = `${id}-ring-back`;
    const frontMask = `${id}-ring-front`;

    return (
        <svg
            class={props.class}
            fill="none"
            viewBox="-64 -72 640 640"
            xmlns="http://www.w3.org/2000/svg"
        >
            <defs>
                <mask id={backMask} maskUnits="userSpaceOnUse">
                    <rect x="-64" y="-72" width="640" height="640" fill="#000" />
                    <path
                        d="M468.771 -200.29L-200.29 43.2286L-78.5306 377.759L590.531 134.241L468.771 -200.29Z"
                        fill="#fff"
                    />
                </mask>
                <mask id={frontMask} maskUnits="userSpaceOnUse">
                    <rect x="-64" y="-72" width="640" height="640" fill="#000" />
                    <path
                        d="M590.531 134.241L-78.5305 377.759L24.0755 659.667L693.137 416.149L590.531 134.241Z"
                        fill="#fff"
                    />
                </mask>
            </defs>

            <path
                d={RING_PATH}
                mask={`url(#${backMask})`}
                stroke="currentColor"
                stroke-width="44"
            />

            <circle cx="258" cy="248" r="185" fill="var(--color-destack-accent)" />

            <path
                d={RING_PATH}
                mask={`url(#${frontMask})`}
                stroke="currentColor"
                stroke-width="44"
            />
        </svg>
    );
}
