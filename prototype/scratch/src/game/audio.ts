// NOTE @Performance: tiny audio helper with preloaded buffers to avoid latency

type SoundName = "hit" | "brick" | "lose" | "power" | "start" | "win";

export class AudioManager {
    private context: AudioContext | null = null;
    private buffers: Map<SoundName, AudioBuffer> = new Map();

    async init(): Promise<void> {
        if (this.context) return;
        this.context = new (window.AudioContext || (window as any).webkitAudioContext)();
        await this.context.resume();
        // generate simple tones procedurally to avoid asset pipeline
        await Promise.all([
            this.makeTone("hit", 880, 0.045),
            this.makeTone("brick", 420, 0.06),
            this.makeTone("lose", 180, 0.22),
            this.makeTone("power", 640, 0.12),
            this.makeTone("start", 520, 0.15),
            this.makeTone("win", 740, 0.25),
        ]);
    }

    private async makeTone(name: SoundName, freq: number, dur: number): Promise<void> {
        if (!this.context) return;
        const sampleRate = this.context.sampleRate;
        const length = Math.floor(sampleRate * dur);
        const buffer = this.context.createBuffer(1, length, sampleRate);
        const data = buffer.getChannelData(0);
        for (let i = 0; i < length; i++) {
            const t = i / sampleRate;
            const env = Math.exp((-3 * t) / dur);
            data[i] = Math.sin(2 * Math.PI * freq * t) * env * 0.25;
        }
        this.buffers.set(name, buffer);
    }

    play(name: SoundName, playbackRate = 1): void {
        if (!this.context) return;
        const buffer = this.buffers.get(name);
        if (!buffer) return;
        const src = this.context.createBufferSource();
        src.buffer = buffer;
        src.playbackRate.value = playbackRate;
        const gain = this.context.createGain();
        gain.gain.value = 0.5;
        src.connect(gain).connect(this.context.destination);
        src.start();
    }
}

export const audioManager = new AudioManager();
