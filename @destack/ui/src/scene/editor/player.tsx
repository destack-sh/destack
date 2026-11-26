import { type Localization, t } from "@destack/locale";
import type { SetStep } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, swatch } from "@destack/theme/tokens.stylex";
import { type CheckResult, For, type JSX, Show, useLocale } from "@destack/view";
import type { Locator, Step } from "@destack/view/scenario";
import { Icon } from "@destack/ui/icon";
import { Button } from "../../button/index.ts";
import { ButtonGroup } from "../../button-group/index.ts";

/** The styles of the player and its steps. */
const styles = style.create({
    player: {
        display: "grid",
        gap: space[3],
    },
    steps: {
        display: "grid",
        gap: space[1],
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
    step: {
        display: "grid",
        gridTemplateColumns: "1.25rem minmax(0, 1fr)",
        alignItems: "start",
        gap: space[2],
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[3],
    },
    next: {
        backgroundColor: color.selected,
    },
    number: {
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    check: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        color: color.mutedForeground,
    },
    passed: {
        color: color.foreground,
    },
    failed: {
        color: swatch.destructiveText,
    },
});

/** Play a scenario's steps one at a time or all, showing the results of each step's checks. */
export function Player(properties: {
    /** The steps. */
    readonly steps: readonly (SetStep | Step)[];
    /** The results of each step's checks, for the steps run so far, in order. */
    readonly results: readonly (readonly CheckResult[])[];
    /** Handle a person asking for the next step. */
    readonly onStep: () => void;
    /** Handle a person asking for every step left. */
    readonly onPlay: () => void;
    /** Handle a person starting over. */
    readonly onReset: () => void;
}): JSX.Element {
    // write the strings in the person's language
    const localization = useLocale();

    // point at the next step, done past the last
    const next = () => properties.results.length;
    const isDone = () => next() >= properties.steps.length;

    return (
        <div {...style.attrs(styles.player)}>
            {/* Controls */}
            <ButtonGroup aria-label={localization.render(t.context("title")`Playback`)}>
                <Button size="sm" variant="outline" disabled={isDone()} onClick={properties.onPlay}>
                    <Icon name="play" />
                    {localization.render(t.context("action")`Play`)}
                </Button>
                <Button size="sm" variant="outline" disabled={isDone()} onClick={properties.onStep}>
                    <Icon name="skip-forward" />
                    {localization.render(t.context("action")`Step`)}
                </Button>
                <Button size="sm" variant="outline" onClick={properties.onReset}>
                    <Icon name="arrow-counter-clockwise" />
                    {localization.render(t.context("action")`Reset`)}
                </Button>
            </ButtonGroup>

            {/* Steps */}
            <ol
                aria-label={localization.render(t.context("title")`Steps`)}
                {...style.attrs(styles.steps)}
            >
                <For each={properties.steps}>
                    {(step, index) => (
                        <li
                            aria-current={index() === next() ? "step" : undefined}
                            {...style.attrs(
                                text.sm,
                                styles.step,
                                index() === next() && styles.next,
                            )}
                        >
                            <span {...style.attrs(styles.number)}>{index() + 1}</span>
                            <div>
                                <div>{wordsOfStep(step, localization)}</div>
                                <Show when={properties.results[index()]}>
                                    {(results) => (
                                        <For each={results()}>
                                            {(check) => (
                                                <div
                                                    {...style.attrs(
                                                        text.xs,
                                                        styles.check,
                                                        check.isPassed
                                                            ? styles.passed
                                                            : styles.failed,
                                                    )}
                                                >
                                                    <Show
                                                        when={check.isPassed}
                                                        fallback={
                                                            <Icon
                                                                name="x"
                                                                label={localization.render(
                                                                    t.context("option")`Failed`,
                                                                )}
                                                            />
                                                        }
                                                    >
                                                        <Icon
                                                            name="check"
                                                            label={localization.render(
                                                                t.context("option")`Passed`,
                                                            )}
                                                        />
                                                    </Show>
                                                    {JSON.stringify(check.seen)}
                                                </div>
                                            )}
                                        </For>
                                    )}
                                </Show>
                            </div>
                        </li>
                    )}
                </For>
            </ol>
        </div>
    );
}

/** Write a step as a person reads it, such as `Press ArrowRight on tab “Edit”`. */
function wordsOfStep(step: SetStep | Step, locale: Localization): string {
    switch (step.action) {
        case "focus":
            return locale.render(t.context("action")`Focus ${targetWords(step.target)}`);
        case "click":
            return step.button === "secondary"
                ? locale.render(t.context("action")`Right-click ${targetWords(step.target)}`)
                : locale.render(t.context("action")`Click ${targetWords(step.target)}`);
        case "pointerMove":
            return locale.render(t`Point at ${targetWords(step.target)}`);
        case "pointerDown":
            return locale.render(t`Press the pointer down`);
        case "pointerUp":
            return locale.render(t`Release the pointer`);
        case "wheel":
            return locale.render(t`Turn the wheel by ${step.deltaX ?? 0}, ${step.deltaY ?? 0}`);
        case "press":
            return step.target === undefined
                ? locale.render(t.context("action")`Press ${step.key}`)
                : locale.render(t`Press ${step.key} on ${targetWords(step.target)}`);
        case "fill":
            return locale.render(t`Fill ${targetWords(step.target)} with “${step.value}”`);
        case "pause":
            return locale.render(t`Wait ${step.milliseconds} ms`);
        case "set":
            return locale.render(t.context("action")`Set ${JSON.stringify(step.properties)}`);
    }
}

/** Write a step's target as a person reads it, such as `tab “Edit”`, by what it matches. */
function targetWords(target: Locator): string {
    if ("role" in target) {
        return target.name === undefined ? target.role : `${target.role} “${target.name}”`;
    } else if ("label" in target) {
        return `“${target.label}”`;
    } else if ("text" in target) {
        return `“${target.text}”`;
    } else if ("testId" in target) {
        return target.testId;
    }

    return target.css;
}
