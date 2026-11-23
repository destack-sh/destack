import { styleText } from "node:util";
import type { Environment } from "@destack/environment";
import { Localization, LocaleTag } from "@destack/locale";
import { DEFAULT_HUES, type Hue } from "@destack/color";
import type { KeyPress } from "@destack/locale";
import { Prompt } from "./prompt.ts";

/** A text format understood by the terminal. */
export type Format = Parameters<typeof styleText>[0];

/** Format text in a format, or leave it as it is. */
export type Style = (format: Format, text: string) => string;

/** The process a command line writes its output to and reads its answers from. */
export type TerminalProcess = Environment<"process">["process"];

/** Leave text unformatted. */
const plain: Style = (_, text) => text;

/** The colours of a terminal's tones: the brand's orange, and the green and red swatches. */
const TONES = {
    accent: "#ff792e",
    done: DEFAULT_HUES.green,
    failed: DEFAULT_HUES.red,
} as const;

/** The COLORTERM values of terminals painting 24-bit colour. */
const TRUECOLOR_TERMINALS = new Set(["truecolor", "24bit"]);

/** The levels of each channel in the 256-colour palette's 6×6×6 cube. */
const CUBE_LEVELS: readonly number[] = [0, 95, 135, 175, 215, 255];

/** The index of the 256-colour palette's cube, after the 16 system colours. */
const CUBE_START = 16;

/** A colour a terminal paints in. */
export type Tone = keyof typeof TONES;

/** The keys a terminal sends in raw mode as escape sequences or control bytes, by their bytes. */
const SEQUENCES: Readonly<Record<string, string>> = {
    "\x1b[A": "ArrowUp",
    "\x1b[B": "ArrowDown",
    "\x1b[C": "ArrowRight",
    "\x1b[D": "ArrowLeft",
    "\r": "Enter",
    "\n": "Enter",
    "\x1b": "Escape",
    "\x7f": "Backspace",
    "\t": "Tab",
};

/** The first control byte, ctrl-a, which the control bytes count from. */
const CONTROL_A = 0x01;

/** The character code of a, which ctrl-a stands for. */
const LETTER_A = 0x61;

/** The last control byte a letter makes, ctrl-z. */
const CONTROL_Z = 0x1a;

/** One key's bytes in a chunk: an escape sequence such as an arrow, else one character. */
const KEY_SEQUENCE = /\x1b\[[A-D]|[\s\S]/gu;

/** The width of the gutter an event and a hint put their one word in, in characters. */
const GUTTER_WIDTH = 10;

/** How an event stands: under way, done, failed, or a step without a state of its own. */
export type EventState = "running" | "done" | "failed" | "step";

/** How a listed thing stands: ready, changing, off or failed. */
export type Status = "ready" | "busy" | "off" | "failed";

/** The dot of each status, which a list shows before a row. */
const DOTS: Readonly<Record<Status, { readonly glyph: string; readonly tone?: Tone }>> = {
    ready: { glyph: "●", tone: "done" },
    busy: { glyph: "◐", tone: "accent" },
    off: { glyph: "○" },
    failed: { glyph: "✕", tone: "failed" },
};

/** The colour of each state's word, none for a step. */
const STATE_TONES: Readonly<Record<EventState, Tone | undefined>> = {
    running: "accent",
    done: "done",
    failed: "failed",
    step: undefined,
};

/** The terminal a command line writes its output and diagnostics to. */
export class Terminal implements AsyncDisposable {
    /** The process whose standard streams the terminal writes. */
    readonly process: TerminalProcess;
    /** Format text only when standard output is a terminal accepting colour. */
    readonly style: Style;
    /** Paint text in a colour only when standard output is a terminal accepting colour. */
    readonly tone: (tone: Tone, text: string) => string;
    /** Paint text in a hue, as objects and highlights take them. */
    readonly swatch: (hue: Hue, text: string) => string;
    /** Whether standard output is a terminal, which tables align for, else tab-separated. */
    readonly isTerminal: boolean;
    /** The answers a person types or a script pipes on standard input, read by one reader. */
    readonly prompt: Prompt;
    /** The time zone the terminal shows times in, such as Europe/Vienna. */
    readonly timeZone: string;
    /** The locale the terminal writes dates in, such as de-AT. */
    readonly locale: string;
    /** The messages the terminal writes, in its locale where a catalog has them. */
    readonly localization: Localization;

    /** Write to a process's standard streams. */
    constructor(running: TerminalProcess, place: Pick<Environment, "timeZone" | "locale">) {
        // set the process, and colour output only on a terminal without NO_COLOR
        this.process = running;
        this.isTerminal = running.stdout.isTerminal;
        this.timeZone = place.timeZone;
        this.locale = place.locale;
        this.localization = Localization.of(LocaleTag.parse(place.locale), []);
        const isColored = this.isTerminal && running.variables["NO_COLOR"] === undefined;
        this.style = isColored
            ? (format, text) => styleText(format, text, { validateStream: false })
            : plain;
        const isTruecolor = TRUECOLOR_TERMINALS.has(running.variables["COLORTERM"] ?? "");
        const paint = (hex: string, text: string) =>
            isColored ? `\x1b[${foreground(hex, isTruecolor)}m${text}\x1b[39m` : text;
        this.tone = (tone, text) => paint(TONES[tone], text);
        this.swatch = (hue, text) => paint(DEFAULT_HUES[hue], text);
        this.prompt = new Prompt(running, (text) => this.question(text));
    }

    /** The terminal's width in characters as it stands now, absent without a terminal. */
    get columns(): number | undefined {
        return this.process.stdout.columns;
    }

    /** The terminal's height in lines as it stands now, absent without a terminal. */
    get rows(): number | undefined {
        return this.process.stdout.rows;
    }

    /** Read the terminal's keys as they are pressed, in raw mode until the reading ends. */
    async *keys(): AsyncGenerator<KeyPress> {
        // switch the terminal to raw mode and read its bytes
        const { stdin } = this.process;
        stdin.setRawMode(true);
        const reader = stdin.readable.getReader();
        const decoder = new TextDecoder();

        // yield each key a chunk holds, an escape sequence as one key, restoring the mode once done
        try {
            for (let read = await reader.read(); !read.done; read = await reader.read()) {
                const text = decoder.decode(read.value, { stream: true });
                for (const sequence of text.match(KEY_SEQUENCE) ?? []) {
                    yield pressOf(sequence);
                }
            }
        } finally {
            reader.releaseLock();
            stdin.setRawMode(false);
        }
    }

    /** Write that something happened: one word in the gutter, what it is about, a dim detail. */
    event(word: string, state: EventState, text: string, detail?: string): string {
        // paint the word by its state, and dim the detail
        const tone = STATE_TONES[state];
        const painted =
            tone === undefined ? this.#gutter(word) : this.tone(tone, this.#gutter(word));
        const detailed = detail === undefined ? "" : ` ${this.style("dim", detail)}`;

        return `${painted}  ${text}${detailed}`;
    }

    /** Write a question a person answers with yes or no, marked in the gutter. */
    question(text: string): string {
        return `${this.tone("accent", this.#gutter("?"))}  ${text} ${this.style("dim", "yes/no")}`;
    }

    /** Write the dot a list shows before a row, by how the row's thing stands. */
    dot(status: Status): string {
        const { glyph, tone } = DOTS[status];

        return tone === undefined ? this.style("dim", glyph) : this.tone(tone, glyph);
    }

    /** Write what a person may do or read next, dim under a fixed word such as next or trace. */
    hint(word: string, text: string): string {
        return this.style("dim", `${this.#gutter(word)}  ${text}`);
    }

    /** Write the heading of one object: its title, then its identifier dim. */
    heading(title: string, id: string): string {
        return `${this.style("bold", title)}  ${this.style("dim", id)}`;
    }

    /** Pad a line's one word into the gutter on a terminal, and leave it as it is for a program. */
    #gutter(word: string): string {
        return this.isTerminal ? word.padStart(GUTTER_WIDTH) : word;
    }

    /** Write a line of a command's output to standard output. */
    async print(text: string): Promise<void> {
        await this.process.stdout.write(`${text}\n`);
    }

    /** Write a line of a command's progress notes and failures to standard error. */
    async printDiagnostic(text: string): Promise<void> {
        await this.process.stderr.write(`${text}\n`);
    }

    /** Stop reading the answers on standard input. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.prompt[Symbol.asyncDispose]();
    }

    /** Start output following a changing value, as text or JSON. */
    live(json: boolean): Live {
        return new Live(this.process.stdout, json);
    }
}

/** Output following a changing value, redrawn in place on a terminal. */
export class Live {
    /** The standard output the renderings go to. */
    readonly #output: TerminalProcess["stdout"];
    /** Whether the renderings are JSON, appended one line per value. */
    readonly #isJson: boolean;
    /** The lines the last drawing took on the terminal. */
    #lines: number;

    /** Start with nothing drawn on a standard output, as text or JSON. */
    constructor(output: TerminalProcess["stdout"], json: boolean) {
        this.#output = output;
        this.#isJson = json;
        this.#lines = 0;
    }

    /** Show the latest rendering: text redrawn on a terminal, else appended line by line. */
    async show(text: string): Promise<void> {
        // redraw text in place on a terminal, erasing the previous rendering
        if (this.#output.isTerminal && !this.#isJson) {
            const erase = this.#lines === 0 ? "" : `\x1b[${this.#lines}F\x1b[J`;
            await this.#output.write(`${erase}${text}\n`);
            this.#lines = text.split("\n").length;
        }
        // append each rendering elsewhere, as NDJSON when the text is JSON
        else {
            await this.#output.write(`${text}\n`);
        }
    }

    /** Draw the next rendering below the last, such as after a question moved the cursor. */
    release(): void {
        this.#lines = 0;
    }
}

/** Write a foreground colour's escape parameters: 24-bit, else the nearest 256-colour one. */
function foreground(hex: string, isTruecolor: boolean): string {
    // read the channels
    const value = Number.parseInt(hex.slice(1), 16);
    const channels = [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
    if (isTruecolor) {
        return `38;2;${channels.join(";")}`;
    }

    // take each channel's nearest cube level
    const [red = 0, green = 0, blue = 0] = channels.map((channel) => {
        const distances = CUBE_LEVELS.map((level) => Math.abs(level - channel));

        return distances.indexOf(Math.min(...distances));
    });

    return `38;5;${String(CUBE_START + 36 * red + 6 * green + blue)}`;
}

/** Read a key's bytes as the key press a keyboard event reports. */
function pressOf(sequence: string): KeyPress {
    // read a named key, a control letter, or a character as typed
    const named = SEQUENCES[sequence];
    const byte = sequence.codePointAt(0) ?? 0;
    const isControl =
        sequence.length === 1 && byte >= CONTROL_A && byte <= CONTROL_Z && named === undefined;
    const key = named ?? (isControl ? String.fromCodePoint(byte - CONTROL_A + LETTER_A) : sequence);
    const isLetter = /^[a-z]$/iu.test(key);

    return {
        key,
        code: isLetter ? `Key${key.toUpperCase()}` : key,
        altKey: false,
        ctrlKey: isControl,
        metaKey: false,
        shiftKey: isLetter && key !== key.toLowerCase(),
    };
}
