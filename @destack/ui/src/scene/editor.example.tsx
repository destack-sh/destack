import { t } from "@destack/locale";
import pauseIcon from "@destack/icon/phosphor/pause";
import playIcon from "@destack/icon/phosphor/play";
import speakerHighIcon from "@destack/icon/phosphor/speaker-high";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { createSignal, For, type JSX, onCleanup, untrack, useEnvironment } from "@destack/view";
import { Button } from "../button/index.ts";
import { Cluster } from "../cluster/index.ts";
import { PICTURES } from "../example/picture.ts";
import { Icon } from "../icon/index.ts";
import { Stack } from "../stack/index.ts";
import {
    Timeline,
    TimelineAxis,
    TimelineBar,
    TimelineBarHandle,
    TimelineBarLabel,
    TimelineBarList,
    TimelineBody,
    TimelineGrid,
    TimelineHeader,
    type TimelineItem,
    TimelineMarker,
    TimelineRange,
    TimelineRow,
    TimelineRowLabel,
    TimelineRowList,
    type TimelineScale,
    TimelineTrack,
} from "../timeline/index.ts";
import { Toggle } from "../toggle/index.ts";

/** The pixels a mark's label takes. */
const LABEL_WIDTH = 72;

/** The frames of a second of the film. */
const RATE = 25;

/** The frames of the film. */
const FILM = 40 * RATE;

/** The steps between the marks of a film, in frames. */
const FRAME_STEPS = [1, 5, RATE, 2 * RATE, 5 * RATE, 10 * RATE, 30 * RATE, 60 * RATE] as const;

/** The parts of a beat notes land on. */
const PARTS = 4;

/** The parts of a bar of four beats. */
const BAR = 4 * PARTS;

/** The parts of the song. */
const SONG = 4 * BAR;

/** The styles of the editors. */
const styles = style.create({
    clip: {
        outlineWidth: stroke.border,
        outlineStyle: "solid",
        outlineColor: color.background,
        outlineOffset: `calc(-1 * ${stroke.border})`,
    },
    first: { insetInlineStart: 0 },
    last: { insetInlineEnd: 0 },
    strip: {
        position: "absolute",
        inset: 0,
        display: "flex",
        overflow: "hidden",
        borderRadius: "inherit",
        pointerEvents: "none",
    },
    frame: (picture: string) => ({
        flex: 1,
        backgroundImage: `url("${picture}")`,
        backgroundSize: "auto 100%",
        backgroundRepeat: "repeat-x",
        opacity: 0.7,
    }),
    wave: {
        position: "absolute",
        insetInline: 0,
        insetBlockEnd: 0,
        width: "100%",
        height: "70%",
        pointerEvents: "none",
        color: swatch.infoMark,
    },
    name: {
        insetBlockEnd: "auto",
    },
    header: {
        justifyContent: "space-between",
    },
    playhead: {
        width: stroke.ring,
        backgroundColor: swatch.destructiveMark,
    },
    loop: {
        opacity: 0.35,
        color: color.foreground,
    },
    black: {
        backgroundColor: color.muted,
    },
    time: {
        fontVariantNumeric: "tabular-nums",
    },
    curve: {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        color: swatch.accentMark,
        paddingBlock: space[1],
    },
});

/** Write a frame as its minute, second and frame. */
function timecode(frame: number): string {
    const whole = Math.round(frame);
    const parts = [
        Math.floor(whole / (60 * RATE)),
        Math.floor(whole / RATE) % 60,
        ((whole % RATE) + RATE) % RATE,
    ];

    return parts.map((part) => String(part).padStart(2, "0")).join(":");
}

/** List the marks of a range at every step of one length. */
function marksOf(range: { readonly start: number; readonly end: number }, step: number) {
    const first = Math.ceil(range.start / step);
    const count = Math.max(0, Math.floor(range.end / step) - first + 1);

    return Array.from({ length: count }, (_, index) => ({ at: (first + index) * step }));
}

/** The frames of a film, marked by minute, second and frame. */
const FRAMES: TimelineScale = {
    ticks: ({ range, width }) => {
        const target = ((range.end - range.start) / width) * LABEL_WIDTH;

        return marksOf(range, FRAME_STEPS.find((step) => step >= target) ?? 300 * RATE);
    },
    label: timecode,
    snap: Math.round,
    step: (at, direction) => Math.round(at) + direction,
};

/** The bars and beats of a song, notes landing on a quarter of a beat. */
const BEATS: TimelineScale = {
    ticks: ({ range, width }) =>
        marksOf(range, ((range.end - range.start) / width) * LABEL_WIDTH > PARTS ? BAR : PARTS),
    label: (at) =>
        `${String(Math.floor(at / BAR) + 1)}.${String((Math.floor(at / PARTS) % 4) + 1)}`,
    snap: Math.round,
    step: (at, direction) => Math.round(at) + direction,
};

/** The tracks and clips of the film. */
const CLIPS: readonly TimelineItem[] = [
    { key: "titles", label: "Titles" },
    { key: "film", label: "Film" },
    { key: "music", label: "Music" },
    { key: "title-1", label: "Lisbon", row: "titles", start: 50, end: 175, color: "purple" },
    { key: "clip-1", label: "Roofs", row: "film", start: 0, end: 300, color: "cyan" },
    { key: "clip-2", label: "Harbour", row: "film", start: 275, end: 620, color: "cyan" },
    { key: "clip-3", label: "Evening", row: "film", start: 620, end: 1000, color: "cyan" },
    { key: "song", label: "Guitar", row: "music", start: 0, end: 900, color: "green" },
];

/** The longest each clip runs, the frames its media has. */
const MEDIA: Readonly<Record<string, number>> = {
    "clip-1": 340,
    "clip-2": 400,
    "clip-3": 420,
    song: 950,
};

/** Hold a clip to its media and the film's start, moving its end with its start. */
function held(clip: TimelineItem): TimelineItem {
    // start no earlier than the film
    const given = clip.start ?? 0;
    const start = Math.max(0, given);
    const end = (clip.end ?? start) + start - given;

    return { ...clip, start, end: Math.min(end, start + (MEDIA[clip.key] ?? Infinity)) };
}

/** Replace the changed items of a list. */
function replaced(
    items: readonly TimelineItem[],
    changed: readonly TimelineItem[],
): readonly TimelineItem[] {
    const changes = new Map(changed.map((item) => [item.key, item]));

    return items.map((item) => changes.get(item.key) ?? item);
}

/** Play a place along a range by the clock's frames, over and over. */
function createPlayback(
    rate: number,
    range: () => { readonly start: number; readonly end: number },
) {
    // move by the time since the last frame
    const { clock } = useEnvironment();
    const [at, setAt] = createSignal(untrack(range).start, { ownedWrite: true });
    const [isPlaying, setPlaying] = createSignal(false, { ownedWrite: true });
    let timer: number | undefined;
    const stop = (): void => {
        clock.clearInterval(timer);
        timer = undefined;
        setPlaying(false);
    };
    const play = (): void => {
        let last = clock.now();
        setPlaying(true);
        timer = clock.setInterval(() => {
            // start over at the range's end
            const now = clock.now();
            const travel = ((now - last) / 1_000) * rate;
            last = now;
            setAt((current) => {
                const { start, end } = range();

                return current + travel >= end ? start : current + travel;
            });
        }, 16);
    };
    onCleanup(() => clock.clearInterval(timer));

    return { at, setAt, isPlaying, toggle: () => (timer === undefined ? play() : stop()) };
}

/** Draw the sound of a clip. */
function Wave(): JSX.Element {
    const bars = Array.from({ length: 90 }, (_, index) =>
        Math.abs(Math.sin(index * 0.7) * Math.cos(index * 0.23)),
    );

    return (
        <svg
            viewBox="0 0 90 20"
            preserveAspectRatio="none"
            aria-hidden="true"
            {...style.attrs(styles.wave)}
        >
            <path
                d={bars
                    .map((bar, index) => `M${String(index + 0.5)} 20v-${String(2 + bar * 17)}`)
                    .join("")}
                stroke="currentColor"
                stroke-width="0.6"
            />
        </svg>
    );
}

/** A film cut from clips: pictures and sound in the clips, a playhead and a loop. */
export const sceneClips = defineExample({
    of: Timeline,
    name: "clips",
    description: t`A film cut from clips, with pictures, sound, a playhead and a loop.`,
    layout: "padded",
    render: () => {
        // keep the clips and the loop, and play along it
        const [clips, setClips] = createSignal(CLIPS);
        const [loop, setLoop] = createSignal({ start: 250, end: 500 });
        const playback = createPlayback(RATE, loop);

        return (
            <Stack space="3">
                <Cluster space="2" align="center">
                    <Button
                        isIconOnly
                        variant="outline"

                        aria-label={playback.isPlaying() ? "Pause" : "Play"}
                        onClick={playback.toggle}
                    >
                        <Icon icon={playback.isPlaying() ? pauseIcon : playIcon} />
                    </Button>
                    <span {...style.attrs(text.sm, styles.time)}>{timecode(playback.at())}</span>
                </Cluster>
                <Timeline
                    axis={FRAMES}
                    items={clips()}
                    overlap="cover"
                    rowHeight={(node, standard) =>
                        node.item.key === "titles" ? standard : 2 * standard
                    }
                    defaultWindow={{ start: 0, end: FILM }}
                    propagate={(changed) => changed.map(held)}
                    onItemsChange={(changed) => setClips((all) => replaced(all, changed))}
                >
                    <TimelineHeader corner="Track">
                        <TimelineAxis>
                            <TimelineRange
                                aria-label="Loop"
                                color="yellow"
                                start={loop().start}
                                end={loop().end}
                                onRangeChange={setLoop}
                                xstyle={styles.loop}
                            />
                            <TimelineMarker
                                aria-label="Playhead on the ruler"
                                at={playback.at()}
                                onAtChange={playback.setAt}
                                xstyle={styles.playhead}
                            />
                        </TimelineAxis>
                    </TimelineHeader>
                    <TimelineBody>
                        <TimelineGrid />
                        <TimelineRange
                            color="yellow"
                            start={loop().start}
                            end={loop().end}
                            xstyle={styles.loop}
                        />
                        <TimelineRowList>
                            {(node) => (
                                <TimelineRow>
                                    <TimelineRowLabel xstyle={styles.header}>
                                        {node().item.label}
                                        <Toggle size="sm" aria-label={`Mute ${node().item.label}`}>
                                            <Icon icon={speakerHighIcon} />
                                        </Toggle>
                                    </TimelineRowLabel>
                                    <TimelineTrack>
                                        <TimelineBarList>
                                            {() => <Clip track={node().item.key} />}
                                        </TimelineBarList>
                                    </TimelineTrack>
                                </TimelineRow>
                            )}
                        </TimelineRowList>
                        <TimelineMarker
                            aria-label="Playhead"
                            at={playback.at()}
                            onAtChange={playback.setAt}
                            xstyle={styles.playhead}
                        />
                    </TimelineBody>
                </Timeline>
            </Stack>
        );
    },
});

/** Render a clip: its pictures on a film's track, its sound on a song's. */
function Clip(properties: { readonly track: string }): JSX.Element {
    return (
        <TimelineBar xstyle={styles.clip}>
            {properties.track === "film" ? (
                <span {...style.attrs(styles.strip)}>
                    <span {...style.attrs(styles.frame(PICTURES.alfama))} />
                </span>
            ) : properties.track === "music" ? (
                <Wave />
            ) : null}
            <TimelineBarLabel xstyle={styles.name} />
            <TimelineBarHandle edge="start" xstyle={styles.first} />
            <TimelineBarHandle edge="end" xstyle={styles.last} />
        </TimelineBar>
    );
}

/** The keys of the piano roll, from the highest. */
const KEYS = ["C5", "B4", "A#4", "A4", "G#4", "G4", "F#4", "F4", "E4", "D#4", "D4", "C#4", "C4"];

/** The notes of the song: a key, a start and a length in parts of a beat. */
const NOTES = [
    ["C4", 0, 4],
    ["E4", 4, 4],
    ["G4", 8, 4],
    ["C5", 12, 8],
    ["A4", 20, 4],
    ["F4", 24, 4],
    ["G4", 28, 8],
    ["E4", 36, 4],
    ["D4", 40, 8],
    ["C4", 48, 16],
] as const;

/** A song's notes on the keys of a piano, with a volume curve under them. */
export const sceneNotes = defineExample({
    of: Timeline,
    name: "notes",
    description: t`A song's notes on a piano's keys in bars and beats, over a volume curve.`,
    layout: "padded",
    render: () => {
        // keep the notes, and play along the song
        const [notes, setNotes] = createSignal<readonly TimelineItem[]>([
            ...KEYS.map((key) => ({ key, label: key })),
            { key: "volume", label: "Volume" },
            ...NOTES.map(([row, start, length], index) => ({
                key: `note-${String(index)}`,
                label: row,
                row,
                start,
                end: start + length,
                color: "purple" as const,
                variant: "solid" as const,
            })),
        ]);
        const playback = createPlayback(2 * PARTS, () => ({ start: 0, end: SONG }));

        return (
            <Timeline
                axis={BEATS}
                items={notes()}
                overlap="cover"
                labelWidth="6rem"
                rowHeight={(node, standard) =>
                    node.item.key === "volume" ? 2 * standard : 0.7 * standard
                }
                defaultWindow={{ start: 0, end: SONG }}
                onItemsChange={(changed) => setNotes((all) => replaced(all, changed))}
            >
                <TimelineHeader corner="Key" />
                <TimelineBody>
                    <TimelineGrid />
                    <TimelineRowList>
                        {(node) => (
                            <TimelineRow xstyle={node().item.label.includes("#") && styles.black}>
                                <TimelineRowLabel />
                                <TimelineTrack>
                                    {node().item.key === "volume" ? (
                                        <Curve />
                                    ) : (
                                        <TimelineBarList>
                                            {() => (
                                                <TimelineBar>
                                                    <TimelineBarHandle edge="start" />
                                                    <TimelineBarHandle edge="end" />
                                                </TimelineBar>
                                            )}
                                        </TimelineBarList>
                                    )}
                                </TimelineTrack>
                            </TimelineRow>
                        )}
                    </TimelineRowList>
                    <TimelineMarker
                        aria-label="Playhead"
                        at={playback.at()}
                        onAtChange={playback.setAt}
                        xstyle={styles.playhead}
                    />
                </TimelineBody>
            </Timeline>
        );
    },
});

/** Draw the volume along the song. */
function Curve(): JSX.Element {
    return (
        <svg
            viewBox="0 0 100 20"
            preserveAspectRatio="none"
            aria-hidden="true"
            {...style.attrs(styles.curve)}
        >
            <For each={["M0 16 C20 2 35 4 50 10 S80 18 100 4"]}>
                {(path) => (
                    <path
                        d={path}
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        vector-effect="non-scaling-stroke"
                    />
                )}
            </For>
        </svg>
    );
}
