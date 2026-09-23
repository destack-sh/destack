import * as stylex from "@destack/style";
import { type JSX, onSettled } from "@destack/view";

import { sound } from "../effect/sound";
import { waveAt } from "../effect/water";
import { tokens } from "../style/tokens.stylex";

/// The slowest and fastest drift, in CSS pixels per second.
const slowest = 9;
const fastest = 17;
/// The longest calm before a piece drifts in again, in seconds.
const longestCalm = 30;
/// The chance each second that a piece on the water splashes.
const splashChance = 0.12;
/// The least room between two pieces on the water, as a share of its width, so at most two show at once.
const leastGap = 0.6;

/// One piece of flotsam: its drawing, how it sits in the water, where its tag hangs, and the tags it wears.
type Piece = {
    /// The drawing, in CSS pixels.
    art: () => JSX.Element;
    /// The drawing's width in CSS pixels.
    width: number;
    /// The drawing's height in CSS pixels.
    height: number;
    /// How deep the drawing sits below the surface, in CSS pixels.
    draft: number;
    /// The point the tag's string is tied to, in the drawing's CSS pixels.
    tie: readonly [number, number];
    /// The short tags it cycles through, one per pass across the water.
    tags: readonly string[];
};

/// The flotsam of vendor software.
const pieces: readonly Piece[] = [
    {
        art: Lifeboat,
        width: 88,
        height: 46,
        draft: 16,
        tie: [82, 26],
        tags: ["Acquired", "Now sunsetting", "Now AI-first", "Pivoted", "Our journey ends"],
    },
    {
        art: LifeRing,
        width: 40,
        height: 40,
        draft: 18,
        tie: [37, 22],
        tags: ["Talk to sales", "Price +40%", "Per seat", "Enterprise only", "Annual billing"],
    },
    {
        art: Buoy,
        width: 30,
        height: 44,
        draft: 14,
        tie: [25, 28],
        tags: ["Deprecated", "Rate limited", "Upgrade to Enterprise", "API v1 EOL", "Book a demo"],
    },
    {
        art: Duck,
        width: 46,
        height: 40,
        draft: 12,
        tie: [41, 28],
        tags: [
            "We value your feedback",
            "Ticket #48213",
            "Contact your admin",
            "Trial expired",
            "Export 3–5 days",
        ],
    },
];

/// One piece's drift across the water.
type Drift = {
    /// Where the piece is across the water, in CSS pixels.
    x: number;
    /// How fast it drifts, in CSS pixels per second.
    speed: number;
    /// The tag it wears on this pass.
    tag: number;
    /// How far it still has to bob up after being tossed onto the water, from 1 to 0.
    rise: number;
};

/// Float the flotsam of vendor software along the waterline, each piece riding the waves at its own pace.
export function Flotsam(props: { isAdrift: boolean; surfacedAt: number; waterline: string }) {
    let water!: HTMLDivElement;
    const elements: HTMLDivElement[] = [];
    const tags: HTMLSpanElement[] = [];

    onSettled(() => {
        let frame: number | undefined;
        let last = performance.now();

        // start each piece somewhere random, some already on the water and some still to come
        const width = () => water.clientWidth;
        const launch = (piece: Piece, x: number): Drift => ({
            x,
            speed: slowest + Math.random() * (fastest - slowest),
            tag: Math.floor(Math.random() * piece.tags.length),
            rise: 0,
        });

        // spread the first pieces out with room between them, the rest queued up off the left edge
        let behind = width() * (0.2 + Math.random() * 0.5);
        const scatter = () =>
            pieces.map((piece) => {
                const drift = launch(piece, behind);
                behind -= width() * leastGap + Math.random() * longestCalm * fastest;
                return drift;
            });
        let drifts = scatter();
        let surfaced = props.surfacedAt;
        drifts.forEach((drift, index) => {
            tags[index].textContent = pieces[index].tags[drift.tag];
        });

        // drift each piece along, riding and tilting with the waves, and send it round again with a new tag
        const loop = (now: number) => {
            frame = requestAnimationFrame(loop);
            const elapsed = Math.min(0.1, (now - last) / 1000);
            last = now;
            if (!props.isAdrift) {
                return;
            }

            // toss the pieces back up onto the refilled water, spread across it
            if (props.surfacedAt !== surfaced) {
                surfaced = props.surfacedAt;
                behind = width() * (0.35 + Math.random() * 0.4);
                drifts = scatter();
                drifts.forEach((drift, index) => {
                    drift.rise = 1;
                    tags[index].textContent = pieces[index].tags[drift.tag];
                });
                sound.play("splash");
            }

            const seconds = now / 1000;
            pieces.forEach((piece, index) => {
                const drift = drifts[index];

                // keep clear of the piece ahead by matching its pace when closing in
                const ahead = drifts.filter((other) => other.x > drift.x);
                const nearest = ahead.reduce((best, other) => (other.x < best.x ? other : best), {
                    x: Infinity,
                    speed: drift.speed,
                    tag: 0,
                });
                if (nearest.x - drift.x < width() * leastGap) {
                    drift.speed = Math.min(drift.speed, nearest.speed);
                }
                drift.x += drift.speed * elapsed;
                if (drift.x > width() + 20) {
                    const trailing = Math.min(...drifts.map((other) => other.x));
                    const start = Math.min(-piece.width, trailing - width() * leastGap);
                    const next = launch(piece, start - Math.random() * longestCalm * fastest);
                    next.tag = (drift.tag + 1) % piece.tags.length;
                    drifts[index] = next;
                    tags[index].textContent = piece.tags[next.tag];
                }

                // splash now and then while on the water
                const isAfloat = drifts[index].x > -piece.width && drifts[index].x < width();
                if (isAfloat && Math.random() < splashChance * elapsed) {
                    sound.play("splash");
                }

                // sit on the wave under the piece's middle, leaning with its slope
                const x = drifts[index].x;
                const middle = x + piece.width / 2;
                const rise = waveAt(middle, seconds);
                const slope = waveAt(middle + 6, seconds) - waveAt(middle - 6, seconds);
                drifts[index].rise *= 0.965;
                const y = rise - piece.height + piece.draft + drifts[index].rise * 48;
                const lean = (Math.atan2(slope, 12) * 180) / Math.PI;
                elements[index].style.transform =
                    `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) rotate(${lean.toFixed(2)}deg)`;
            });
        };
        frame = requestAnimationFrame(loop);

        return () => {
            if (frame !== undefined) {
                cancelAnimationFrame(frame);
            }
        };
    });

    return (
        <div
            ref={water}
            aria-hidden="true"
            style={{ top: props.waterline }}
            {...stylex.attrs(styles.water, props.isAdrift && styles.adrift)}
        >
            {pieces.map((piece, index) => (
                <div
                    ref={(element) => {
                        elements[index] = element;
                    }}
                    style={{ width: `${piece.width}px`, height: `${piece.height}px` }}
                    {...stylex.attrs(styles.piece)}
                >
                    {piece.art()}

                    {/* trail the tag on a slack string from its tie */}
                    <svg {...stylex.attrs(styles.string)}>
                        <path
                            d={`M${piece.tie[0]} ${piece.tie[1]} Q${piece.tie[0] + 6} ${piece.tie[1] + 5} ${piece.tie[0] + 13} ${piece.tie[1] - 2}`}
                        />
                    </svg>
                    <span
                        ref={(element) => {
                            tags[index] = element;
                        }}
                        style={{
                            left: `${piece.tie[0] + 12}px`,
                            top: `${piece.tie[1] - 12}px`,
                        }}
                        {...stylex.attrs(styles.tag)}
                    >
                        {piece.tags[0]}
                    </span>
                </div>
            ))}
        </div>
    );
}

/// Draw an orange lifeboat with a castaway rowing.
function Lifeboat() {
    return (
        <svg width="88" height="46" viewBox="0 0 88 46" {...stylex.attrs(styles.art)}>
            <path d="M34 20 L60 44" {...stylex.attrs(styles.oar)} />
            <circle cx="30" cy="10" r="5" {...stylex.attrs(styles.skin)} />
            <path d="M30 15 V26 M30 19 L36 22" {...stylex.attrs(styles.limb)} />
            <path
                d="M2 26 H86 L80 40 Q78 44 72 44 H16 Q10 44 8 40 Z"
                {...stylex.attrs(styles.hull)}
            />
            <path d="M6 31 H82" {...stylex.attrs(styles.stripe)} />
            <circle cx="80" cy="22" r="2" {...stylex.attrs(styles.brass)} />
        </svg>
    );
}

/// Draw a red and white life ring.
function LifeRing() {
    return (
        <svg width="40" height="40" viewBox="0 0 40 40" {...stylex.attrs(styles.art)}>
            <circle cx="20" cy="22" r="13" {...stylex.attrs(styles.ring)} />
            <circle cx="20" cy="22" r="13" pathLength="8" {...stylex.attrs(styles.ringStripes)} />
            <circle cx="20" cy="22" r="17.5" {...stylex.attrs(styles.edge)} />
            <circle cx="20" cy="22" r="8.5" {...stylex.attrs(styles.edge)} />
        </svg>
    );
}

/// Draw a red and white warning buoy with a lamp on top.
function Buoy() {
    return (
        <svg width="30" height="44" viewBox="0 0 30 44" {...stylex.attrs(styles.art)}>
            <circle cx="15" cy="6" r="5" {...stylex.attrs(styles.glow)} />
            <circle cx="15" cy="6" r="2.5" {...stylex.attrs(styles.lamp)} />
            <path d="M11 22 V9 H19 V22 M11 15 H19" {...stylex.attrs(styles.frame)} />
            <path d="M5 22 H25 L22 42 H8 Z" {...stylex.attrs(styles.buoyRed)} />
            <path d="M6.1 28 H23.9 L23.1 34 H6.9 Z" {...stylex.attrs(styles.buoyWhite)} />
        </svg>
    );
}

/// Draw a yellow rubber duck.
function Duck() {
    return (
        <svg width="46" height="40" viewBox="0 0 46 40" {...stylex.attrs(styles.art)}>
            <path
                d="M4 24 Q4 38 22 38 Q40 38 40 26 Q40 20 32 20 L10 20 Q6 16 4 24 Z"
                {...stylex.attrs(styles.duck)}
            />
            <circle cx="30" cy="12" r="9" {...stylex.attrs(styles.duck)} />
            <path d="M38 11 L45 13 L38 16 Z" {...stylex.attrs(styles.beak)} />
            <circle cx="32" cy="9" r="1.4" {...stylex.attrs(styles.eye)} />
            <path d="M14 27 Q20 32 27 27" {...stylex.attrs(styles.wing)} />
        </svg>
    );
}

const styles = stylex.create({
    water: {
        clipPath: "inset(-100vh 0 -100vh 0)",
        height: 0,
        left: 0,
        opacity: 0,
        pointerEvents: "none",
        position: "absolute",
        right: 0,
        transition: "opacity 1200ms ease",
        zIndex: 1,
        "@media (prefers-reduced-motion: reduce)": { display: "none" },
    },
    adrift: {
        opacity: 1,
    },
    piece: {
        left: 0,
        position: "absolute",
        top: 0,
        transformOrigin: "50% 100%",
        willChange: "transform",
    },
    art: {
        display: "block",
        overflow: "visible",
    },
    string: {
        fill: "none",
        height: "1px",
        left: 0,
        overflow: "visible",
        position: "absolute",
        stroke: tokens.signalInk,
        strokeWidth: 1,
        top: 0,
        width: "1px",
    },
    tag: {
        backgroundColor: tokens.cream,
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: tokens.signalInk,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        fontWeight: 700,
        letterSpacing: "0.06em",
        paddingBlock: "0.0625rem",
        paddingInline: "0.375rem",
        position: "absolute",
        textTransform: "uppercase",
        transform: "rotate(-3deg)",
        transformOrigin: "0 50%",
        whiteSpace: "nowrap",
    },
    skin: {
        fill: "#f2c9a0",
        stroke: tokens.signalInk,
        strokeWidth: 1.25,
    },
    limb: {
        fill: "none",
        stroke: tokens.signalInk,
        strokeLinecap: "round",
        strokeWidth: 2.5,
    },
    oar: {
        fill: "none",
        stroke: "#8a5a32",
        strokeLinecap: "round",
        strokeWidth: 2.5,
    },
    hull: {
        fill: tokens.signal,
        stroke: tokens.signalInk,
        strokeLinejoin: "round",
        strokeWidth: 1.5,
    },
    stripe: {
        stroke: tokens.cream,
        strokeWidth: 2.5,
    },
    brass: {
        fill: "#e8c27a",
        stroke: tokens.signalInk,
        strokeWidth: 1,
    },
    ring: {
        fill: "none",
        stroke: "#ffffff",
        strokeWidth: 8,
    },
    ringStripes: {
        fill: "none",
        stroke: "#e2462c",
        strokeDasharray: "1 1",
        strokeWidth: 8,
    },
    edge: {
        fill: "none",
        stroke: tokens.signalInk,
        strokeWidth: 1.25,
    },
    glow: {
        fill: "#ffe7a3",
        opacity: 0.45,
    },
    lamp: {
        fill: "#ffd166",
        stroke: tokens.signalInk,
        strokeWidth: 1,
    },
    frame: {
        fill: "none",
        stroke: tokens.signalInk,
        strokeWidth: 1.5,
    },
    buoyRed: {
        fill: "#e2462c",
        stroke: tokens.signalInk,
        strokeLinejoin: "round",
        strokeWidth: 1.5,
    },
    buoyWhite: {
        fill: "#ffffff",
    },
    duck: {
        fill: "#ffd23f",
        stroke: tokens.signalInk,
        strokeLinejoin: "round",
        strokeWidth: 1.5,
    },
    beak: {
        fill: tokens.signal,
        stroke: tokens.signalInk,
        strokeLinejoin: "round",
        strokeWidth: 1.25,
    },
    eye: {
        fill: tokens.signalInk,
    },
    wing: {
        fill: "none",
        stroke: tokens.signalInk,
        strokeLinecap: "round",
        strokeWidth: 1.25,
    },
});
