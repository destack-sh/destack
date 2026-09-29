import * as stylex from "@destack/style";
import { onSettled } from "@destack/view";

import { sound } from "../effect/sound";
import { tokens } from "../style/tokens.stylex";

/** How far the dot stretches its band before it resists fully, in ems of the wordmark. */
const reach = 1.4;
/** The band's pull back toward the gap, per second squared per pixel. */
const stiffness = 320;
/** The band's damping, per second, low enough for a few wobbles home. */
const damping = 11;
/** The speed the dot springs up at on a tap, in ems per second. */
const hop = 9;
/** The most the dot stretches along its motion, as a fraction of its width. */
const longestStretch = 0.45;
/** The pointer speed that stretches the dot fully, in pixels per second. */
const fullStretchSpeed = 1800;
/** The distance and speed under which the dot counts as home, in pixels and pixels per second. */
const rest = 0.3;
/** The longest press that still counts as a tap, in milliseconds. */
const tapTime = 250;
/** The farthest a press can travel and still count as a tap, in pixels. */
const tapTravel = 4;

/** Set the wordmark's dictionary dot, which the reader can pull out on a band and let spring home. */
export function Dot() {
    let handle!: HTMLSpanElement;

    // pull the dot on a band while held, and spring it home with a wobble once let go
    onSettled(() => {
        // hold the dot's offset from its gap, its speed, and the press holding it
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        const motion = { x: 0, y: 0, speedX: 0, speedY: 0 };
        let press: { x: number; y: number; at: number; travel: number } | undefined;
        let frame = 0;
        let last = 0;

        // draw the offset, stretching the dot along its motion
        const draw = () => {
            // measure the stretch and its direction
            const speed = Math.hypot(motion.speedX, motion.speedY);
            const stretch = 1 + longestStretch * Math.min(1, speed / fullStretchSpeed);
            const angle = Math.atan2(motion.speedY, motion.speedX);

            // move and stretch the dot
            handle.style.translate = `${motion.x.toFixed(2)}px ${motion.y.toFixed(2)}px`;
            handle.style.transform = `rotate(${angle}rad) scale(${stretch.toFixed(3)}, ${(1 / stretch).toFixed(3)}) rotate(${-angle}rad)`;
        };

        // step the band toward the gap until the dot rests there
        const spring = (now: number) => {
            // integrate the pull and the damping over the frame
            const step = Math.min(0.032, (now - last) / 1000);
            last = now;
            motion.speedX += (-stiffness * motion.x - damping * motion.speedX) * step;
            motion.speedY += (-stiffness * motion.y - damping * motion.speedY) * step;
            motion.x += motion.speedX * step;
            motion.y += motion.speedY * step;

            // settle home, or keep springing
            const isHome =
                Math.hypot(motion.x, motion.y) < rest &&
                Math.hypot(motion.speedX, motion.speedY) < rest * 10;
            if (isHome) {
                Object.assign(motion, { x: 0, y: 0, speedX: 0, speedY: 0 });
                frame = 0;
            } else {
                frame = requestAnimationFrame(spring);
            }
            draw();
        };

        // let the band go, or snap straight home when the reader prefers less motion
        const letGo = () => {
            // stop any spring still running, and snap home when still
            cancelAnimationFrame(frame);
            if (isStill) {
                Object.assign(motion, { x: 0, y: 0, speedX: 0, speedY: 0 });
                draw();

                return;
            }
            // spring home from where it was let go
            last = performance.now();
            frame = requestAnimationFrame(spring);
        };

        // pick the dot up where it was grabbed
        const grab = (event: PointerEvent) => {
            // hold the pointer and stop any spring
            event.preventDefault();
            handle.setPointerCapture(event.pointerId);
            cancelAnimationFrame(frame);

            // remember where the press started, from the dot's current offset
            press = {
                x: event.clientX - motion.x,
                y: event.clientY - motion.y,
                at: performance.now(),
                travel: 0,
            };
            sound.play("lift");
        };

        // pull the band toward the pointer, resisting more the farther it stretches
        const pull = (event: PointerEvent) => {
            if (!press) {
                return;
            }

            // ease the pointer's distance into the band's reach
            const em = parseFloat(getComputedStyle(handle).fontSize);
            const deltaX = event.clientX - press.x;
            const deltaY = event.clientY - press.y;
            const distance = Math.hypot(deltaX, deltaY);
            const band = distance === 0 ? 0 : (reach * em) / (distance + reach * em);
            press.travel = Math.max(press.travel, distance);

            // follow, measuring the speed for the stretch and the release
            const now = performance.now();
            const step = Math.max(1, now - last) / 1000;
            last = now;
            const x = deltaX * band;
            const y = deltaY * band;
            motion.speedX = (x - motion.x) / step;
            motion.speedY = (y - motion.y) / step;
            motion.x = x;
            motion.y = y;
            draw();
        };

        // let go, hopping on a tap and springing home otherwise
        const release = () => {
            if (!press) {
                return;
            }
            const isTap = performance.now() - press.at < tapTime && press.travel < tapTravel;
            press = undefined;

            // hop up on a tap
            if (isTap) {
                const em = parseFloat(getComputedStyle(handle).fontSize);
                motion.speedY = -hop * em;
            }
            sound.play("set");
            letGo();
        };

        // listen on the dot itself
        handle.addEventListener("pointerdown", grab);
        handle.addEventListener("pointermove", pull);
        handle.addEventListener("pointerup", release);
        handle.addEventListener("pointercancel", release);

        return () => cancelAnimationFrame(frame);
    });

    return (
        <span ref={handle} aria-hidden="true" {...stylex.attrs(styles.handle)}>
            <span {...stylex.attrs(styles.dot)} />
        </span>
    );
}

/** The dot warming now and then, like the one near star among the letters' far ones. */
const glow = stylex.keyframes({
    "0%, 60%, 100%": { boxShadow: "0 0 0 0 transparent" },
    "80%": { boxShadow: `0 0 0.08em 0 color-mix(in srgb, ${tokens.signal} 35%, transparent)` },
});

/** The dot styles. */
const styles = stylex.create({
    handle: {
        cursor: { default: "grab", ":active": "grabbing" },
        "--swell": { default: "1", ":hover": "1.35", ":active": "1.35" },
        display: "block",
        padding: "0.08em",
        position: "relative",
        touchAction: "none",
        zIndex: 1,
    },
    dot: {
        animationDuration: "9s",
        animationIterationCount: "infinite",
        animationName: glow,
        animationTimingFunction: "ease-in-out",
        backgroundColor: tokens.signal,
        borderColor: tokens.cream,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1.5px",
        boxSizing: "border-box",
        display: "block",
        height: "0.12em",
        scale: "var(--swell)",
        transition: "scale 220ms cubic-bezier(0.3, 1.6, 0.5, 1)",
        translate: "0 -0.05em",
        width: "0.12em",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none", transition: "none" },
    },
});
