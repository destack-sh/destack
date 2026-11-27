import { expect, test } from "@destack/test";
import { Spring, TimingFunction } from "./index.ts";

test("move a spring from 0 to 1, past its end where it bounces, and settle within its time", () => {
    const calm: Spring = { kind: "spring", duration: { milliseconds: 500 }, bounce: 0 };
    const bouncy: Spring = { kind: "spring", duration: { milliseconds: 500 }, bounce: 0.5 };
    const settled = Spring.settle(bouncy);
    const samples = Array.from({ length: 200 }, (_, index) =>
        Spring.at(bouncy, (settled * index) / 199),
    );

    expect([
        Spring.at(calm, 0),
        Math.max(...Array.from({ length: 100 }, (_, index) => Spring.at(calm, index / 50))) <= 1,
        Math.max(...samples) > 1,
        Math.abs(1 - Spring.at(bouncy, settled)) <= 0.001,
        Spring.settle(calm) > 0.5 && Spring.settle(calm) < 2,
    ]).toEqual([0, true, true, true, true]);
});

test("write curves, steps and springs as CSS easing functions, a spring timed by its settle time", () => {
    const spring: Spring = { kind: "spring", duration: { milliseconds: 300 }, bounce: 0.2 };
    const written = TimingFunction.css(spring);
    const points = written.slice("linear(".length, -1).split(", ").map(Number);

    expect([
        TimingFunction.css([0.4, 0, 0.2, 1]),
        TimingFunction.css("{motion.easingStandard}"),
        TimingFunction.css({ kind: "steps", count: 4, position: "jump-end" }),
        [points[0], points.at(-1)],
        TimingFunction.duration(spring, { milliseconds: 100 }),
        TimingFunction.duration([0.4, 0, 0.2, 1], { milliseconds: 100 }),
    ]).toEqual([
        "cubic-bezier(0.4, 0, 0.2, 1)",
        "var(--destack-motion-easing-standard)",
        "steps(4, jump-end)",
        [0, 1],
        { milliseconds: Math.round(Spring.settle(spring) * 1000) },
        { milliseconds: 100 },
    ]);
});

test("ease a share of time by a curve, by steps as CSS jumps them, and by a spring", () => {
    const spring: Spring = { kind: "spring", duration: { milliseconds: 400 }, bounce: 0 };

    expect([
        TimingFunction.at([0, 0, 1, 1], 0.3),
        TimingFunction.at([0, 0, 0.58, 1], 0.5) > 0.5,
        TimingFunction.at({ kind: "steps", count: 4, position: "jump-end" }, 0.3),
        TimingFunction.at({ kind: "steps", count: 4, position: "jump-start" }, 0),
        TimingFunction.at({ kind: "steps", count: 2, position: "jump-none" }, 0.6),
        Math.abs(1 - TimingFunction.at(spring, 1)) <= 0.001,
    ]).toEqual([0.3, true, 0.25, 0.25, 1, true]);
});
