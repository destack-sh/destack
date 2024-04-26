import { isOnMac } from "@/utils/browser";

/** String based keymap. '+' to combine, ' ' for chords. */
export type KeySignature = string;

// special 'mod' key :ModKey (ctrl on windows, cmd on mac)
const IS_ON_MAC = isOnMac(window);
export type KeyModifier = "shift" | "ctrl" | "alt" | "meta" | "mod"; // :ModifierOrder
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
    modifiers.sort((a, b) => KEYMAP_MODIFIERS.indexOf(a) - KEYMAP_MODIFIERS.indexOf(b)); // :ModifierOrder
    return { key: key, modifiers: modifiers };
  });
  if (chords.length == 0) throw new Error(`empty keymap signature ${signature}`);
  return { chords };
}

/** Renders a parsed keymap key back into a nice string signature */
export function renderKeymapSignature(key: ParsedKeySignature): KeySignature {
  return key.chords.map(renderChord).join(" ");
}

/** Renders a single keymap chord back into a nice string signature (partial) */
export function renderChord(chord: KeyChord): string {
  return (chord.modifiers as string[]).concat(chord.key).map(normalizeKeymapKey).join("+");
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

const KEY_ALIAS: Record<string, string> = {
  " ": "space",
  arrowup: "up",
  arrowdown: "down",
  arrowleft: "left",
  arrowright: "right",
};

export function normalizeKeymapKey(key: string) {
  if (key == "mod") return IS_ON_MAC ? "meta" : "ctrl";
  else return key;
}

function getEventModifiers(e: KeyboardEvent): KeyModifier[] {
  const modifiers: KeyModifier[] = [];
  // must be in :ModifierOrder
  if (e.shiftKey) modifiers.push("shift");
  if (e.altKey) modifiers.push("alt");
  if (e.ctrlKey) modifiers.push("ctrl");
  if (e.metaKey) modifiers.push("meta");
  return modifiers;
}

type KeymapBinding = {
  signature: KeySignature;
  parsedSignature: ParsedKeySignature;
  callback: KeymapCallback;
  opaque?: unknown;
};

/**
 * Simple key trap that fires callbacks on keymap signatures.
 * Handles chords and all the funky stuff. A key signature must be unique.
 */
export class Keytrap {
  private bindings: Record<KeySignature, KeymapBinding> = {};
  private bindingsByChord: Record<string, KeymapBinding> = {};

  private onKeyDown(e: Event) {
    if (!(e instanceof KeyboardEvent)) return;

    // get binding by chord
    let mainKey = e.key.toLowerCase();
    mainKey = KEY_ALIAS[mainKey] ?? mainKey;
    if (!CHAR_KEYS[mainKey] || KEYMAP_MODIFIERS.includes(mainKey as any)) return;
    const modifiers = getEventModifiers(e);
    const chord = renderChord({ key: mainKey, modifiers });
    const binding = this.bindingsByChord[chord];

    // fire callback (assumes :SingleChord)
    if (binding != null) {
      if (binding.callback(e, binding.signature)) {
        e.preventDefault();
        e.stopPropagation();
      }
    }
  }

  /** Tracks events in the given element */
  track(element: HTMLElement | Document) {
    element.addEventListener("keydown", this.onKeyDown.bind(this));
  }

  /** Binds the given key signature uniquely to some callback */
  bind(
    signature: KeySignature | ParsedKeySignature | Array<KeySignature | ParsedKeySignature>,
    callback: KeymapCallback,
    options?: { key?: string; replace?: boolean },
  ): () => void {
    const signatures = Array.isArray(signature) ? signature : [signature];
    const parsedSignatures: ParsedKeySignature[] = [];
    for (const signature of signatures) {
      // normalize
      const parsed = typeof signature == "string" ? parseKeymapSignature(signature) : signature;
      const binding = {
        signature: renderKeymapSignature(parsed),
        parsedSignature: parsed,
        callback,
        opaque: options?.key,
      };
      if (parsed.chords.length != 1) throw new Error("only :SingleChord is supported for now");
      parsedSignatures.push(parsed);

      // bind
      const existing = this.bindings[binding.signature];
      if (existing) {
        if (options?.replace) {
          delete this.bindings[binding.signature];
          delete this.bindingsByChord[renderChord(parsed.chords[0])]; // :SingleChord
        } else {
          throw new Error(
            `${binding.signature} already bound: ${existing.opaque ?? existing.callback} vs ${options?.key ?? callback}`,
          );
        }
      }
      this.bindings[binding.signature] = binding;
      this.bindingsByChord[renderChord(parsed.chords[0])] = binding; // :SingleChord
    }
    return () => this.unbind(parsedSignatures);
  }

  unbind(signature: KeySignature | ParsedKeySignature | Array<KeySignature | ParsedKeySignature>) {
    const signatures = Array.isArray(signature) ? signature : [signature];
    for (const signature of signatures) {
      // normalize
      const parsed = typeof signature == "string" ? parseKeymapSignature(signature) : signature;
      const key = renderKeymapSignature(parsed);

      // unbind
      const binding = this.bindings[key];
      if (binding != null) {
        delete this.bindings[key];
        delete this.bindingsByChord[renderChord(parsed.chords[0])]; // :SingleChord
      }
    }
  }
}

// global key tracker
export const keytrap = new Keytrap();
