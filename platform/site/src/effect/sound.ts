/// The storage key that remembers whether sound is on.
const storageKey = "destack-sound";
/// The loudness of every sound together, from 0 to 1.
const volume = 0.4;
/// The loudness of the sea under the stack today.
const seaLevel = 0.012;
/// The loudness of the piano over the open stack.
const pianoLevel = 0.16;
/// The seconds per beat: a slow, unhurried 72 beats a minute.
const beat = 60 / 72;

/// The piece's eight bars of chords, each as the notes its left hand breaks into eighths.
const bars: readonly (readonly number[])[] = [
    [41, 48, 57, 64],
    [45, 52, 60, 67],
    [38, 45, 53, 64],
    [46, 53, 62, 69],
    [41, 48, 57, 64],
    [40, 48, 55, 62],
    [38, 45, 53, 60],
    [46, 53, 57, 62],
];
/// The order the left hand breaks each chord into eight eighths.
const pattern = [0, 1, 2, 1, 3, 1, 2, 1];
/// The melody over the eight bars, as notes and their beats; a note below zero is a rest.
const melody: readonly (readonly [number, number])[] = [
    [69, 1.5],
    [72, 0.5],
    [67, 2],
    [64, 3],
    [-1, 1],
    [62, 1],
    [65, 1],
    [69, 1],
    [72, 1],
    [74, 3],
    [-1, 1],
    [72, 1.5],
    [69, 0.5],
    [67, 1],
    [69, 1],
    [64, 2],
    [67, 2],
    [65, 1],
    [64, 1],
    [62, 1],
    [69, 1],
    [65, 4],
];

/// The one-off sounds the site plays.
export type Cue = "destack" | "restack" | "click" | "splash";

/// The two moods of the figure: the sea under the stack today, and the piano over the open stack.
export type Mood = "today" | "destack";

/// Play quiet synthesised sounds: a sea or a piano for the figure's mood, and short cues for what happens.
export class Sound {
    /// Whether sound is on.
    isOn: boolean;
    /// The mood the background follows.
    mood: Mood;
    /// The audio context, created on first use.
    context: AudioContext | undefined;
    /// The gain every sound passes through.
    master: GainNode | undefined;
    /// The gain of the sea.
    sea: GainNode | undefined;
    /// The gain of the piano.
    piano: GainNode | undefined;
    /// One second of brown noise that the sea and cues shape.
    noise: AudioBuffer | undefined;
    /// The pending piano note, if any.
    timer: ReturnType<typeof setTimeout> | undefined;
    /// The next bar to play.
    step: number;
    /// The audio time the next bar starts.
    next: number;

    /// Create the sound, off until the reader turns it on.
    constructor() {
        this.isOn = false;
        this.mood = "today";
        this.context = undefined;
        this.master = undefined;
        this.sea = undefined;
        this.piano = undefined;
        this.noise = undefined;
        this.timer = undefined;
        this.step = 0;
        this.next = 0;
    }

    /// Restore the reader's choice from the last visit, and return it; the sound starts on their next interaction.
    restore() {
        try {
            this.isOn = localStorage.getItem(storageKey) === "on";
        } catch (error) {
            console.warn("Could not read the sound setting", error);
        }

        // start with the reader's first click anywhere
        if (this.isOn) {
            window.addEventListener("pointerdown", () => this.fade(), { once: true });
        }

        return this.isOn;
    }

    /// Turn sound on or off, remember the choice, and return it.
    toggle() {
        this.isOn = !this.isOn;
        try {
            localStorage.setItem(storageKey, this.isOn ? "on" : "off");
        } catch (error) {
            console.warn("Could not save the sound setting", error);
        }
        this.fade();

        return this.isOn;
    }

    /// Follow the figure's mood: the sea under the stack, or the piano over the open stack.
    follow(mood: Mood) {
        this.mood = mood;
        if (this.context) {
            this.fade();
        }
    }

    /// Fade the master, the sea, and the piano to match the setting and the mood.
    fade() {
        const context = this.wake();
        const now = context.currentTime;
        const isOpen = this.mood === "destack";
        this.master!.gain.setTargetAtTime(this.isOn ? volume : 0, now, 0.3);
        this.sea!.gain.setTargetAtTime(isOpen ? 0 : seaLevel, now, 0.8);
        this.piano!.gain.setTargetAtTime(isOpen ? pianoLevel : 0, now, 0.8);

        // play the piano only while it can be heard
        clearTimeout(this.timer);
        if (this.isOn && isOpen) {
            this.playPiano();
        }
    }

    /// Create the audio graph on first use, and resume it after the browser suspends it.
    wake() {
        if (this.context) {
            void this.context.resume();
            return this.context;
        }

        // shape one second of brown noise to loop and filter
        const context = new AudioContext();
        const noise = context.createBuffer(1, context.sampleRate, context.sampleRate);
        const samples = noise.getChannelData(0);
        let last = 0;
        for (let index = 0; index < samples.length; index++) {
            last = (last + 0.02 * (Math.random() * 2 - 1)) / 1.02;
            samples[index] = last * 3.5;
        }

        // route everything through one master gain, silent until on
        const master = context.createGain();
        master.gain.value = 0;
        master.connect(context.destination);

        // wash a soft, low sea that swells and ebbs every twelve seconds or so
        const sea = context.createGain();
        sea.gain.value = 0;
        sea.connect(master);
        const wash = context.createBufferSource();
        wash.buffer = noise;
        wash.loop = true;
        const low = context.createBiquadFilter();
        low.type = "lowpass";
        low.frequency.value = 260;
        const swell = context.createGain();
        swell.gain.value = 0.6;
        const tide = context.createOscillator();
        tide.frequency.value = 0.08;
        const depth = context.createGain();
        depth.gain.value = 0.35;
        tide.connect(depth).connect(swell.gain);
        wash.connect(low).connect(swell).connect(sea);
        wash.start();
        tide.start();

        // soften the piano like felt, and let it bloom in a large, dark room
        const piano = context.createGain();
        piano.gain.value = 0;
        const felt = context.createBiquadFilter();
        felt.type = "lowpass";
        felt.frequency.value = 2000;
        const room = context.createConvolver();
        room.buffer = hall(context, 5);
        const wet = context.createGain();
        wet.gain.value = 0.7;
        piano.connect(felt);
        felt.connect(master);
        felt.connect(room).connect(wet).connect(master);

        this.context = context;
        this.master = master;
        this.sea = sea;
        this.piano = piano;
        this.noise = noise;

        return context;
    }

    /// Play the next bar of the piece and schedule the one after: arpeggios always,
    /// the melody on every other pass, and a few high notes on the passes between.
    playPiano() {
        const context = this.context!;
        const bar = this.step % bars.length;
        const pass = Math.floor(this.step / bars.length);
        const start = Math.max(this.next, context.currentTime + 0.05);
        this.step += 1;

        // break the chord into gentle eighths, the bass a little heavier
        const chord = bars[bar];
        pattern.forEach((index, eighth) => {
            const level = eighth === 0 ? 0.26 : 0.13;
            this.note(chord[index], level + Math.random() * 0.03, start + eighth * beat * 0.5, 4);
        });

        // sing the melody on every other pass, and scatter a few high notes on the passes between
        if (pass % 2 === 1) {
            let at = start;
            let beats = 0;
            for (const [midi, length] of melody) {
                if (beats >= bar * 4 && beats < bar * 4 + 4 && midi > 0) {
                    this.note(midi, 0.3, at + (beats - bar * 4) * beat, 5);
                }
                beats += length;
            }
        } else if (Math.random() < 0.35) {
            this.note(chord[3] + 12, 0.12, start + beat * 2, 5);
        }

        // queue the next bar a little ahead of time
        this.next = start + beat * 4;
        this.timer = setTimeout(
            () => this.playPiano(),
            (this.next - context.currentTime - 0.3) * 1000,
        );
    }

    /// Strike one felt piano note at a time: a soft hammer, a warm body, and a long fading tail.
    note(midi: number, level: number, start: number, length: number) {
        const context = this.context!;
        const frequency = 440 * 2 ** ((midi - 69) / 12);
        const gain = context.createGain();
        gain.gain.setValueAtTime(0, start);
        gain.gain.linearRampToValueAtTime(level, start + 0.012);
        gain.gain.exponentialRampToValueAtTime(level * 0.3, start + 0.6);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
        gain.connect(this.piano!);
        for (const [ratio, weight] of [
            [1, 1],
            [2.001, 0.28],
            [3.003, 0.06],
        ] as const) {
            const partial = context.createOscillator();
            partial.frequency.value = frequency * ratio;
            const share = context.createGain();
            share.gain.value = weight;
            partial.connect(share).connect(gain);
            partial.start(start);
            partial.stop(start + length);
        }
    }

    /// Pluck the next note of a rising arpeggio as each band of the open stack comes into view.
    pluck(step: number) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        this.note([74, 78, 81, 86][step % 4], 0.22, context.currentTime + 0.02, 4);
    }

    /// Play one cue, if sound is on.
    play(cue: Cue) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        const now = context.currentTime;

        // click the switch down, crack the ice, rush the water away, and answer with a rising fifth
        if (cue === "destack") {
            this.click(now);
            for (let index = 0; index < 6; index++) {
                this.burst(
                    now + 0.1 + index * 0.07 + Math.random() * 0.05,
                    2200 + Math.random() * 2400,
                    0.3,
                );
            }
            this.rush(now + 0.1, 3.2, true);
            this.note(62, 0.4, now + 0.05, 5);
            this.note(69, 0.34, now + 0.22, 5);
        }
        // click the switch back, rush the water in, and answer with a falling fifth
        else if (cue === "restack") {
            this.click(now);
            this.rush(now + 0.1, 3.2, false);
            this.note(69, 0.34, now + 0.05, 5);
            this.note(62, 0.3, now + 0.22, 5);
        }
        // lap the water faintly against something afloat
        else if (cue === "splash") {
            this.lap(now);
        }
        // tick softly
        else {
            this.burst(now, 3200, 0.12);
        }
    }

    /// Rush water for a duration: a roar that swells and fades with the move, gurgling as it drains or fills.
    rush(start: number, duration: number, isDraining: boolean) {
        const context = this.context!;
        const source = context.createBufferSource();
        source.buffer = this.noise!;
        source.loop = true;

        // shape the roar through a band that falls as the water drains and rises as it fills
        const band = context.createBiquadFilter();
        band.type = "bandpass";
        band.Q.value = 0.7;
        band.frequency.setValueAtTime(isDraining ? 1400 : 300, start);
        band.frequency.exponentialRampToValueAtTime(isDraining ? 260 : 1200, start + duration);

        // gurgle by wobbling the level quickly
        const gurgle = context.createGain();
        gurgle.gain.value = 0.7;
        const wobble = context.createOscillator();
        wobble.frequency.value = 6.5;
        const depth = context.createGain();
        depth.gain.value = 0.3;
        wobble.connect(depth).connect(gurgle.gain);

        // swell and fade with the move
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(1.6, start + duration * 0.35);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
        source.connect(band).connect(gurgle).connect(gain).connect(this.master!);
        source.start(start);
        source.stop(start + duration);
        wobble.start(start);
        wobble.stop(start + duration);
    }

    /// Lap a little water: a short, dull wash with a few tiny bubbles in it.
    lap(start: number) {
        const context = this.context!;

        // wash softly through a fixed, muffled band
        const source = context.createBufferSource();
        source.buffer = this.noise!;
        const high = context.createBiquadFilter();
        high.type = "highpass";
        high.frequency.value = 350;
        const low = context.createBiquadFilter();
        low.type = "lowpass";
        low.frequency.value = 1800;
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(0.07, start + 0.03);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.28);
        source.connect(high).connect(low).connect(gain).connect(this.master!);
        source.start(start, Math.random() * 0.6);
        source.stop(start + 0.3);

        // pop two or three small bubbles, each a quick upward chirp
        const bubbles = 2 + Math.floor(Math.random() * 2);
        for (let index = 0; index < bubbles; index++) {
            const at = start + 0.02 + Math.random() * 0.16;
            const pitch = 600 + Math.random() * 700;
            const bubble = context.createOscillator();
            bubble.frequency.setValueAtTime(pitch, at);
            bubble.frequency.exponentialRampToValueAtTime(pitch * 1.7, at + 0.035);
            const pop = context.createGain();
            pop.gain.setValueAtTime(0.0001, at);
            pop.gain.exponentialRampToValueAtTime(0.025, at + 0.004);
            pop.gain.exponentialRampToValueAtTime(0.0001, at + 0.05);
            bubble.connect(pop).connect(this.master!);
            bubble.start(at);
            bubble.stop(at + 0.06);
        }
    }

    /// Click like a well-made switch: a crisp press and a weighty thud, then a lighter release.
    click(start: number) {
        const context = this.context!;
        this.burst(start, 3400, 0.9);
        this.burst(start + 0.055, 4200, 0.45);

        // thud the body under the press
        const body = context.createOscillator();
        body.frequency.setValueAtTime(220, start);
        body.frequency.exponentialRampToValueAtTime(70, start + 0.07);
        const gain = context.createGain();
        gain.gain.setValueAtTime(1.4, start);
        gain.gain.exponentialRampToValueAtTime(0.001, start + 0.1);
        body.connect(gain).connect(this.master!);
        body.start(start);
        body.stop(start + 0.11);
    }

    /// Crack a short burst of noise through a high band, at a level.
    burst(start: number, frequency: number, level: number) {
        const context = this.context!;
        const source = context.createBufferSource();
        source.buffer = this.noise!;
        const band = context.createBiquadFilter();
        band.type = "highpass";
        band.frequency.value = frequency;
        const gain = context.createGain();
        gain.gain.setValueAtTime(level, start);
        gain.gain.exponentialRampToValueAtTime(0.001, start + 0.03);
        source.connect(band).connect(gain).connect(this.master!);
        source.start(start, Math.random() * 0.8);
        source.stop(start + 0.04);
    }
}

/// The site's one sound.
export const sound = new Sound();

/// Build a soft, dark reverb tail of a given length in seconds from decaying noise.
function hall(context: AudioContext, seconds: number) {
    const length = Math.floor(context.sampleRate * seconds);
    const buffer = context.createBuffer(2, length, context.sampleRate);
    for (let channel = 0; channel < 2; channel++) {
        const samples = buffer.getChannelData(channel);
        let last = 0;
        for (let index = 0; index < length; index++) {
            last = last * 0.7 + (Math.random() * 2 - 1) * 0.3;
            samples[index] = last * (1 - index / length) ** 2.5;
        }
    }

    return buffer;
}
