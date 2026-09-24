import * as stylex from "@destack/style";
import { type JSX, onSettled } from "@destack/view";

import { sound } from "../effect/sound";
import { stir, waveAt } from "../effect/water";
import { tokens } from "../style/tokens.stylex";

/** The slowest drift, in CSS pixels per second. */
const slowest = 9;
/** The fastest drift, in CSS pixels per second. */
const fastest = 17;
/** The longest calm before a piece drifts in again, in seconds. */
const longestCalm = 30;
/** The chance each second that a piece on the water splashes. */
const splashChance = 0.12;
/** The least room between two pieces on the water, as a share of its width, so at most two show at once. */
const leastGap = 0.6;
/** The length of the wire between a piece and its tag, in CSS pixels. */
const wireLength = 44;
/** The stretch at either edge of the water over which pieces and tags fade, in CSS pixels. */
const edgeFade = 70;
/** The pull of gravity on a thrown piece, in CSS pixels per second squared. */
const gravity = 1400;
/** The fastest a piece can be thrown, in CSS pixels per second. */
const fastestThrow = 900;
/** The seconds a sinking piece takes to go under. */
const sinkTime = 1.4;
/** The seconds a sunk piece's tag drifts alone before the piece returns. */
const strandTime = 4;
/** The steepest a floating tag tilts, in degrees, so it stays readable. */
const steepestTag = 6;

/** One piece of flotsam: its drawing, how it sits in the water, where its tag hangs, and the tags it wears. */
type Piece = {
    /** The drawing, in CSS pixels. */
    art: () => JSX.Element;
    /** The drawing's width in CSS pixels. */
    width: number;
    /** The drawing's height in CSS pixels. */
    height: number;
    /** How deep the drawing sits below the waterline, in CSS pixels. */
    draft: number;
    /** The point the tag's string is tied to, in the drawing's CSS pixels. */
    hitch: readonly [number, number];
    /** The short tags it cycles through, one per pass across the water. */
    tags: readonly string[];
};

/** The flotsam of vendor software. */
const vendorFlotsam: readonly Piece[] = [
    {
        art: Unicorn,
        width: 72,
        height: 56,
        draft: 14,
        hitch: [5, 32],
        tags: [
            "To the spoons",
            "Sunsetting in 30 days",
            "Our incredible journey",
            "Now with 3 AI buttons",
            "Pivoting to agents",
            "Read-only from Friday",
        ],
    },
    {
        art: Card,
        width: 46,
        height: 32,
        draft: 12,
        hitch: [4, 22],
        tags: [
            "Updated pricing",
            "Usage-based (surprise)",
            "SSO is Enterprise-only",
            "Free plan retired",
            "Renewed for 3 years",
            "Talk to sales to cancel",
            "AI add-on, now included*",
        ],
    },
    {
        art: Buoy,
        width: 30,
        height: 44,
        draft: 14,
        hitch: [6, 30],
        tags: [
            "Now deprecated",
            "Rate limited, try later",
            "API v1 retired today",
            "Breaking change (minor)",
            "On our 73rd incident",
            "Status: all green",
            "Feature moved to Pro",
            "Upgrade to Enterprise",
        ],
    },
    {
        art: Duck,
        width: 46,
        height: 40,
        draft: 12,
        hitch: [5, 24],
        tags: [
            "We value your feedback",
            "Closed as won't fix",
            "Works on our end",
            "Your call is important",
            "Contact your admin (you)",
            "Ticket #48213 auto-closed",
        ],
    },
    {
        art: Chest,
        width: 46,
        height: 38,
        draft: 20,
        hitch: [4, 24],
        tags: [
            "Export ready in 3 days",
            "CSV export only",
            "Your data, our model",
            "Data retained 30 days",
            "Storage limit reached",
            "Account under review",
        ],
    },
    {
        art: Bottle,
        width: 52,
        height: 22,
        draft: 9,
        hitch: [3, 12],
        tags: [
            "We've updated our terms",
            "Action required",
            "Your export link expired",
            "Price change notice",
            "Your seat was removed",
        ],
    },
];

/** One piece's drift across the water. */
type Drift = {
    /** Where the piece is across the water, in CSS pixels. */
    x: number;
    /** How fast the current carries it, in CSS pixels per second. */
    pace: number;
    /** How fast it actually moves across, easing back to the current's pace after a throw. */
    speed: number;
    /** How high it is thrown above the water, in CSS pixels. */
    lift: number;
    /** How fast it rises, in CSS pixels per second, falling back under gravity. */
    climb: number;
    /** The seconds since it started to sink, when sinking. */
    sunk: number | undefined;
    /** Where it was grabbed, relative to its top left, while held. */
    grab: { x: number; y: number } | undefined;
    /** The tag it wears on this pass. */
    tag: number;
    /** How far it still has to bob up after being tossed onto the water, from 1 to 0. */
    rise: number;
    /** Where its tag's grommet floats across the water, in CSS pixels, once placed. */
    tagX: number | undefined;
    /** How fast its tag drifts relative to the water, in CSS pixels per second. */
    tagSpeed: number;
};

/** The lone bottle that drifts past a missing page. */
export const strandedBottle: Piece = {
    art: Bottle,
    width: 52,
    height: 22,
    draft: 9,
    hitch: [3, 12],
    tags: ["This page was sunset"],
};

/** Float flotsam along the waterline, the vendor software by default. */
export function Flotsam(properties: {
    isAdrift: boolean;
    surfacedAt: number;
    waterline: string;
    pieces?: readonly Piece[];
}) {
    // pick the pieces and hold their elements
    const pieces = properties.pieces ?? vendorFlotsam;
    let water!: HTMLDivElement;
    const elements: HTMLDivElement[] = [];
    const tags: HTMLSpanElement[] = [];
    const wires: SVGPathElement[] = [];

    // float the pieces once the water is in the page
    onSettled(() => {
        // hold the frame and the time of the last one
        let frame: number | undefined;
        let last = performance.now();

        // start each piece somewhere random, some already on the water and some still to come
        const width = () => water.clientWidth;
        const launch = (piece: Piece, x: number): Drift => {
            const pace = slowest + Math.random() * (fastest - slowest);
            return {
                x,
                pace,
                speed: pace,
                lift: 0,
                climb: 0,
                sunk: undefined,
                grab: undefined,
                tag: Math.floor(Math.random() * piece.tags.length),
                rise: 0,
                tagX: undefined,
                tagSpeed: 0,
            };
        };

        // track the held piece and the recent path of the pointer holding it
        let held:
            | { index: number; path: { x: number; y: number; at: number }[]; downAt: number }
            | undefined;
        let pointer = { x: 0, y: 0 };

        // locate a pointer event on the water
        const locate = (event: PointerEvent) => {
            const bounds = water.getBoundingClientRect();
            return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
        };

        // pick up a floating piece where it was grabbed
        const grab = (index: number, event: PointerEvent) => {
            // leave sinking pieces alone
            const drift = drifts[index];
            if (drift.sunk !== undefined) {
                return;
            }

            // capture the pointer and hold the piece by the grabbed spot
            event.preventDefault();
            (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
            pointer = locate(event);
            const box = elements[index].getBoundingClientRect();
            const bounds = water.getBoundingClientRect();
            drift.grab = { x: pointer.x - drift.x, y: pointer.y - (box.top - bounds.top) };

            // start the path to throw with
            held = {
                index,
                path: [{ ...pointer, at: performance.now() }],
                downAt: performance.now(),
            };
        };

        // follow the pointer, keeping only the last moments of its path
        const drag = (event: PointerEvent) => {
            if (!held) {
                return;
            }
            pointer = locate(event);
            held.path.push({ ...pointer, at: performance.now() });
            held.path = held.path.filter((point) => performance.now() - point.at < 90);
        };

        // let go of the held piece
        const release = () => {
            if (!held) {
                return;
            }

            // measure the last moments of the path
            const drift = drifts[held.index];
            const first = held.path[0];
            const last = held.path[held.path.length - 1];
            const span = Math.max(16, last.at - first.at) / 1000;
            const travelled = Math.hypot(last.x - first.x, last.y - first.y);
            drift.grab = undefined;

            // sink the piece on a quick tap
            if (performance.now() - held.downAt < 250 && travelled < 6 && drift.lift <= 0) {
                drift.sunk = 0;
                bubble(drift.x + pieces[held.index].width / 2);
                sound.play("splash");
            }
            // throw it at the pointer's last speed otherwise
            else {
                const clamp = (value: number) =>
                    Math.max(-fastestThrow, Math.min(fastestThrow, value));
                drift.speed = clamp((last.x - first.x) / span);
                drift.climb = clamp(-(last.y - first.y) / span);
            }
            held = undefined;
        };

        // listen for grabs on every piece
        for (const [index, element] of elements.entries()) {
            element.addEventListener("pointerdown", (event) => grab(index, event));
            element.addEventListener("pointermove", drag);
            element.addEventListener("pointerup", release);
            element.addEventListener("pointercancel", release);
        }

        // send a few bubbles up from where a piece goes under
        const bubble = (x: number) => {
            for (let index = 0; index < 6; index++) {
                const element = document.createElement("span");
                element.className = stylex.attrs(styles.bubble).class!;
                const size = 3 + Math.random() * 4;
                element.style.cssText = `left:${(x + (Math.random() - 0.5) * 24).toFixed(1)}px;top:${(20 + Math.random() * 20).toFixed(1)}px;width:${size.toFixed(1)}px;height:${size.toFixed(1)}px;animation-delay:${(index * 0.15).toFixed(2)}s`;
                water.append(element);
                setTimeout(() => element.remove(), 2400);
            }
        };

        // spread the first pieces out with room between them, the rest queued up off the left edge
        let behind = width() * (0.2 + Math.random() * 0.5);
        const scatter = () =>
            pieces.map((piece) => {
                const drift = launch(piece, behind);
                behind -= width() * leastGap + Math.random() * longestCalm * fastest;

                return drift;
            });
        let drifts = scatter();
        let surfaced = properties.surfacedAt;
        drifts.forEach((drift, index) => {
            tags[index].textContent = pieces[index].tags[drift.tag];
        });

        // return how visible something spanning across the water is, fading out before it meets either edge
        const fade = (start: number, end: number) =>
            Math.max(0, Math.min(1, (width() - end) / edgeFade, start / edgeFade));

        // drift each piece along, riding and tilting with the waves, and send it round again with a new tag
        const loop = (now: number) => {
            // schedule the next frame and measure the time since the last
            frame = requestAnimationFrame(loop);
            const elapsed = Math.min(0.1, (now - last) / 1000);
            last = now;
            if (!properties.isAdrift) {
                return;
            }

            // toss the pieces back up onto the refilled water, spread across it
            if (properties.surfacedAt !== surfaced) {
                surfaced = properties.surfacedAt;
                behind = width() * (0.35 + Math.random() * 0.4);
                drifts = scatter();
                drifts.forEach((drift, index) => {
                    drift.rise = 1;
                    tags[index].textContent = pieces[index].tags[drift.tag];
                });
                sound.play("splash");
            }

            // move each piece, its tag, and its wire
            const seconds = now / 1000;
            pieces.forEach((piece, index) => {
                // look up the piece's drift
                const drift = drifts[index];

                // keep clear of the piece ahead by matching its pace when closing in
                const ahead = drifts.filter((other) => other.x > drift.x);
                const nearest = ahead.reduce((best, other) => (other.x < best.x ? other : best), {
                    x: Infinity,
                    pace: drift.pace,
                });
                if (nearest.x - drift.x < width() * leastGap) {
                    drift.pace = Math.min(drift.pace, nearest.pace);
                }

                // move the piece: held, flying, sinking, or drifting with the current
                if (drift.grab) {
                    const next = pointer.x - drift.grab.x;
                    drift.speed = (next - drift.x) / Math.max(elapsed, 0.001);
                    drift.x = next;
                } else {
                    drift.x += drift.speed * elapsed;
                    if (drift.lift > 0 || drift.climb > 0) {
                        drift.climb -= gravity * elapsed;
                        drift.lift += drift.climb * elapsed;
                        if (drift.lift <= 0) {
                            stir(drift.x + piece.width / 2, Math.min(8, 1.5 - drift.climb / 150));
                            sound.play("splash");
                            drift.lift = 0;
                            drift.climb = 0;
                        }
                    } else {
                        drift.speed += (drift.pace - drift.speed) * Math.min(1, elapsed * 1.5);
                    }
                }
                if (drift.sunk !== undefined) {
                    drift.sunk += elapsed;
                }
                const isGone = drift.sunk !== undefined && drift.sunk > sinkTime + strandTime;
                if (isGone || drift.x > width() + wireLength + tags[index].offsetWidth + 20) {
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
                const grip = drifts[index].grab;
                const sinking = drifts[index].sunk;
                const depth =
                    sinking === undefined
                        ? 0
                        : (piece.height + 24) * Math.min(1, (sinking / sinkTime) ** 2);
                if (grip) {
                    const floatTop = rise - piece.height + piece.draft;
                    drifts[index].lift = Math.max(0, floatTop - (pointer.y - grip.y));
                }
                const y =
                    rise -
                    piece.height +
                    piece.draft +
                    drifts[index].rise * 48 -
                    drifts[index].lift +
                    depth;
                const lean =
                    drifts[index].lift > 0
                        ? Math.max(-0.6, Math.min(0.6, drifts[index].speed * 0.0012))
                        : Math.atan2(slope, 12);
                elements[index].style.transform =
                    `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) rotate(${lean.toFixed(4)}rad)`;

                // find the hitch on the leaning piece, turning about its bottom middle
                const pivot = { x: middle, y: y + piece.height };
                const arm = { x: x + piece.hitch[0] - pivot.x, y: y + piece.hitch[1] - pivot.y };
                const hitch = {
                    x: pivot.x + arm.x * Math.cos(lean) - arm.y * Math.sin(lean),
                    y: pivot.y + arm.x * Math.sin(lean) + arm.y * Math.cos(lean),
                };

                // tow the floating tag behind the hitch, springing toward a wandering spot and settling
                const tag = tags[index];
                const tether = drifts[index];
                const goal = hitch.x - wireLength * 0.6 + Math.sin(seconds * 0.7 + index * 2.1) * 6;
                tether.tagX ??= goal;
                if (sinking === undefined) {
                    tether.tagSpeed += ((goal - tether.tagX) * 7 - tether.tagSpeed * 2.6) * elapsed;
                    tether.tagX += tether.tagSpeed * elapsed;
                    tether.tagX = Math.max(
                        hitch.x - wireLength,
                        Math.min(hitch.x - 4, tether.tagX),
                    );
                } else {
                    tether.tagSpeed +=
                        (tether.pace * 0.7 - tether.tagSpeed) * Math.min(1, elapsed * 2);
                    tether.tagX += tether.tagSpeed * elapsed;
                }

                // float the tag on the wave under it, tilting gently with the slope
                const tagWidth = tag.offsetWidth;
                const tagHeight = tag.offsetHeight;
                const left = tether.tagX + 5 - tagWidth;
                const waterTop = waveAt(left + tagWidth / 2, seconds);
                const tilt = Math.max(
                    -steepestTag,
                    Math.min(
                        steepestTag,
                        (Math.atan2(
                            waveAt(left + tagWidth, seconds) - waveAt(left, seconds),
                            tagWidth,
                        ) *
                            180) /
                            Math.PI,
                    ),
                );
                const dip =
                    sinking === undefined ? 0 : 16 * Math.sin(Math.min(1, sinking / 1.1) * Math.PI);
                const top = waterTop - tagHeight + 3 + tether.rise * 48 + dip;

                // fade the piece, its tag, and the wire in and out at the edges of the water
                const pieceFade = fade(x, x + piece.width);
                const tagFade = fade(left, left + tagWidth);
                const sinkFade = sinking === undefined ? 1 : Math.max(0, 1 - sinking / sinkTime);
                const strandFade =
                    sinking === undefined
                        ? 1
                        : Math.max(0, Math.min(1, (sinkTime + strandTime - sinking) / 1.2));
                elements[index].style.opacity = String(pieceFade * sinkFade);
                tag.style.opacity = String(tagFade * strandFade);
                wires[index].style.opacity = String(Math.min(pieceFade, tagFade) * sinkFade);
                tag.style.transform = `translate(${left.toFixed(1)}px, ${top.toFixed(1)}px) rotate(${tilt.toFixed(2)}deg)`;

                // run the wire from the hitch to the tag's grommet, sagging while slack
                const hole = { x: tether.tagX, y: top + tagHeight / 2 };
                const span = Math.hypot(hole.x - hitch.x, hole.y - hitch.y);
                const sag = Math.max(0, wireLength - span) * 0.5 + 1.5;
                wires[index].setAttribute(
                    "d",
                    `M${hitch.x.toFixed(1)} ${hitch.y.toFixed(1)}Q${((hitch.x + hole.x) / 2).toFixed(1)} ${((hitch.y + hole.y) / 2 + sag).toFixed(1)} ${hole.x.toFixed(1)} ${hole.y.toFixed(1)}`,
                );
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
            style={{ top: properties.waterline }}
            {...stylex.attrs(styles.water, properties.isAdrift && styles.adrift)}
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
                </div>
            ))}

            {/* float each tag on its own, tied to its piece by a thin wire */}
            {pieces.map((piece, index) => (
                <>
                    <svg {...stylex.attrs(styles.wire)}>
                        <path
                            ref={(element) => {
                                wires[index] = element;
                            }}
                        />
                    </svg>
                    <span
                        ref={(element) => {
                            tags[index] = element;
                        }}
                        {...stylex.attrs(styles.tag)}
                    >
                        {piece.tags[0]}
                    </span>
                </>
            ))}
        </div>
    );
}

/** Draw a patched pool unicorn, deflating, its head drooping. */
function Unicorn() {
    return (
        <svg width="72" height="56" viewBox="0 0 72 56" {...stylex.attrs(styles.art)}>
            <path d="M8 37 Q1 33 4 25 Q6 30 10 31" {...stylex.attrs(styles.outline, styles.pink)} />
            <g transform="rotate(24 50 33)">
                <path
                    d="M44 34 Q45 20 52 15 Q58 11 63 15 Q67 18 66 22 Q64 25 60 24 Q56 23 55 27 L54 36 Z"
                    {...stylex.attrs(styles.outline, styles.vinyl)}
                />
                <path
                    d="M47 29 Q43 25 47 22 Q44 18 49 16 Q48 12 53 12"
                    {...stylex.attrs(styles.mane)}
                />
                <path
                    d="M57 13 Q57.5 8 60 4 L61.5 12.5 Z"
                    transform="rotate(30 59 12)"
                    {...stylex.attrs(styles.outline, styles.gold)}
                />
                <path d="M59.5 17.5 Q61 16.3 62.3 17.5" {...stylex.attrs(styles.lash)} />
            </g>
            <path
                d="M7 42 Q5 31 17 30 H45 Q58 30 58 41 Q58 51 45 51 H17 Q7 51 7 42 Z"
                {...stylex.attrs(styles.outline, styles.vinyl)}
            />
            <path d="M15 43 Q30 46 50 43" {...stylex.attrs(styles.seam)} />
            <path d="M24 36 l7 -3 l1.5 3.5 l-7 3 Z" {...stylex.attrs(styles.patch)} />
            <path d="M27.5 35.5 l1 2.2" {...stylex.attrs(styles.patchLine)} />
        </svg>
    );
}

/** Draw a credit card floating on its edge. */
function Card() {
    return (
        <svg width="46" height="32" viewBox="0 0 46 32" {...stylex.attrs(styles.art)}>
            <g transform="rotate(-10 23 16)">
                <rect
                    x="3"
                    y="4"
                    width="40"
                    height="25"
                    rx="3.5"
                    {...stylex.attrs(styles.outline, styles.plastic)}
                />
                <rect x="8" y="10" width="8" height="6" rx="1.2" {...stylex.attrs(styles.chip)} />
                <path d="M8 22 H14 M17 22 H23 M26 22 H32" {...stylex.attrs(styles.digits)} />
                <path
                    d="M33 9.5 Q35.5 12 33 14.5 M36 8 Q39.5 12 36 16"
                    {...stylex.attrs(styles.digits)}
                />
            </g>
        </svg>
    );
}

/** Draw a padlocked treasure chest, barely afloat. */
function Chest() {
    return (
        <svg width="46" height="38" viewBox="0 0 46 38" {...stylex.attrs(styles.art)}>
            <path d="M4 17 Q4 5 23 5 Q42 5 42 17 Z" {...stylex.attrs(styles.outline, styles.lid)} />
            <path d="M4 17 H42 V35 H4 Z" {...stylex.attrs(styles.outline, styles.timber)} />
            <path d="M11 6.5 V35 M35 6.5 V35" {...stylex.attrs(styles.band)} />
            <path
                d="M19.5 17.5 Q19.5 12.5 23 12.5 Q26.5 12.5 26.5 17.5"
                {...stylex.attrs(styles.outline)}
            />
            <rect
                x="18"
                y="17"
                width="10"
                height="8.5"
                rx="1.2"
                {...stylex.attrs(styles.outline, styles.gold)}
            />
            <path d="M23 20 V22.5" {...stylex.attrs(styles.keyhole)} />
        </svg>
    );
}

/** Draw a corked bottle with a rolled-up message inside. */
function Bottle() {
    return (
        <svg width="52" height="22" viewBox="0 0 52 22" {...stylex.attrs(styles.art)}>
            <path
                d="M3 11 Q3 3 10 3 H32 Q36 3 38.5 7.5 H44 V14.5 H38.5 Q36 19 32 19 H10 Q3 19 3 11 Z"
                {...stylex.attrs(styles.outline, styles.glass)}
            />
            <rect
                x="44"
                y="7.8"
                width="5.5"
                height="6.4"
                rx="1"
                {...stylex.attrs(styles.outline, styles.cork)}
            />
            <rect x="9" y="7" width="22" height="8" rx="3.5" {...stylex.attrs(styles.scroll)} />
            <path d="M12 10 H26 M12 12.5 H22" {...stylex.attrs(styles.scrollLine)} />
            <path d="M7 6.5 Q10 5 14 5" {...stylex.attrs(styles.glint)} />
        </svg>
    );
}

/** Draw a red and white warning buoy with a lamp on top. */
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

/** Draw a yellow rubber duck. */
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

/** The rise of a bubble from a sinking piece. */
const rise = stylex.keyframes({
    "0%": { opacity: 0, transform: "translateY(0)" },
    "20%": { opacity: 0.9 },
    "100%": { opacity: 0, transform: "translateY(-44px)" },
});

/** The flotsam styles. */
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
        zIndex: 2,
        "@media (prefers-reduced-motion: reduce)": { display: "none" },
    },
    adrift: {
        opacity: 1,
    },
    piece: {
        cursor: "grab",
        left: 0,
        pointerEvents: "auto",
        position: "absolute",
        top: 0,
        touchAction: "none",
        transformOrigin: "50% 100%",
        willChange: "transform",
        ":active": { cursor: "grabbing" },
    },
    bubble: {
        animationDuration: "1.2s",
        animationFillMode: "both",
        animationName: rise,
        animationTimingFunction: "ease-out",
        borderColor: "#ffffff",
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        pointerEvents: "none",
        position: "absolute",
    },
    art: {
        display: "block",
        overflow: "visible",
    },
    outline: {
        fill: "none",
        stroke: tokens.signalInk,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.5,
    },
    vinyl: {
        fill: "#fdfbf7",
    },
    pink: {
        fill: "#f7a8c8",
    },
    mane: {
        fill: "none",
        stroke: "#f7a8c8",
        strokeLinecap: "round",
        strokeWidth: 3.2,
    },
    gold: {
        fill: "#f4c542",
    },
    lash: {
        fill: "none",
        stroke: tokens.signalInk,
        strokeLinecap: "round",
        strokeWidth: 1.2,
    },
    seam: {
        fill: "none",
        stroke: "#e9dfe6",
        strokeLinecap: "round",
        strokeWidth: 2.5,
    },
    patch: {
        fill: "#e7c7a0",
        stroke: tokens.signalInk,
        strokeWidth: 1,
    },
    patchLine: {
        stroke: tokens.signalInk,
        strokeWidth: 0.8,
    },
    plastic: {
        fill: "#2e5a7a",
    },
    chip: {
        fill: "#f4c542",
        stroke: tokens.signalInk,
        strokeWidth: 1,
    },
    digits: {
        fill: "none",
        stroke: tokens.cream,
        strokeLinecap: "round",
        strokeWidth: 1.5,
    },
    lid: {
        fill: "#b07a45",
    },
    timber: {
        fill: "#9a6636",
    },
    band: {
        opacity: 0.55,
        stroke: tokens.signalInk,
        strokeWidth: 1.5,
    },
    keyhole: {
        stroke: tokens.signalInk,
        strokeLinecap: "round",
        strokeWidth: 1.3,
    },
    glass: {
        fill: "#cfe8dc",
        fillOpacity: 0.85,
    },
    cork: {
        fill: "#c89b66",
    },
    scroll: {
        fill: tokens.cream,
        stroke: tokens.signalInk,
        strokeWidth: 1,
    },
    scrollLine: {
        opacity: 0.6,
        stroke: tokens.signalInk,
        strokeWidth: 0.8,
    },
    glint: {
        fill: "none",
        stroke: "#ffffff",
        strokeLinecap: "round",
        strokeWidth: 1.3,
    },
    wire: {
        fill: "none",
        height: "1px",
        left: 0,
        overflow: "visible",
        position: "absolute",
        stroke: tokens.signalInk,
        strokeLinecap: "round",
        strokeWidth: 0.9,
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
        borderRadius: "2px 5px 5px 2px",
        boxShadow: "0 1px 0 rgb(0 0 0 / 12%)",
        left: 0,
        paddingBlock: "0.0625rem",
        paddingInlineEnd: "0.875rem",
        paddingInlineStart: "0.375rem",
        position: "absolute",
        textTransform: "uppercase",
        top: 0,
        transformOrigin: "calc(100% - 5px) 50%",
        whiteSpace: "nowrap",
        willChange: "transform",
        "::before": {
            borderColor: tokens.signalInk,
            borderRadius: "50%",
            borderStyle: "solid",
            borderWidth: "1.25px",
            content: "''",
            height: "4px",
            position: "absolute",
            right: "3px",
            top: "calc(50% - 3.25px)",
            width: "4px",
        },
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
