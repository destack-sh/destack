import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import type { Motion, Transition } from "@destack/style";
import { createSignal } from "@destack/view";
import { MotionField } from "./field.tsx";
import { KeyframeTrack } from "./track.tsx";

/** A quick spring with a little bounce, which every part of the card moves with. */
const SPRING: Transition = {
    duration: { milliseconds: 200 },
    delay: { milliseconds: 0 },
    timingFunction: { kind: "spring", duration: { milliseconds: 300 }, bounce: 0.2 },
};

/** A card that lifts on hover, fades in as it enters, and pulses on a scroll. */
const CARD: Motion = {
    transition: SPRING,
    states: [
        {
            state: "hover",
            values: [
                {
                    property: "transform",
                    value: {
                        x: { value: 0, unit: "px" },
                        y: { value: -4, unit: "px" },
                        rotate: 0,
                        scaleX: 1.02,
                        scaleY: 1.02,
                    },
                },
            ],
            transition: SPRING,
        },
    ],
    enter: { values: [{ property: "opacity", value: 0 }], transition: SPRING },
    exit: { values: [{ property: "opacity", value: 0 }], transition: SPRING },
    animations: [
        {
            tracks: [
                {
                    property: "opacity",
                    keyframes: [
                        { offset: 0, value: 0.4, easing: [0.25, 0.1, 0.25, 1] },
                        { offset: 1, value: 1, easing: [0.25, 0.1, 0.25, 1] },
                    ],
                },
            ],
            duration: { milliseconds: 1000 },
            delay: { milliseconds: 0 },
            iterations: 1,
            direction: "normal",
            fill: "both",
            timeline: {
                kind: "view",
                axis: "block",
                start: { range: "entry", percent: 0 },
                end: { range: "cover", percent: 50 },
            },
        },
    ],
    stagger: { delay: { milliseconds: 0 }, from: "first" },
    viewTransition: "card",
};

/** A card's motion: its stage, transition, hover, entry, exit and a scroll-driven fade. */
export const motionField = defineExample({
    of: MotionField,
    name: "field",
    description: t`A card's motion: its stage, transition, hover, entry, exit and a scroll-driven fade.`,
    render: (properties) => {
        const [motion, setMotion] = createSignal<Motion>(CARD);

        return (
            <MotionField
                {...properties}
                label="Motion"
                value={motion()}
                onValueChange={setMotion}
            />
        );
    },
});

/** The keyframes of an opacity track along its time, the middle one selected. */
export const motionKeyframeTrack = defineExample({
    of: KeyframeTrack,
    name: "keyframeTrack",
    description: t`The keyframes of an opacity track along its time, the middle one selected.`,
    render: () => {
        const [offsets, setOffsets] = createSignal<readonly number[]>([0, 0.4, 1]);
        const [selected, setSelected] = createSignal(1);

        return (
            <KeyframeTrack
                label="Opacity"
                offsets={offsets()}
                selected={selected()}
                disabled={false}
                onSelect={setSelected}
                onMove={(index, offset) => setOffsets((current) => current.with(index, offset))}
                onRemove={(index) => setOffsets((current) => current.toSpliced(index, 1))}
            />
        );
    },
});
