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
export function parseKeymapSignature(signature: KeySignature): ParsedKeySignature {
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
export function renderKeymapSignature(key: ParsedKeySignature): KeySignature {
  const chords = key.chords.map((chord) => {
    const keys = (chord.modifiers as string[]).concat(chord.key);
    return keys.map(renderKeymapKey).join("+");
  });
  return chords.join(" ");
}
export function renderKeymapKey(key: string) {
  if (key == "mod") return IS_ON_MAC ? "cmd" : "ctrl";
  else return key;
}

export type KeymapCallback = (e: KeyboardEvent, combination: KeySignature) => boolean | void;

export const CHAR_KEYS: Record<string, number> = {
  backspace: 8,
  tab: 9,
  enter: 13,
  shift: 16,
  ctrl: 17,
  alt: 18,
  capslock: 20,
  esc: 27,
  space: 32,
  pageup: 33,
  pagedown: 34,
  end: 35,
  home: 36,
  left: 37,
  up: 38,
  right: 39,
  down: 40,
  ins: 45,
  del: 46,
};
// a-z (lowercase only)
for (let i = 0; i < 26; ++i) {
  CHAR_KEYS[String.fromCharCode(97 + i)] = 65 + i; // a-z to A-Z keycode
}
// 0-9 (top row numbers)
for (let i = 0; i < 10; ++i) {
  CHAR_KEYS[i.toString()] = 48 + i; // numbers as strings to keycodes
}
// special characters `,-,=,[,],\,;,' ,, ., /
for (const key of "`-=[]\\;',./") {
  CHAR_KEYS[key] = key.charCodeAt(0);
}
// F keys (F1 to F19)
for (let i = 1; i <= 19; ++i) {
  CHAR_KEYS["f" + i] = 112 + i - 1;
}
// numpad 0-9
for (let i = 0; i <= 9; ++i) {
  CHAR_KEYS["numpad" + i.toString()] = 96 + i; // numpad numbers as strings to keycodes
}

// :ModKey
if (IS_ON_MAC) CHAR_KEYS["mod"] = CHAR_KEYS["meta"];
else CHAR_KEYS["mod"] = CHAR_KEYS["ctrl"];

function modifiersMatch(e: KeyboardEvent, modifiers: KeyModifier[]) {
  if (e.shiftKey != modifiers.includes("shift")) return false;
  if (e.altKey != modifiers.includes("alt")) return false;
  if (e.ctrlKey != (modifiers.includes("ctrl") || (!IS_ON_MAC && modifiers.includes("mod")))) return false;
  if (e.metaKey != (modifiers.includes("meta") || (IS_ON_MAC && modifiers.includes("mod")))) return false;
  return true;
}

type KeymapBinding = {
  signature: KeySignature;
  parsedSignature: ParsedKeySignature;
  callback: KeymapCallback;
};

/**
 * Simple key trap that fires callbacks on keymap signatures.
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
      const parsedSignature = typeof signature == "string" ? parseKeymapSignature(signature) : signature;
      const key = renderKeymapSignature(parsedSignature);
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
      const parsedSignature = typeof signature == "string" ? parseKeymapSignature(signature) : signature;
      const key = renderKeymapSignature(parsedSignature);

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
