import { isOnMac } from "@/utils/browser";
import { reverseRecord } from "@/utils/functools";

/** String based keymap. '+' to combine, ' ' for chords. */
export type KeySignature = string;

// special 'mod' key :ModKey (ctrl on windows, cmd on mac)
const IS_ON_MAC = isOnMac(window);
export type KeyModifier = "shift" | "ctrl" | "alt" | "meta" | "mod";
export const KEYMAP_MODIFIERS: KeyModifier[] = ["shift", "ctrl", "alt", "meta", "mod"];

/** A parsed 'key' in our keymap. */
export type ParsedKeySignature = {
  chords: KeyChord[]; // only :SingleChord for now
};

/** Combination of key and modifiers */
export type KeyChord = {
  // The main key. Must not be a modifier.
  key: string;
  modifiers: KeyModifier[];
};

/**
 * Parses a string-based keymap signature into a proper key.
 * Case is irrelevant. Throws if invalid.
 *  */
export function parseKeymapKey(signature: KeySignature): ParsedKeySignature {
  signature = signature.toLowerCase();
  // parse out individual chords
  const chords = signature.split(" ").map((chord) => {
    let key: string | null = null;
    const modifiers: KeyModifier[] = [];
    for (const k of chord.split("+")) {
      if (KEYMAP_MODIFIERS.includes(k as KeyModifier)) {
        // modifier
        if (modifiers.includes(k as KeyModifier))
          throw new Error(`duplicate modifier ${k} in chord ${chord} of ${signature}`);
        modifiers.push(k as KeyModifier);
      } else {
        // main key
        if (key) throw new Error(`multiple keys in chord ${chord} of ${signature}`);
        if (!CHAR_KEYS[k]) throw new Error(`invalid key ${k} in chord ${chord} of ${signature}`);
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
export function renderKeymapKey(key: ParsedKeySignature): KeySignature {
  const chords = key.chords.map((chord) => {
    const keys = (chord.modifiers as string[]).concat(chord.key);
    return keys.join("+");
  });
  return chords.join(" ");
}

export type KeymapCallback = (e: KeyboardEvent, combination: KeySignature) => boolean | void;

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
// :ModKey
if (IS_ON_MAC) CHAR_KEYS["mod"] = CHAR_KEYS["meta"];
else CHAR_KEYS["mod"] = CHAR_KEYS["ctrl"];

function getEventModifiers(e: KeyboardEvent): KeyModifier[] {
  const modifiers: KeyModifier[] = [];
  if (e.shiftKey) modifiers.push("shift");
  if (e.altKey) modifiers.push("alt");
  if (e.ctrlKey) {
    modifiers.push("ctrl");
    if (!IS_ON_MAC) modifiers.push("mod");
  }
  if (e.metaKey) {
    modifiers.push("meta");
    if (IS_ON_MAC) modifiers.push("mod");
  }
  return modifiers;
}

function modifiersMatch(e: KeyboardEvent, modifiers: KeyModifier[]) {
  if (e.shiftKey != modifiers.includes("shift")) return false;
  if (e.altKey != modifiers.includes("alt")) return false;
  if (e.ctrlKey != (modifiers.includes("ctrl") || !IS_ON_MAC && modifiers.includes("mod"))) return false;
  if (e.metaKey != (modifiers.includes("meta") || IS_ON_MAC && modifiers.includes("mod"))) return false;
  return true;
}

type KeymapBinding = {
  signature: KeySignature;
  parsedSignature: ParsedKeySignature;
  callback: KeymapCallback;
};

/**
 * Simple key trap that fires callbacks based on keymap keys.
 * Handles chords and all the funky stuff.
 */
export class Keytrap {
  private bindings: Record<KeySignature, KeymapBinding[]> = {};
  private bindingsByKey: Record<string, KeymapBinding[]> = {};

  private onKeyDown(e: Event) {
    if (!(e instanceof KeyboardEvent)) return;

    // prefilter by main key
    const mainKey = e.key.toLowerCase();
    if (!CHAR_KEYS[mainKey] || KEYMAP_MODIFIERS.includes(mainKey as any)) return;
    const candidateBindings = this.bindingsByKey[mainKey];
    if (!candidateBindings) return;

    // match (:SingleChord only for now)
    for (const binding of candidateBindings) {
      const { parsedSignature, callback } = binding;
      const { chords } = parsedSignature;
      const lastChord = chords[chords.length - 1];
      if (lastChord.key != mainKey || !modifiersMatch(e, lastChord.modifiers)) continue;

      // fire callback
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

  clear() {
    this.bindings = {};
    this.bindingsByKey = {};
  }

  bind(
    signature: KeySignature | ParsedKeySignature | Array<KeySignature | ParsedKeySignature>,
    callback: KeymapCallback,
  ): () => void {
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
    return () => this.unbind(signature, callback);
  }

  unbind(
    signature: KeySignature | ParsedKeySignature | Array<KeySignature | ParsedKeySignature>,
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
