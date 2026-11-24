import { type Accessor, createContext, createSignal, type Setter, useContext } from "@destack/view";

/** How far a skip goes, in seconds, as Media Chrome's. */
export const SKIP = 10;

/** What a lock screen or headphones ask for. */
const SESSION_ACTIONS = [
    "play",
    "pause",
    "seekbackward",
    "seekforward",
    "seekto",
    "previoustrack",
    "nexttrack",
] as const;

/** How much louder or softer a key makes the sound. */
const VOLUME_STEP = 0.1;

/** What plays, as a lock screen or headphones show and steer it. */
export interface PlayerSessionOptions {
    /** Its title. */
    readonly title?: string | undefined;
    /** Who made it. */
    readonly artist?: string | undefined;
    /** The address of its picture. */
    readonly picture?: string | undefined;
    /** Play what comes before it. */
    readonly onPrevious?: (() => void) | undefined;
    /** Play what comes after it. */
    readonly onNext?: (() => void) | undefined;
}

/** Captions a person chooses among. */
export interface PlayerCaption {
    /** What a person calls them. */
    readonly label: string;
    /** The language, as BCP 47 writes it. */
    readonly language: string;
    /** Whether they are shown. */
    readonly isShown: boolean;
}

/** A film or a sound that plays, as its parts read and steer it. */
export class PlayerControl {
    /** Whether it is paused. */
    readonly isPaused: Accessor<boolean>;
    /** Whether it waits for bytes while it plays. */
    readonly isWaiting: Accessor<boolean>;
    /** Whether it played to its end. */
    readonly isEnded: Accessor<boolean>;
    /** Where it is, in seconds. */
    readonly time: Accessor<number>;
    /** How long it is in seconds, undefined while unknown and for a live one. */
    readonly length: Accessor<number | undefined>;
    /** How far it is loaded past where it is, in seconds. */
    readonly loaded: Accessor<number>;
    /** How loud it is, from zero to one. */
    readonly volume: Accessor<number>;
    /** Whether its sound is off. */
    readonly isMuted: Accessor<boolean>;
    /** How fast it plays, one as it was made. */
    readonly rate: Accessor<number>;
    /** The captions it has, one shown at most. */
    readonly captions: Accessor<readonly PlayerCaption[]>;
    /** The lines its captions say now, none while they are not shown. */
    readonly lines: Accessor<readonly string[]>;
    /** Whether it is over the whole screen. */
    readonly isFullscreen: Accessor<boolean>;
    /** Whether it is in a window over others. */
    readonly isPictureInPicture: Accessor<boolean>;
    /** Why it could not be played, undefined while it can. */
    readonly failure: Accessor<MediaError | undefined>;
    /** The element that plays it, undefined before it is there. */
    readonly element: Accessor<HTMLMediaElement | undefined>;
    /** Replace the states an event of the element changes. */
    readonly #set: {
        readonly paused: Setter<boolean>;
        readonly waiting: Setter<boolean>;
        readonly ended: Setter<boolean>;
        readonly time: Setter<number>;
        readonly length: Setter<number | undefined>;
        readonly loaded: Setter<number>;
        readonly volume: Setter<number>;
        readonly muted: Setter<boolean>;
        readonly rate: Setter<number>;
        readonly captions: Setter<readonly PlayerCaption[]>;
        readonly lines: Setter<readonly string[]>;
        readonly fullscreen: Setter<boolean>;
        readonly pictureInPicture: Setter<boolean>;
        readonly failure: Setter<MediaError | undefined>;
        readonly element: Setter<HTMLMediaElement | undefined>;
    };
    /** The element shown over the whole screen, the film's frame. */
    #frame: HTMLElement | undefined;

    /** Play nothing yet, of a length known ahead where there is one. */
    constructor(length?: number) {
        // follow what the element tells
        const [isPaused, paused] = createSignal(true, { ownedWrite: true });
        const [isWaiting, waiting] = createSignal(false, { ownedWrite: true });
        const [isEnded, ended] = createSignal(false, { ownedWrite: true });
        const [at, time] = createSignal(0, { ownedWrite: true });
        const [known, setLength] = createSignal<number | undefined>(length, { ownedWrite: true });
        const [ahead, loaded] = createSignal(0, { ownedWrite: true });
        const [loud, volume] = createSignal(1, { ownedWrite: true });
        const [isMuted, muted] = createSignal(false, { ownedWrite: true });
        const [fast, rate] = createSignal(1, { ownedWrite: true });
        const [tracks, captions] = createSignal<readonly PlayerCaption[]>([], { ownedWrite: true });
        const [said, lines] = createSignal<readonly string[]>([], { ownedWrite: true });
        const [isFullscreen, fullscreen] = createSignal(false, { ownedWrite: true });
        const [isOver, pictureInPicture] = createSignal(false, { ownedWrite: true });
        const [error, failure] = createSignal<MediaError | undefined>(undefined, {
            ownedWrite: true,
        });
        const [player, element] = createSignal<HTMLMediaElement | undefined>(undefined, {
            ownedWrite: true,
        });
        this.isPaused = isPaused;
        this.isWaiting = isWaiting;
        this.isEnded = isEnded;
        this.time = at;
        this.length = known;
        this.loaded = ahead;
        this.volume = loud;
        this.isMuted = isMuted;
        this.rate = fast;
        this.captions = tracks;
        this.lines = said;
        this.isFullscreen = isFullscreen;
        this.isPictureInPicture = isOver;
        this.failure = error;
        this.element = player;
        this.#set = {
            paused,
            waiting,
            ended,
            time,
            length: setLength,
            loaded,
            volume,
            muted,
            rate,
            captions,
            lines,
            fullscreen,
            pictureInPicture,
            failure,
            element,
        };
        this.#frame = undefined;
    }

    /** Take the element that plays, until the signal aborts. */
    attach(element: HTMLMediaElement, signal: AbortSignal): void {
        // read each state with the events that change it
        const set = this.#set;
        const on = (names: readonly (keyof HTMLMediaElementEventMap)[], read: () => void): void => {
            for (const name of names) {
                element.addEventListener(name, read, { signal });
            }
            read();
        };
        set.element(element);
        on(["play", "pause", "ended", "emptied"], () => {
            set.paused(element.paused);
            set.ended(element.ended);
        });
        on(["waiting", "stalled", "playing", "canplay", "pause", "emptied", "error"], () => {
            const isShort = element.readyState < HTMLMediaElement.HAVE_FUTURE_DATA;
            set.waiting(!element.paused && isShort && element.error === null);
        });
        on(["timeupdate", "seeking", "seeked", "emptied"], () => set.time(element.currentTime));
        on(["progress", "timeupdate", "seeked", "emptied"], () => set.loaded(loadedOf(element)));
        on(["volumechange"], () => {
            set.volume(element.volume);
            set.muted(element.muted);
        });
        on(["ratechange"], () => set.rate(element.playbackRate));
        on(["error", "emptied", "loadedmetadata"], () => set.failure(element.error ?? undefined));

        // keep a length known ahead until the element knows it
        on(["durationchange", "loadedmetadata"], () => {
            if (!Number.isNaN(element.duration)) {
                set.length(Number.isFinite(element.duration) ? element.duration : undefined);
            }
        });

        // follow its captions, which the player writes itself, above its bar
        const tracks = element.textTracks;
        // read the lines said now
        const readLines = (): void => {
            const shown = captionsOf(element).find(({ mode }) => mode === "hidden");
            set.lines([...(shown?.activeCues ?? [])].map(linesOf).filter((line) => line !== ""));
        };
        const chosen = new Set<TextTrack>();
        const readTracks = (): void => {
            // keep the browser from writing those shown
            const captions = captionsOf(element);
            for (const track of captions) {
                const isFirst = !chosen.has(track) && isDefault(element, track);
                chosen.add(track);
                track.mode = isFirst || track.mode === "showing" ? "hidden" : track.mode;
                track.addEventListener("cuechange", readLines, { signal });
            }

            // tell which there are and which is shown
            set.captions(
                captions.map(({ label, language, mode }) => ({
                    label,
                    language,
                    isShown: mode === "hidden",
                })),
            );
            readLines();
        };
        for (const name of ["addtrack", "removetrack", "change"]) {
            tracks?.addEventListener(name, readTracks, { signal });
        }
        readTracks();

        // follow where a film is shown
        element.addEventListener("enterpictureinpicture", () => set.pictureInPicture(true), {
            signal,
        });
        element.addEventListener("leavepictureinpicture", () => set.pictureInPicture(false), {
            signal,
        });
        document.addEventListener(
            "fullscreenchange",
            () => set.fullscreen(this.#frame !== undefined && isWhole(this.#frame)),
            { signal },
        );

        // pause once another sound starts
        document.addEventListener(
            "play",
            ({ target }) => {
                const isOther = target instanceof HTMLMediaElement && target !== element;
                if (isOther && !target.muted && !element.muted) {
                    element.pause();
                }
            },
            { signal, capture: true },
        );
        signal.addEventListener("abort", () => set.element(undefined));
    }

    /** Take the element shown over the whole screen. */
    setFrame(frame: HTMLElement): void {
        this.#frame = frame;
    }

    /** Play it, telling a refusal as it fails. */
    play(): void {
        this.element()
            ?.play()
            .catch((error: unknown) => {
                // leave a play that a pause or another source ends, or that the page may not start
                const name = error instanceof DOMException ? error.name : undefined;
                if (name !== "AbortError" && name !== "NotAllowedError") {
                    this.#set.failure(this.element()?.error ?? undefined);
                }
            });
    }

    /** Pause it. */
    pause(): void {
        this.element()?.pause();
    }

    /** Play it, or pause it. */
    toggle(): void {
        if (this.element()?.paused === false) {
            this.pause();
        } else {
            this.play();
        }
    }

    /** Load it again after it failed, and play it. */
    retry(): void {
        this.element()?.load();
        this.play();
    }

    /** Go to a time in seconds, within its length. */
    seek(time: number): void {
        const element = this.element();
        if (element !== undefined) {
            const end = Number.isFinite(element.duration) ? element.duration : Infinity;
            element.currentTime = Math.min(Math.max(time, 0), end);
            this.#set.time(element.currentTime);
        }
    }

    /** Go some seconds forward or back. */
    skip(seconds: number): void {
        this.seek((this.element()?.currentTime ?? 0) + seconds);
    }

    /** Set how loud it is, from zero to one, its sound on above zero. */
    setVolume(volume: number): void {
        const element = this.element();
        if (element !== undefined) {
            element.volume = Math.min(Math.max(volume, 0), 1);
            element.muted = element.volume === 0;
        }
    }

    /** Turn the sound up, or down. */
    turn(direction: 1 | -1): void {
        const element = this.element();
        if (element !== undefined) {
            this.setVolume((element.muted ? 0 : element.volume) + direction * VOLUME_STEP);
        }
    }

    /** Mute the sound, or turn it on again, at half where it was turned down to nothing. */
    toggleMute(): void {
        const element = this.element();
        if (element !== undefined) {
            element.muted = !element.muted;
            if (!element.muted && element.volume === 0) {
                element.volume = 0.5;
            }
        }
    }

    /** Set how fast it plays. */
    setRate(rate: number): void {
        const element = this.element();
        if (element !== undefined) {
            // keep it for what plays next
            element.defaultPlaybackRate = rate;
            element.playbackRate = rate;
        }
    }

    /** Show captions by their place, or none. */
    showCaptions(shown: number | undefined): void {
        const element = this.element();
        for (const [place, track] of (element === undefined ? [] : captionsOf(element)).entries()) {
            track.mode = place === shown ? "hidden" : "disabled";
        }
    }

    /** Show its first captions, or hide those shown. */
    toggleCaptions(): void {
        this.showCaptions(this.captions().some(({ isShown }) => isShown) ? undefined : 0);
    }

    /** Report whether it can be shown over the whole screen. */
    canFullscreen(): boolean {
        return (
            typeof document !== "undefined" &&
            (document.fullscreenEnabled || phoneFilm(this.element()) !== undefined)
        );
    }

    /** Show it over the whole screen, or back in its place. */
    async toggleFullscreen(): Promise<void> {
        // show the film alone on a phone that shows no other element so
        const film = phoneFilm(this.element());
        if (!document.fullscreenEnabled) {
            film?.webkitEnterFullscreen();
        } else if (this.#frame !== undefined) {
            await (isWhole(this.#frame)
                ? document.exitFullscreen()
                : this.#frame.requestFullscreen());
        }
    }

    /** Report whether it can be shown in a window over others. */
    canPictureInPicture(): boolean {
        const element = this.element();

        return (
            typeof document !== "undefined" &&
            document.pictureInPictureEnabled &&
            element instanceof HTMLVideoElement &&
            !element.disablePictureInPicture
        );
    }

    /** Show it in a window over others, or back in its place. */
    async togglePictureInPicture(): Promise<void> {
        const element = this.element();
        if (document.pictureInPictureElement === element) {
            await document.exitPictureInPicture();
        } else if (element instanceof HTMLVideoElement) {
            await element.requestPictureInPicture();
        }
    }

    /** Let lock screens, headphones and keyboards steer it while it is the last played. */
    offer(options: () => PlayerSessionOptions, signal: AbortSignal): void {
        // offer it once it plays, where the browser has a session
        const element = this.element();
        if (element === undefined || !("mediaSession" in navigator)) {
            return;
        }
        const session = navigator.mediaSession;
        let isOffered = false;
        const offer = (): void => {
            // describe it, and hand on what a person presses
            const { title = "", artist = "", picture, onPrevious, onNext } = options();
            const artwork = picture === undefined ? [] : [{ src: picture }];
            isOffered = true;
            session.metadata = new MediaMetadata({ title, artist, artwork });
            session.setActionHandler("play", () => this.play());
            session.setActionHandler("pause", () => this.pause());
            session.setActionHandler("seekbackward", (details) =>
                this.skip(-(details.seekOffset ?? SKIP)),
            );
            session.setActionHandler("seekforward", (details) =>
                this.skip(details.seekOffset ?? SKIP),
            );
            session.setActionHandler("seekto", (details) => this.seek(details.seekTime ?? 0));
            session.setActionHandler("previoustrack", onPrevious ?? null);
            session.setActionHandler("nexttrack", onNext ?? null);
        };
        element.addEventListener("play", offer, { signal });

        // tell where it is, and leave the session as it goes
        const tell = (): void => {
            const { duration, playbackRate, currentTime } = element;
            if (isOffered && Number.isFinite(duration)) {
                const position = Math.min(currentTime, duration);
                session.setPositionState?.({ duration, playbackRate, position });
            }
        };
        for (const name of ["loadedmetadata", "play", "pause", "seeked", "ratechange"]) {
            element.addEventListener(name, tell, { signal });
        }
        signal.addEventListener("abort", () => {
            if (isOffered) {
                session.metadata = null;
                for (const action of SESSION_ACTIONS) {
                    session.setActionHandler(action, null);
                }
            }
        });
    }
}

/** A film as an iPhone shows it over the whole screen. */
interface PhoneFilm extends HTMLVideoElement {
    /** Show it over the whole screen. */
    readonly webkitEnterFullscreen: () => void;
}

/** Read a film an iPhone can show over the whole screen, undefined elsewhere. */
function phoneFilm(element: HTMLMediaElement | undefined): PhoneFilm | undefined {
    return element instanceof HTMLVideoElement && isPhoneFilm(element) ? element : undefined;
}

/** Report whether a film is one an iPhone can show over the whole screen. */
function isPhoneFilm(film: HTMLVideoElement): film is PhoneFilm {
    return "webkitEnterFullscreen" in film;
}

/** List the captions of an element. */
function captionsOf(element: HTMLMediaElement): TextTrack[] {
    return [...(element.textTracks ?? [])].filter(
        ({ kind }) => kind === "captions" || kind === "subtitles",
    );
}

/** Report whether captions are those an element shows first, which browsers choose differently. */
function isDefault(element: HTMLMediaElement, track: TextTrack): boolean {
    const tracks = [...element.querySelectorAll("track")];

    return tracks.some((each) => each.default && each.track === track);
}

/** Read the words of a cue, without the marks of who says them and how. */
function linesOf(cue: TextTrackCue): string {
    return "getCueAsHTML" in cue && cue instanceof VTTCue
        ? (cue.getCueAsHTML().textContent ?? "")
        : "";
}

/** Read how far an element is loaded where it is, in seconds. */
function loadedOf({ buffered, currentTime }: HTMLMediaElement): number {
    const ranges = Array.from({ length: buffered?.length ?? 0 }, (_range, place) => ({
        start: buffered.start(place),
        end: buffered.end(place),
    }));
    const range = ranges.find(({ start, end }) => start <= currentTime && currentTime <= end);

    return range?.end ?? currentTime;
}

/** Report whether an element is shown over the whole screen. */
function isWhole(frame: HTMLElement): boolean {
    return document.fullscreenElement === frame;
}

/** The nearest film or sound, null outside one. */
export const PlayerContext = createContext<PlayerControl | null>(null);

/** Read the nearest film or sound, refusing a part outside one. */
export function usePlayer(): PlayerControl {
    const media = useContext(PlayerContext);
    if (media === null) {
        throw new TypeError("a part of a player needs a player around it");
    }

    return media;
}
