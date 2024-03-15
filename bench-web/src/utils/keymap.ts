import { reverseRecord } from "@/utils/functools";

/** String based keymap. '+' to combine, ' ' for chords. */
export type KeymapSignature = string;

export type KeymapModifier = "shift" | "ctrl" | "alt" | "meta";
export const KEYMAP_MODIFIERS: KeymapModifier[] = ["shift", "ctrl", "alt", "meta"];

/** A parsed 'key' in our keymap. */
export type ParsedKeymapSignature = {
  chords: KeymapChord[]; // only :SingleChord for now
};

/** Combination of key and modifiers */
export type KeymapChord = {
  // The main key. Must not be a modifier.
  key: string;
  modifiers: KeymapModifier[];
};

/**
 * Parses a string-based keymap signature into a proper key.
 * Case is irrelevant. Throws if invalid.
 *  */
export function parseKeymapKey(signature: KeymapSignature): ParsedKeymapSignature {
  signature = signature.toLowerCase();
  const chords = signature.split(" ").map((chord) => {
    const keys = chord.split("+");
    // figure out the main key
    let key: string | null = null;
    const modifiers: KeymapModifier[] = [];
    for (const k of keys) {
      if (KEYMAP_MODIFIERS.includes(k as KeymapModifier)) {
        if (modifiers.includes(k as KeymapModifier))
          throw new Error(`duplicate modifier ${k} in chord ${chord} of ${signature}`);
        modifiers.push(k as KeymapModifier);
      } else {
        if (key) throw new Error(`multiple keys in chord ${chord} of ${signature}`);
        if (!CHAR_KEYS_BY_CODE[k.charCodeAt(0)]) throw new Error(`invalid key ${k} in chord ${chord} of ${signature}`);
        key = k;
      }
    }
    if (!key) throw new Error(`missing key in chord ${chord} of ${signature}`);
    return { key: key, modifiers: modifiers };
  });
  if (chords.length == 0) throw new Error(`empty keymap signature ${signature}`);
  return { chords };
}

/** Renders a parsed keymap key back into a nice string signature */
export function renderKeymapKey(key: ParsedKeymapSignature): KeymapSignature {
  const chords = key.chords.map((chord) => {
    const keys = (chord.modifiers as string[]).concat(chord.key);
    return keys.join("+");
  });
  return chords.join(" ");
}

export type KeymapCallback = (e: KeyboardEvent, combination: KeymapSignature) => boolean | void;

/**
 * Regular character keycodes. Much like in VSCode.
 */
const CHAR_KEYS_BY_CODE: { [key: number]: string } = {
  8: "backspace",
  9: "tab",
  13: "enter",
  16: "shift",
  17: "ctrl",
  18: "alt",
  20: "capslock",
  27: "esc",
  32: "space",
  33: "pageup",
  34: "pagedown",
  35: "end",
  36: "home",
  37: "left",
  38: "up",
  39: "right",
  40: "down",
  45: "ins",
  46: "del",
  91: "meta",
  93: "meta",
  224: "meta",
};
// a-z (lowercase only)
for (let i = 0; i < 26; ++i) {
  CHAR_KEYS_BY_CODE[65 + i] = String.fromCharCode(97 + i);
}
// 0-9
for (let i = 0; i < 10; ++i) {
  CHAR_KEYS_BY_CODE[48 + i] = i.toString();
}
// `, -, =, [, ], \, ;, ', ,, ., /
for (const key of "`-=[]\\;',./") {
  CHAR_KEYS_BY_CODE[key.charCodeAt(0)] = key;
}
// F keys
for (let i = 1; i < 20; ++i) {
  CHAR_KEYS_BY_CODE[111 + i] = "f" + i;
}
// numpad keys
for (let i = 0; i <= 9; ++i) {
  CHAR_KEYS_BY_CODE[i + 96] = i.toString();
}
export const CHAR_KEYS = reverseRecord(CHAR_KEYS_BY_CODE);

function getEventModifiers(e: KeyboardEvent): KeymapModifier[] {
  const modifiers: KeymapModifier[] = [];
  if (e.shiftKey) modifiers.push("shift");
  if (e.altKey) modifiers.push("alt");
  if (e.ctrlKey) modifiers.push("ctrl");
  if (e.metaKey) modifiers.push("meta");
  return modifiers;
}

type KeymapBinding = {
  signature: KeymapSignature;
  parsedSignature: ParsedKeymapSignature;
  callback: KeymapCallback;
};
/**
 * Simple key trap that fires callbacks based on keymap keys.
 * Handles chords and all the funky stuff.
 */
export class Keytrap {
  private bindings: Record<KeymapSignature, KeymapBinding[]> = {};
  private bindingsByKey: Record<string, KeymapBinding[]> = {};

  private onKeyDown(e: Event) {
    if (!(e instanceof KeyboardEvent)) return;

    // prefilter
    const modifiers = getEventModifiers(e);
    const mainKey = e.key.toLowerCase();
    if (!CHAR_KEYS[mainKey] || KEYMAP_MODIFIERS.includes(mainKey as any)) return;
    const candidateBindings = this.bindingsByKey[mainKey];
    if (!candidateBindings) return;

    // match (:SingleChord only for now)
    for (const binding of candidateBindings) {
      const { parsedSignature, callback } = binding;
      const { chords } = parsedSignature;
      const lastChord = chords[chords.length - 1];
      if (lastChord.modifiers.length != modifiers.length) continue;
      if (lastChord.modifiers.some((m) => !modifiers.includes(m))) continue;

      // fire
      if (callback(e, binding.signature)) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }
    }
  }

  track(element: HTMLElement | Document) {
    element.addEventListener("keydown", this.onKeyDown.bind(this));
  }

  bind(
    signature: KeymapSignature | ParsedKeymapSignature | Array<KeymapSignature | ParsedKeymapSignature>,
    callback: KeymapCallback,
  ) {
    const signatures = Array.isArray(signature) ? signature : [signature];
    for (const signature of signatures) {
      // normalize
      const parsedSignature = typeof signature == "string" ? parseKeymapKey(signature) : signature;
      const key = renderKeymapKey(parsedSignature);
      const binding = { signature: key, parsedSignature, callback };

      // bind
      if (!this.bindings[key]) this.bindings[key] = [];
      this.bindings[key].push(binding);
      for (const chord of parsedSignature.chords) {
        const key = chord.key;
        if (!this.bindingsByKey[key]) this.bindingsByKey[key] = [];
        this.bindingsByKey[key].push(binding);
      }
    }
  }

  unbind(
    signature: KeymapSignature | ParsedKeymapSignature | Array<KeymapSignature | ParsedKeymapSignature>,
    callback: KeymapCallback,
  ) {
    const signatures = Array.isArray(signature) ? signature : [signature];
    for (const signature of signatures) {
      // normalize
      const parsedSignature = typeof signature == "string" ? parseKeymapKey(signature) : signature;
      const key = renderKeymapKey(parsedSignature);

      // unbind
      const bindings = this.bindings[key];
      if (!bindings) return;
      const index = bindings.findIndex((b) => b.callback == callback);
      if (index >= 0) bindings.splice(index, 1);
      if (bindings.length == 0) delete this.bindings[key];
    }
  }
}

// global key tracker
export const keytrap = new Keytrap();
