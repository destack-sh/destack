import { color } from "@destack/theme/tokens.stylex";
import { createSignal, onSettled, Show } from "@destack/view";
import * as stylex from "@destack/style";

import { sound } from "../effect/sound";

/** Turn the site's quiet sounds on or off, off until the reader chooses. */
export function SoundToggle() {
    const [isOn, setIsOn] = createSignal(false);

    // restore the reader's choice from the last visit
    onSettled(() => {
        setIsOn(sound.restore());
    });

    return (
        <button
            aria-label={isOn() ? "Turn sound off" : "Turn sound on"}
            aria-pressed={isOn() ? "true" : "false"}
            title={isOn() ? "Turn sound off" : "Turn sound on"}
            onClick={() => setIsOn(sound.toggle())}
            type="button"
            {...stylex.attrs(styles.toggle)}
        >
            <svg
                aria-hidden="true"
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4Z" />
                <Show when={isOn()} fallback={<path d="M16 9.5l5 5m0-5l-5 5" />}>
                    <path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11" />
                </Show>
            </svg>
        </button>
    );
}

/** The sound toggle styles. */
const styles = stylex.create({
    toggle: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        display: "inline-flex",
        height: "2.75rem",
        justifyContent: "center",
        padding: 0,
        width: "2.75rem",
        ":hover": { color: color.primary },
    },
});
