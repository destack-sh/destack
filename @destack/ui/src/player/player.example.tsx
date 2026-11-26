import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal, For } from "@destack/view";
import { CAPTIONS, FILM, sound, CAPTIONS_GERMAN } from "../example/media.ts";
import { landscape } from "../example/picture.ts";
import { Cluster } from "../cluster/index.ts";
import { usePlayer } from "./control.ts";
import {
    AudioPlayer,
    Player,
    PlayerAudio,
    PlayerPicture,
    PlayerPlayButton,
    PlayerSeekBar,
    PlayerTime,
    PlayerTitle,
    VideoPlayer,
} from "./player.tsx";

/** The styles of the examples. */
const styles = style.create({
    narrow: {
        width: "100%",
        maxWidth: "40rem",
        marginInline: "auto",
    },
    card: {
        display: "grid",
        gap: space[2],
        width: "16rem",
        marginInline: "auto",
    },
    cover: {
        width: "100%",
        height: "auto",
        aspectRatio: 1,
    },
    list: {
        display: "grid",
        gap: space[3],
        width: "100%",
        maxWidth: "32rem",
        marginInline: "auto",
    },
});

/** A song of an album. */
interface Song {
    /** Its title. */
    readonly title: string;
    /** Its notes, in hertz. */
    readonly notes: readonly number[];
}

/** The songs of an album. */
const SONGS: readonly Song[] = [
    { title: "First Tram", notes: [262, 330, 392, 523, 392, 330] },
    { title: "Over the Pass", notes: [220, 262, 330, 440, 330, 262] },
    { title: "Last Light", notes: [196, 247, 294, 392, 294, 247] },
];

/** A film with captions. */
export const playerFilm = defineExample({
    of: VideoPlayer,
    name: "film",
    description: t`A film with captions.`,
    layout: "padded",
    render: () => (
        <VideoPlayer
            source={FILM}
            title="The pass"
            picture={landscape(5, 16 / 9)}
            length={20}
            captions={[
                { source: CAPTIONS, language: "en", label: "English" },
                { source: CAPTIONS_GERMAN, language: "de", label: "Deutsch" },
            ]}
            xstyle={styles.narrow}
        />
    ),
});

/** A film that fails to play. */
export const playerFailure = defineExample({
    of: VideoPlayer,
    name: "failure",
    description: t`A film that fails to play.`,
    layout: "padded",
    render: () => (
        <VideoPlayer source="data:video/mp4;base64,AAAA" title="The pass" xstyle={styles.narrow} />
    ),
});

/** Voice messages, of which one plays at a time. */
export const playerVoice = defineExample({
    of: AudioPlayer,
    name: "voice",
    description: t`Voice messages, of which one plays at a time.`,
    layout: "padded",
    render: () => (
        <div {...style.attrs(styles.list)}>
            <For each={SONGS}>
                {(song) => <AudioPlayer source={sound(song.notes, 12)} length={12} />}
            </For>
        </div>
    ),
});

/** An album, whose app keeps the list and the song that plays. */
export const playerAlbum = defineExample({
    of: AudioPlayer,
    name: "album",
    description: t`An album, whose app keeps the list and the song that plays.`,
    layout: "padded",
    render: () => {
        // keep the song that plays
        const [at, setAt] = createSignal(0);
        const step = (by: number) => setAt((at() + by + SONGS.length) % SONGS.length);
        const song = () => SONGS[at()] ?? { title: "", notes: [] };

        return (
            <div {...style.attrs(styles.list)}>
                <AudioPlayer
                    source={sound(song().notes, 12)}
                    title={song().title}
                    artist="The Hill"
                    picture={landscape(at() + 2, 1)}
                    onPrevious={() => step(-1)}
                    onNext={() => step(1)}
                    onEnded={() => step(1)}
                />
            </div>
        );
    },
});

/** Tell how much of a sound is left, as a part of its own. */
function Left() {
    const player = usePlayer();

    const left = () => Math.ceil((player.length() ?? 0) - player.time());

    return <small>{left()} s left</small>;
}

/** A sound in a card, from the parts and one of its own. */
export const playerCard = defineExample({
    of: Player,
    name: "card",
    description: t`A sound in a card, from the parts and one of its own.`,
    layout: "padded",
    render: () => (
        <Player
            source={sound([262, 330, 392, 523], 12)}
            title="First Tram"
            artist="The Hill"
            picture={landscape(2, 1)}
            length={12}
            xstyle={styles.card}
        >
            <PlayerAudio />
            <PlayerPicture xstyle={styles.cover} />
            <PlayerTitle />
            <PlayerSeekBar />
            <Cluster>
                <PlayerPlayButton />
                <PlayerTime />
                <Left />
            </Cluster>
        </Player>
    ),
});
