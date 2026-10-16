import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { For } from "@destack/view";

/** The strip every rolling digit slides along, one digit to a line. */
const STRIP = "0\n1\n2\n3\n4\n5\n6\n7\n8\n9";

/** A character that rolls: one decimal digit. */
const DIGIT = /^[0-9]$/u;

/** The milliseconds a digit takes to roll to its new value. */
const ROLL_TIME = 600;

/** The milliseconds a roll waits for the frames that render the change itself. */
const ROLL_DELAY = 60;

/** The milliseconds each digit waits after the one to its right, so a change ripples left. */
const ROLL_STEP = 24;

/** The cubic ease-out every roll lands on, under a line a frame even across nine digits. */
const LANDING = "cubic-bezier(0.33, 1, 0.68, 1)";

/** Draw a figure whose digits roll to their new values, up as it grows and down as it falls. */
export function Digits(properties: { text: string }) {
    // count the characters from the right, so the ones place stays put as the figure grows
    const slots = () => properties.text.split("").toReversed();

    return (
        <span {...style.attrs(styles.digits)}>
            {/* the figure as text, for reading and copying */}
            <span {...style.attrs(styles.text)}>{properties.text}</span>

            {/* each character, a digit rolling along its strip or a mark set still */}
            <span aria-hidden="true" {...style.attrs(styles.slots)}>
                <For each={slots()} keyed={false}>
                    {(character, place) => (
                        <span {...style.attrs(styles.slot)}>
                            {DIGIT.test(character()) ? (
                                <span
                                    style={{
                                        transform: `translateY(-${Number(character()) * 10}%)`,
                                        "transition-delay": `${ROLL_DELAY + place * ROLL_STEP}ms`,
                                    }}
                                    {...style.attrs(styles.strip)}
                                >
                                    {STRIP}
                                </span>
                            ) : (
                                character()
                            )}
                        </span>
                    )}
                </For>
            </span>
        </span>
    );
}

/** The rolling figure styles. */
const styles = style.create({
    digits: {
        display: "inline-grid",
    },
    text: {
        clipPath: "inset(50%)",
        gridArea: "1 / 1",
        height: "1px",
        overflow: "hidden",
        position: "absolute",
        whiteSpace: "nowrap",
        width: "1px",
    },
    slots: {
        display: "inline-flex",
        flexDirection: "row-reverse",
        gridArea: "1 / 1",
        height: "1lh",
        maskImage: "linear-gradient(transparent, black 18%, black 82%, transparent)",
        overflow: "hidden",
        userSelect: "none",
    },
    slot: {
        display: "inline-block",
        textAlign: "center",
        width: "1ch",
    },
    strip: {
        display: "block",
        transitionDuration: { default: `${ROLL_TIME}ms`, [media.motionReduce]: "0ms" },
        transitionProperty: "transform",
        transitionTimingFunction: LANDING,
        whiteSpace: "pre",
    },
});
