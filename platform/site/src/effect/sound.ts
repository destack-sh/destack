/** The storage key that remembers whether sound is on. */
const storageKey = "destack-sound";
/** The loudness of every sound together, from 0 to 1. */
const volume = 0.3;
/** The loudness of the sea under the stack today. */
const seaLevel = 0.05;
/** The loudness of the music over the open stack. */
const musicLevel = 0.9;
/** The seconds after the stack opens before the music begins, once the shards have reached the ring. */
const musicDelay = 3;
/** The seconds of one eighth note of the music, at 80 beats a minute. */
const eighth = 60 / 80 / 2;
/** The eighths in one bar. */
const barSteps = 8;
/** The bars each chord of the song holds. */
const chordBars = 2;
/** The bar the bass joins in, after the arpeggio has played alone. */
const bassBar = 2;
/** The bar the melody joins in. */
const melodyBar = 4;
/** The seconds ahead the music schedules its notes, so timers never make it stumble. */
const lookahead = 0.25;
/** The milliseconds between two scheduling passes of the music. */
const scheduleGap = 80;
/** The fewest milliseconds between two splashes from stirring the water, so skimming it stays gentle. */
const stirGap = 90;
/** How far sounds pan left and right with where they happen on the page, from 0 to 1. */
const panWidth = 0.6;

/** The chords of the song in D Lydian, two bars each: the bass note, the pad's notes, and the arpeggio's notes from low to high. */
const song: readonly { bass: number; pad: readonly number[]; arpeggio: readonly number[] }[] = [
    { bass: 50, pad: [50, 57, 61], arpeggio: [62, 69, 73, 76, 78] },
    { bass: 49, pad: [49, 56, 57], arpeggio: [61, 64, 69, 71, 76] },
    { bass: 47, pad: [47, 54, 57], arpeggio: [59, 66, 69, 73, 74] },
    { bass: 43, pad: [43, 50, 54], arpeggio: [55, 62, 66, 69, 73] },
];

/** The arpeggio's two bar patterns, as indices into the chord's arpeggio notes, one per eighth. */
const patterns: readonly (readonly number[])[] = [
    [0, 1, 2, 3, 4, 3, 2, 1],
    [0, 2, 1, 3, 2, 4, 3, 2],
];

/** The melody over sixteen bars, as its notes' starting eighth, pitch, and length in eighths. */
const melody: readonly (readonly [number, number, number])[] = [
    [0, 78, 3],
    [3, 76, 1],
    [4, 74, 4],
    [8, 69, 6],
    [16, 76, 3],
    [19, 73, 1],
    [20, 76, 2],
    [22, 81, 6],
    [32, 78, 3],
    [35, 76, 1],
    [36, 74, 2],
    [38, 73, 2],
    [40, 71, 6],
    [48, 74, 2],
    [50, 73, 2],
    [52, 69, 4],
    [56, 69, 8],
    [64, 78, 3],
    [67, 76, 1],
    [68, 74, 2],
    [70, 76, 2],
    [72, 78, 2],
    [74, 81, 6],
    [80, 83, 3],
    [83, 81, 1],
    [84, 76, 4],
    [88, 73, 6],
    [96, 74, 3],
    [99, 73, 1],
    [100, 71, 2],
    [102, 78, 4],
    [106, 76, 2],
    [112, 74, 4],
    [116, 73, 2],
    [118, 74, 10],
];

/** The eighths the melody spans before it repeats. */
const melodySteps = 128;

/** The melody's notes by the eighth they start on. */
const melodyAt = new Map(melody.map(([step, midi, length]) => [step, { midi, length }]));

/** The high notes of D major pentatonic the shards chime on as they reach the ring, from low to high. */
const chimes = [74, 76, 78, 81, 83, 86, 88, 90];

/** The one-off sounds the site plays. */
export type Cue = "destack" | "restack" | "tap" | "lift" | "set" | "copy" | "theme";

/** The two moods of the figure: the sea under the stack today, and the music over the open stack. */
export type Mood = "today" | "destack";

/** One shard's flight: the seconds until it leaves its place, the seconds until it reaches its target, and where across the page it is, from 0 to 1. */
export type Flight = { leaveIn: number; reachIn: number; position: number };

/** The ways a remixed card moves that make a sound. */
export type Shift = "flip" | "fuse" | "enter" | "leave";

/** Play quiet synthesised sounds: a sea or music for the figure's mood, and short cues for what the reader does. */
export class Sound {
    /** Whether sound is on. */
    isOn: boolean;
    /** The mood the background follows. */
    mood: Mood;
    /** The audio context, created on first use. */
    context: AudioContext | undefined;
    /** The gain every sound passes through. */
    master: GainNode | undefined;
    /** The gain of the sea. */
    sea: GainNode | undefined;
    /** The dry and reverberant space the music and chimes sound in. */
    air: GainNode | undefined;
    /** The gain of the music, faded in as the stack opens and out as it closes. */
    music: GainNode | undefined;
    /** The gain the arpeggio and melody pass through, echoing left and right on the beat. */
    ring: GainNode | undefined;
    /** The gain the pad's notes pass through, dark and breathing. */
    pad: GainNode | undefined;
    /** The gain of the goo's squelch, following the pointer's stir. */
    squelch: GainNode | undefined;
    /** The filter that shapes the goo's squelch. */
    squelchFilter: BiquadFilterNode | undefined;
    /** One second of brown noise that the sea and cues shape. */
    noise: AudioBuffer | undefined;
    /** The pending scheduling pass of the music, if any. */
    timer: ReturnType<typeof setTimeout> | undefined;
    /** The next eighth of the music to schedule. */
    step: number;
    /** The audio time of the next eighth. */
    next: number;
    /** The pad's sounding voices, to fade out when the chord changes. */
    voices: GainNode[];
    /** When the water was last stirred with a sound, in milliseconds. */
    stirredAt: number;

    /** Create the sound, off until the reader turns it on. */
    constructor() {
        // start silent with no audio graph
        this.isOn = false;
        this.mood = "today";
        this.context = undefined;
        this.master = undefined;
        this.sea = undefined;
        this.air = undefined;
        this.music = undefined;
        this.ring = undefined;
        this.pad = undefined;
        this.squelch = undefined;
        this.squelchFilter = undefined;
        this.noise = undefined;
        this.timer = undefined;
        this.step = 0;
        this.next = 0;
        this.voices = [];
        this.stirredAt = 0;
    }

    /** Restore the reader's choice from the last visit, and return it; the sound starts on their next interaction. */
    restore() {
        // read the stored choice
        try {
            this.isOn = localStorage.getItem(storageKey) === "on";
        } catch (error) {
            console.warn("Could not read the sound setting", error);
        }

        // start with the reader's first press anywhere, and tap softly for every button and link they press after
        if (this.isOn) {
            window.addEventListener("pointerdown", () => this.fade(), { once: true });
        }
        document.addEventListener("pointerdown", (event) => this.tapOn(event), { capture: true });

        return this.isOn;
    }

    /** Turn sound on or off, remember the choice, and return it. */
    toggle() {
        // flip and store the choice, then fade to it
        this.isOn = !this.isOn;
        try {
            localStorage.setItem(storageKey, this.isOn ? "on" : "off");
        } catch (error) {
            console.warn("Could not save the sound setting", error);
        }
        this.fade();

        return this.isOn;
    }

    /** Follow the figure's mood: the sea under the stack, or the music over the open stack. */
    follow(mood: Mood) {
        this.mood = mood;
        if (this.context) {
            this.fade();
        }
    }

    /** Fade the master, the sea, and the music to match the setting and the mood, and start or stop the song. */
    fade() {
        // wake the graph and fade the master and the sea toward the setting and mood
        const context = this.wake();
        const now = context.currentTime;
        const isOpen = this.mood === "destack";
        this.master!.gain.setTargetAtTime(this.isOn ? volume : 0, now, 0.3);
        this.sea!.gain.setTargetAtTime(isOpen ? 0 : seaLevel, now, 0.8);

        // start the song from its first bar after the delay, or fade it out and stop it
        clearTimeout(this.timer);
        this.music!.gain.cancelScheduledValues(now);
        if (this.isOn && isOpen) {
            this.step = 0;
            this.next = now + musicDelay;
            this.music!.gain.setTargetAtTime(musicLevel, now + musicDelay, 0.4);
            this.schedule();
        } else {
            this.music!.gain.setTargetAtTime(0, now, 0.8);
            this.release(now);
        }
    }

    /** Create the audio graph on first use, and resume it after the browser suspends it. */
    wake() {
        // resume the graph once it exists
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
        low.frequency.value = 320;
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

        // send the air dry and through a wide space
        const air = context.createGain();
        air.connect(master);
        const space = context.createConvolver();
        space.buffer = hall(context, 6);
        const wet = context.createGain();
        wet.gain.value = 0.7;
        air.connect(space).connect(wet).connect(master);

        // fade the whole song through one gain
        const music = context.createGain();
        music.gain.value = 0;
        music.connect(air);

        // ring the arpeggio and melody into the song, echoing left on the dotted eighth and right on the quarter
        const ring = context.createGain();
        ring.connect(music);
        for (const [side, time] of [
            [-0.7, eighth * 1.5],
            [0.7, eighth * 2],
        ] as const) {
            const echo = context.createDelay(2);
            echo.delayTime.value = time;
            const feedback = context.createGain();
            feedback.gain.value = 0.3;
            const tone = context.createBiquadFilter();
            tone.type = "lowpass";
            tone.frequency.value = 2400;
            const pan = context.createStereoPanner();
            pan.pan.value = side;
            const send = context.createGain();
            send.gain.value = 0.22;
            ring.connect(send).connect(echo).connect(tone).connect(feedback).connect(echo);
            tone.connect(pan).connect(music);
        }

        // keep the pad dark and breathing under the song
        const pad = context.createGain();
        const breath = context.createBiquadFilter();
        breath.type = "lowpass";
        breath.frequency.value = 700;
        const lung = context.createOscillator();
        lung.frequency.value = 0.07;
        const lungDepth = context.createGain();
        lungDepth.gain.value = 300;
        lung.connect(lungDepth).connect(breath.frequency);
        lung.start();
        pad.connect(breath).connect(music);

        // squelch the goo through a resonant band that the pointer's stir opens
        const squelch = context.createGain();
        squelch.gain.value = 0;
        const squelchFilter = context.createBiquadFilter();
        squelchFilter.type = "bandpass";
        squelchFilter.Q.value = 6;
        squelchFilter.frequency.value = 300;
        const ooze = context.createBufferSource();
        ooze.buffer = noise;
        ooze.loop = true;
        ooze.connect(squelchFilter).connect(squelch).connect(master);
        ooze.start();

        // keep the graph for later cues
        this.context = context;
        this.master = master;
        this.sea = sea;
        this.air = air;
        this.music = music;
        this.ring = ring;
        this.pad = pad;
        this.squelch = squelch;
        this.squelchFilter = squelchFilter;
        this.noise = noise;

        return context;
    }

    /** Schedule the song's eighths that fall within the lookahead, then look again shortly. */
    schedule() {
        // play every eighth that is due soon
        const context = this.context!;
        while (this.next < context.currentTime + lookahead) {
            this.playStep(this.step, this.next);
            this.step += 1;
            this.next += eighth;
        }

        // look again before the lookahead runs out
        this.timer = setTimeout(() => this.schedule(), scheduleGap);
    }

    /** Play one eighth of the song at an audio time: the chord's pad and bass on its first eighth, the arpeggio, and the melody. */
    playStep(step: number, at: number) {
        // find the bar, the chord, and the eighth within the bar
        const bar = Math.floor(step / barSteps);
        const chord = song[Math.floor(bar / chordBars) % song.length];
        const beat = step % barSteps;

        // change the pad at each chord, and ground it with the bass once the bass has joined
        if (step % (barSteps * chordBars) === 0) {
            this.release(at);
            for (const midi of chord.pad) {
                this.hold(midi, at, eighth * barSteps * chordBars + 3);
            }
            if (bar >= bassBar) {
                this.bass(chord.bass, 0.07, at, eighth * barSteps * chordBars);
            }
        }

        // pick the arpeggio's note, leaning on the downbeats, growing over the first bars, and swaying left and right
        const note = chord.arpeggio[patterns[bar % patterns.length][beat]];
        const accent = beat === 0 ? 1 : beat === 4 ? 0.8 : 0.55;
        const growth = Math.min(1, 0.5 + bar * 0.25);
        const sway = beat % 2 === 0 ? -0.25 : 0.25;
        this.key(note, 0.1 * accent * growth, at + Math.random() * 0.01, sway, this.ring!);

        // sing the melody once it has joined, repeating it every sixteen bars
        if (bar >= melodyBar) {
            const line = melodyAt.get((step - melodyBar * barSteps) % melodySteps);
            if (line) {
                this.lead(line.midi, 0.07, at, line.length * eighth);
            }
        }
    }

    /** Fade out the pad's sounding voices from an audio time. */
    release(at: number) {
        for (const voice of this.voices) {
            voice.gain.cancelScheduledValues(at);
            voice.gain.setTargetAtTime(0, at, 1.2);
        }
        this.voices = [];
    }

    /** Hold one pad note: two slightly detuned soft tones that swell in slowly and fade out long. */
    hold(midi: number, start: number, length: number) {
        // swell the note in over a few seconds
        const context = this.context!;
        const frequency = pitchOf(midi);
        const gain = context.createGain();
        gain.gain.setValueAtTime(0, start);
        gain.gain.linearRampToValueAtTime(0.018, start + 2.5);
        gain.connect(this.pad!);

        // detune two soft tones against each other so the note shimmers
        for (const detune of [-2, 2]) {
            const tone = context.createOscillator();
            tone.type = "triangle";
            tone.frequency.value = frequency;
            tone.detune.value = detune;
            tone.connect(gain);
            tone.start(start);
            tone.stop(start + length);
        }
        this.voices.push(gain);
    }

    /** Strike one soft key, like a felted electric piano: a round tone with a brief bell of its octave, placed left or right. */
    key(midi: number, level: number, start: number, pan: number, output: AudioNode) {
        // place the key, and let lower notes ring a little longer
        const context = this.context!;
        const frequency = pitchOf(midi);
        const length = 1.2 + (84 - midi) * 0.03;
        const place = context.createStereoPanner();
        place.pan.value = pan;
        place.connect(output);

        // strike the round tone and its quicker octave
        for (const [ratio, weight, fade] of [
            [1, 1, 1],
            [2, 0.22, 0.35],
            [3, 0.05, 0.2],
        ] as const) {
            const tone = context.createOscillator();
            tone.frequency.value = frequency * ratio;
            const gain = context.createGain();
            gain.gain.setValueAtTime(0.0001, start);
            gain.gain.exponentialRampToValueAtTime(level * weight, start + 0.005);
            gain.gain.exponentialRampToValueAtTime(0.0001, start + length * fade);
            tone.connect(gain).connect(place);
            tone.start(start);
            tone.stop(start + length * fade + 0.01);
        }
    }

    /** Sing one melody note: a soft triangle with a warm octave below, easing in and gaining a slow vibrato as it holds. */
    lead(midi: number, level: number, start: number, length: number) {
        // ease the note in, hold it, and let it go
        const context = this.context!;
        const frequency = pitchOf(midi);
        const end = start + length;
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + 0.06);
        gain.gain.setTargetAtTime(level * 0.7, start + 0.1, 0.4);
        gain.gain.setTargetAtTime(0.0001, end - 0.05, 0.25);
        const warmth = context.createBiquadFilter();
        warmth.type = "lowpass";
        warmth.frequency.value = frequency * 2.5;
        warmth.connect(gain).connect(this.ring!);

        // sway the pitch gently once the note has settled
        const vibrato = context.createOscillator();
        vibrato.frequency.value = 4.8;
        const depth = context.createGain();
        depth.gain.setValueAtTime(0, start);
        depth.gain.linearRampToValueAtTime(7, start + 0.5);
        vibrato.connect(depth);
        vibrato.start(start);
        vibrato.stop(end + 1.2);

        // sound the triangle and its octave below
        for (const [ratio, type, weight] of [
            [1, "triangle", 1],
            [0.5, "sine", 0.4],
        ] as const) {
            const tone = context.createOscillator();
            tone.type = type;
            tone.frequency.value = frequency * ratio;
            depth.connect(tone.detune);
            const voice = context.createGain();
            voice.gain.value = weight;
            tone.connect(voice).connect(warmth);
            tone.start(start);
            tone.stop(end + 1.2);
        }
    }

    /** Play one bass note: a round sine with a little body, swelling in and fading over its length. */
    bass(midi: number, level: number, start: number, length: number) {
        // shape the note's swell and fade
        const context = this.context!;
        const frequency = pitchOf(midi);
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + 0.04);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
        gain.connect(this.music!);

        // sound the sine and a soft triangle for body on small speakers
        for (const [type, weight] of [
            ["sine", 1],
            ["triangle", 0.25],
        ] as const) {
            const tone = context.createOscillator();
            tone.type = type;
            tone.frequency.value = frequency;
            const voice = context.createGain();
            voice.gain.value = weight;
            tone.connect(voice).connect(gain);
            tone.start(start);
            tone.stop(start + length);
        }
    }

    /** Strike a key from the song's current chord as a band of the open stack comes into view. */
    pluck(step: number) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        const chord = song[Math.floor(this.step / barSteps / chordBars) % song.length];
        const note = chord.arpeggio[step % chord.arpeggio.length] + 12;
        this.key(note, 0.05, context.currentTime + 0.02, (step % 3) * 0.4 - 0.4, this.air!);
    }

    /** Break the ice into shards: tick as each breaks off, shimmer as they rise, and chime up the scale as they reach the ring. */
    shatter(flights: readonly Flight[]) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        const now = context.currentTime;

        // tick as each shard breaks off
        for (const flight of flights) {
            this.glint(
                now + flight.leaveIn,
                3000 + Math.random() * 4000,
                0.02,
                this.panOf(flight.position),
            );
        }

        // shimmer upward as the shards rise
        this.sweep(now + 0.2, 2.2, 700, 6500, 0.12);

        // chime up the scale as every third shard reaches the ring, in the order they arrive
        const arrivals = flights.toSorted((first, second) => first.reachIn - second.reachIn);
        const chimed = arrivals.filter((_, index) => index % 3 === 0);
        for (const [index, flight] of chimed.entries()) {
            const note = chimes[Math.floor((index / chimed.length) * chimes.length)];
            this.key(note, 0.035, now + flight.reachIn, this.panOf(flight.position), this.air!);
        }
    }

    /** Bring the shards home: tick as each lifts off the ring, shimmer as they fall, chime down the scale as they land, and freeze the ice. */
    gather(flights: readonly Flight[]) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        const now = context.currentTime;

        // tick as each shard lifts off the ring
        for (const flight of flights) {
            this.glint(
                now + flight.leaveIn,
                4000 + Math.random() * 3000,
                0.012,
                this.panOf(flight.position),
            );
        }

        // shimmer downward as the shards fall
        this.sweep(now + 0.1, 1.4, 6500, 700, 0.1);

        // chime down the scale as every third shard lands, in the order they land
        const landings = flights.toSorted((first, second) => first.reachIn - second.reachIn);
        const chimed = landings.filter((_, index) => index % 3 === 0);
        for (const [index, flight] of chimed.entries()) {
            const note =
                chimes[chimes.length - 1 - Math.floor((index / chimed.length) * chimes.length)] -
                12;
            this.key(note, 0.035, now + flight.reachIn, this.panOf(flight.position), this.air!);
        }

        // freeze the ice shut as the last shard lands
        const last = Math.max(...flights.map((flight) => flight.reachIn));
        this.crack(now + last, false);
    }

    /** Squelch and bubble the goo as the pointer stirs it, from still at 0 to churning at 1. */
    ooze(stir: number) {
        if (!this.isOn || !this.context) {
            return;
        }
        const now = this.context.currentTime;

        // open the squelch with the stir, wobbling its pitch
        const wobble = Math.sin(now * 9) * 60;
        this.squelch!.gain.setTargetAtTime(stir * 0.3, now, 0.08);
        this.squelchFilter!.frequency.setTargetAtTime(220 + stir * 380 + wobble, now, 0.05);

        // blorp a bubble now and then, more often the harder the stir
        if (Math.random() < stir * 0.18) {
            this.bubble(
                now,
                160 + Math.random() * 260,
                0.08 + stir * 0.08,
                (Math.random() * 2 - 1) * 0.4,
            );
        }
    }

    /** Sound a card moving in a remix after a delay in seconds, where across the board it is, from 0 to 1. */
    shift(kind: Shift, delay: number, position: number) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        const start = context.currentTime + delay;
        const pan = this.panOf(position);

        // flick a card over
        if (kind === "flip") {
            this.swish(start, 1800, 5200, 0.08, 0.05, pan);
            this.knock(start + 0.07, 900, 0.04);
        }
        // melt two cards into one, their notes gliding together
        else if (kind === "fuse") {
            this.glide(start, 69, 64, 0.04, pan);
            this.glide(start, 57, 64, 0.04, -pan);
        }
        // slide a card in
        else if (kind === "enter") {
            this.swish(start, 400, 1600, 0.3, 0.05, pan);
        }
        // slide a card out
        else {
            this.swish(start, 1600, 400, 0.3, 0.04, pan);
        }
    }

    /**
     * Splash the water, as hard as something hit it, where it hit across the page.
     *
     * Strength runs from a light stir at 0 to a heavy landing at 1; position runs from the page's left at 0 to its right at 1.
     */
    splash(strength: number, position: number) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        const now = context.currentTime;
        const pan = this.panOf(position);
        const force = Math.max(0, Math.min(1, strength));

        // wash through a band that opens wider and lasts longer the harder the hit
        const place = context.createStereoPanner();
        place.pan.value = pan;
        place.connect(this.master!);
        const source = context.createBufferSource();
        source.buffer = this.noise!;
        const high = context.createBiquadFilter();
        high.type = "highpass";
        high.frequency.value = 500 - force * 250;
        const low = context.createBiquadFilter();
        low.type = "lowpass";
        low.frequency.value = 1400 + force * 2600;
        const length = 0.18 + force * 0.5;
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.exponentialRampToValueAtTime(0.12 + force * 0.6, now + 0.015);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + length);
        source.connect(high).connect(low).connect(gain).connect(place);
        source.start(now, Math.random() * 0.5);
        source.stop(now + length + 0.02);

        // pop more and deeper bubbles the harder the hit
        const bubbles = 1 + Math.round(force * 4 + Math.random());
        for (let index = 0; index < bubbles; index++) {
            const at = now + 0.02 + Math.random() * (0.1 + force * 0.25);
            const pitch = (900 - force * 400) * (0.7 + Math.random() * 0.8);
            this.bubble(at, pitch, 0.05 + force * 0.05, pan);
        }
    }

    /** Splash the water lightly where the pointer skims it, no more often than a gentle rhythm allows. */
    stir(strength: number, position: number) {
        // skip stirs that come too soon after the last
        const now = performance.now();
        if (now - this.stirredAt < stirGap) {
            return;
        }

        // splash softly
        this.stirredAt = now;
        this.splash(strength * 0.35, position);
    }

    /** Tap softly for a pressed button or link, unless it makes its own sound. */
    tapOn(event: PointerEvent) {
        const target = (event.target as Element | null)?.closest("a, button, summary");
        if (!target || target.matches('[role="switch"], [disabled], [data-silent]')) {
            return;
        }
        this.play("tap");
    }

    /** Play one cue, if sound is on. */
    play(cue: Cue) {
        if (!this.isOn) {
            return;
        }
        const context = this.wake();
        const now = context.currentTime;

        // click the switch down, crack the ice open, and pull the water down into the drain
        if (cue === "destack") {
            this.click(now, 1);
            this.crack(now + 0.05, true);
            this.rush(now + 0.1, 3, true);
        }
        // click the switch back and well the water up out of the drain
        else if (cue === "restack") {
            this.click(now, 0.8);
            this.rush(now + 0.1, 3, false);
        }
        // pick something up with a small wooden lift
        else if (cue === "lift") {
            this.knock(now, 520, 0.18);
        }
        // set something down with a lower, softer knock
        else if (cue === "set") {
            this.knock(now, 300, 0.22);
        }
        // strike two keys, rising, for something copied
        else if (cue === "copy") {
            this.key(74, 0.06, now + 0.01, 0, this.air!);
            this.key(81, 0.05, now + 0.09, 0, this.air!);
        }
        // flip a small latch for the theme
        else if (cue === "theme") {
            this.knock(now, 700, 0.12);
            this.knock(now + 0.06, 460, 0.1);
        }
        // tap a button or link softly
        else {
            this.knock(now, 1100, 0.07);
        }
    }

    /** Turn a position across the page, from 0 to 1, into a pan. */
    panOf(position: number) {
        return (Math.max(0, Math.min(1, position)) * 2 - 1) * panWidth;
    }

    /** Crack ice at an audio time: a low boom, a crunch of splinters, and a creak, as it breaks open or freezes shut. */
    crack(start: number, isBreaking: boolean) {
        // boom the body of the ice
        const context = this.context!;
        const boom = context.createOscillator();
        boom.frequency.setValueAtTime(isBreaking ? 130 : 90, start);
        boom.frequency.exponentialRampToValueAtTime(40, start + 0.4);
        const boomGain = context.createGain();
        boomGain.gain.setValueAtTime(0.0001, start);
        boomGain.gain.exponentialRampToValueAtTime(isBreaking ? 0.3 : 0.2, start + 0.01);
        boomGain.gain.exponentialRampToValueAtTime(0.0001, start + 0.5);
        boom.connect(boomGain).connect(this.master!);
        boom.start(start);
        boom.stop(start + 0.52);

        // crunch splinters, spreading out as it breaks and closing in as it freezes
        for (let index = 0; index < 14; index++) {
            const spread = Math.random() ** (isBreaking ? 1.6 : 0.6) * 0.4;
            const at = isBreaking ? start + spread : start - spread;
            this.burst(at, 1500 + Math.random() * 3500, 0.04 + Math.random() * 0.12);
        }

        // creak the ice through a narrow band that slides down
        const creak = context.createOscillator();
        creak.type = "sawtooth";
        creak.frequency.value = 48;
        const band = context.createBiquadFilter();
        band.type = "bandpass";
        band.Q.value = 12;
        band.frequency.setValueAtTime(isBreaking ? 900 : 500, start);
        band.frequency.exponentialRampToValueAtTime(isBreaking ? 260 : 700, start + 0.6);
        const creakGain = context.createGain();
        creakGain.gain.setValueAtTime(0.0001, start);
        creakGain.gain.exponentialRampToValueAtTime(0.1, start + 0.05);
        creakGain.gain.exponentialRampToValueAtTime(0.0001, start + 0.7);
        creak.connect(band).connect(creakGain).connect(this.master!);
        creak.start(start);
        creak.stop(start + 0.72);
    }

    /** Sweep a shimmer of noise from one pitch to another over a duration, swelling and fading. */
    sweep(start: number, duration: number, from: number, to: number, level: number) {
        // loop the noise through a resonant band that sweeps
        const context = this.context!;
        const source = context.createBufferSource();
        source.buffer = this.noise!;
        source.loop = true;
        const band = context.createBiquadFilter();
        band.type = "bandpass";
        band.Q.value = 4;
        band.frequency.setValueAtTime(from, start);
        band.frequency.exponentialRampToValueAtTime(to, start + duration);

        // swell to the middle of the sweep and fade into the space
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + duration * 0.5);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
        source.connect(band).connect(gain).connect(this.air!);
        source.start(start);
        source.stop(start + duration);
    }

    /** Swish air past something moving, from one pitch to another over a duration, placed left or right. */
    swish(start: number, from: number, to: number, duration: number, level: number, pan: number) {
        // loop the noise through a band that slides with the move
        const context = this.context!;
        const source = context.createBufferSource();
        source.buffer = this.noise!;
        const band = context.createBiquadFilter();
        band.type = "bandpass";
        band.Q.value = 1.5;
        band.frequency.setValueAtTime(from, start);
        band.frequency.exponentialRampToValueAtTime(to, start + duration);

        // swell and fade with the move
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + duration * 0.4);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
        const place = context.createStereoPanner();
        place.pan.value = pan;
        source.connect(band).connect(gain).connect(place).connect(this.master!);
        source.start(start, Math.random() * 0.5);
        source.stop(start + duration + 0.02);
    }

    /** Glide a soft tone from one note to another, as two things melt into one. */
    glide(start: number, from: number, to: number, level: number, pan: number) {
        // slide the pitch and let it ring out
        const context = this.context!;
        const tone = context.createOscillator();
        tone.frequency.setValueAtTime(pitchOf(from), start);
        tone.frequency.exponentialRampToValueAtTime(pitchOf(to), start + 0.25);
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + 0.04);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.9);
        const place = context.createStereoPanner();
        place.pan.value = pan;
        tone.connect(gain).connect(place).connect(this.air!);
        tone.start(start);
        tone.stop(start + 0.92);
    }

    /** Pop one bubble at a pitch: a short tone that slides up as it rises, placed left or right. */
    bubble(start: number, pitch: number, level: number, pan: number) {
        // slide a short tone up
        const context = this.context!;
        const tone = context.createOscillator();
        tone.frequency.setValueAtTime(pitch, start);
        tone.frequency.exponentialRampToValueAtTime(pitch * 1.9, start + 0.05);

        // pop it briefly, placed left or right
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + 0.005);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.07);
        const place = context.createStereoPanner();
        place.pan.value = pan;
        tone.connect(gain).connect(place).connect(this.master!);
        tone.start(start);
        tone.stop(start + 0.08);
    }

    /** Rush water for a duration: a roar that swirls down in pitch as the water drains, or up as it fills. */
    rush(start: number, duration: number, isDraining: boolean) {
        // loop the noise for the rush
        const context = this.context!;
        const source = context.createBufferSource();
        source.buffer = this.noise!;
        source.loop = true;

        // shape the roar through a band that swirls down as the water drains and up as it fills
        const band = context.createBiquadFilter();
        band.type = "bandpass";
        band.Q.value = 1.1;
        band.frequency.setValueAtTime(isDraining ? 1600 : 220, start);
        band.frequency.exponentialRampToValueAtTime(isDraining ? 160 : 1400, start + duration);

        // swirl by wobbling the level, faster as the water nears the drain
        const gurgle = context.createGain();
        gurgle.gain.value = 0.7;
        const wobble = context.createOscillator();
        wobble.frequency.setValueAtTime(isDraining ? 4 : 9, start);
        wobble.frequency.linearRampToValueAtTime(isDraining ? 9 : 4, start + duration);
        const depth = context.createGain();
        depth.gain.value = 0.3;
        wobble.connect(depth).connect(gurgle.gain);

        // swell and fade with the move
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(3.5, start + duration * 0.35);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
        source.connect(band).connect(gurgle).connect(gain).connect(this.master!);
        source.start(start);
        source.stop(start + duration);
        wobble.start(start);
        wobble.stop(start + duration);
    }

    /** Click like a well-made switch, at a strength: a crisp press and a weighty thud, then a lighter release. */
    click(start: number, strength: number) {
        // press and release the switch
        const context = this.context!;
        this.burst(start, 3400, 0.5 * strength);
        this.burst(start + 0.055, 4200, 0.25 * strength);

        // thud the body under the press
        const body = context.createOscillator();
        body.frequency.setValueAtTime(220, start);
        body.frequency.exponentialRampToValueAtTime(70, start + 0.07);
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.8 * strength, start);
        gain.gain.exponentialRampToValueAtTime(0.001, start + 0.1);
        body.connect(gain).connect(this.master!);
        body.start(start);
        body.stop(start + 0.11);
    }

    /** Knock once on wood at a pitch and level: a short sine with a quick pitch drop and a touch of noise. */
    knock(start: number, pitch: number, level: number) {
        // drop the pitch quickly for the body of the knock
        const context = this.context!;
        const body = context.createOscillator();
        body.frequency.setValueAtTime(pitch, start);
        body.frequency.exponentialRampToValueAtTime(pitch * 0.6, start + 0.05);
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + 0.003);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.08);
        body.connect(gain).connect(this.master!);
        body.start(start);
        body.stop(start + 0.09);

        // add the grain of the surface
        this.burst(start, pitch * 4, level * 0.35);
    }

    /** Tick a tiny glassy glint, as a shard of ice breaks off, placed left or right. */
    glint(start: number, pitch: number, level: number, pan: number) {
        // ring a short high tone
        const context = this.context!;
        const tone = context.createOscillator();
        tone.frequency.value = pitch;
        const gain = context.createGain();
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(level, start + 0.002);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.12);
        const place = context.createStereoPanner();
        place.pan.value = pan;
        tone.connect(gain).connect(place).connect(this.air!);
        tone.start(start);
        tone.stop(start + 0.13);
    }

    /** Crack a short burst of noise through a high band, at a level. */
    burst(start: number, frequency: number, level: number) {
        // filter the noise into a short crack
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

/** The site's one sound. */
export const sound = new Sound();

/** Return the frequency of a MIDI note in hertz. */
function pitchOf(midi: number) {
    return 440 * 2 ** ((midi - 69) / 12);
}

/** Build a soft reverb tail of a given length in seconds from slowly darkening, decaying noise. */
function hall(context: AudioContext, seconds: number) {
    // fill both sides with decaying noise that loses its highs as it fades
    const length = Math.floor(context.sampleRate * seconds);
    const buffer = context.createBuffer(2, length, context.sampleRate);
    for (let side = 0; side < 2; side++) {
        const samples = buffer.getChannelData(side);
        let last = 0;
        for (let index = 0; index < length; index++) {
            const progress = index / length;
            const smooth = 0.4 + progress * 0.5;
            last = last * smooth + (Math.random() * 2 - 1) * (1 - smooth);
            samples[index] = last * (1 - progress) ** 2.2;
        }
    }

    return buffer;
}
