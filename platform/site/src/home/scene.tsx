import type { JSX } from "@destack/view";

/** Ring inclination. */
const ringTilt = "rotate(-22 480 254)";
/** Cloud inclination. */
const cloudTilt = "rotate(-10 480 254)";

/** Place the ringed planet above the lunar foreground. */
export function Scene() {
    return (
        <div class="landscape" aria-hidden="true">
            <Planet />
        </div>
    );
}

/** Extend a low, smooth foreground behind the invitation and footer. */
export function Horizon() {
    return (
        <svg class="horizon" viewBox="0 0 1000 160" aria-hidden="true">
            <path
                d="M-2000 595Q500 -655 3000 595V2000H-2000Z"
                fill="var(--destack-color-background)"
                stroke="#33535b"
                stroke-width="1"
                vector-effect="non-scaling-stroke"
            />
        </svg>
    );
}

/** Align the planet and ring in one viewport. */
function SceneLayer(props: { children: JSX.Element }) {
    return (
        <svg
            aria-hidden="true"
            class="orbit-scene"
            preserveAspectRatio="xMidYMid meet"
            viewBox="150 60 660 388"
            xmlns="http://www.w3.org/2000/svg"
        >
            {props.children}
        </svg>
    );
}

/** Draw the globe and its divided rings. */
function Planet() {
    return (
        <SceneLayer>
            <defs>
                <clipPath id="poster-globe">
                    <circle cx="480" cy="254" r="176" />
                </clipPath>
            </defs>

            {/* draw the back of the ring before the globe */}
            <g class="orbit-planet">
                <g transform={ringTilt}>
                    <Ring path="M174 254 A306 82 0 0 1 786 254" />
                </g>

                <g clip-path="url(#poster-globe)">
                    <circle cx="480" cy="254" r="176" fill="#dc5b2d" />
                    <g transform={cloudTilt}>
                        <path
                            d="M280 118 Q480 185 680 118 L680 155 Q480 212 280 155Z"
                            fill="#e77443"
                        />
                        <path
                            d="M280 176 Q480 226 680 176 L680 209 Q480 257 280 209Z"
                            fill="#a44328"
                        />
                        <path
                            d="M280 235 Q480 278 680 235 L680 251 Q480 296 280 251Z"
                            fill="#ee8952"
                        />
                        <path
                            d="M280 276 Q480 319 680 276 L680 315 Q480 355 280 315Z"
                            fill="#aa4729"
                        />
                        <path
                            d="M280 338 Q480 383 680 338 L680 354 Q480 397 280 354Z"
                            fill="#e77443"
                        />
                        <path d="M280 389 Q480 426 680 389 L680 450 H280Z" fill="#a44328" />
                    </g>
                    {/* follow the ring tilt when projecting its shadow */}
                    <g transform={ringTilt}>
                        <path
                            d="M270 286 Q480 370 690 286"
                            fill="none"
                            stroke="#542e29"
                            stroke-width="20"
                        />
                    </g>
                    {/* shade the cloud bands with two flat curved regions */}
                    <path
                        d="M546 78 C650 190 587 368 416 430 H700 V50Z"
                        fill="#623027"
                        opacity=".55"
                    />
                    <path d="M614 128 C682 267 595 405 480 430 H700 V80Z" fill="#102a33" />
                </g>
                <path
                    d="M306 278 A176 176 0 0 1 515 81"
                    fill="none"
                    stroke="#f1b77a"
                    stroke-width="1.5"
                    opacity=".7"
                />
            </g>
            {/* close the ring in front of the globe */}
            <g transform={ringTilt}>
                <Ring path="M174 254 A306 82 0 0 0 786 254" />
            </g>
        </SceneLayer>
    );
}

/** Draw the divisions along one half of the ring. */
function Ring(props: { path: string }) {
    return (
        <g fill="none" stroke="#f1eadb">
            <path d={props.path} stroke-width="22" />
            <path d={props.path} stroke="var(--destack-color-background)" stroke-width="3" />
            <path d={props.path} stroke="#8b8376" stroke-width="1" />
        </g>
    );
}
