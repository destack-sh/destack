import { type Grammar, Language } from "@destack/code";
import bash from "@destack/code/grammar/bash";
import html from "@destack/code/grammar/html";
import javascript from "@destack/code/grammar/javascript";
import json from "@destack/code/grammar/json";
import rust from "@destack/code/grammar/rust";
import tsx from "@destack/code/grammar/tsx";
import typescript from "@destack/code/grammar/typescript";
import xml from "@destack/code/grammar/xml";

/** The grammars of the site's listings. */
const LISTED: readonly Grammar[] = [bash, html, javascript, json, rust, tsx, typescript, xml];

/** Find the grammar of a listing's language, absent for one not listed. */
export function grammarOf(tag: string): Grammar | undefined {
    const language = Language.ofTag(tag);

    return LISTED.find((each) => each.language === language);
}
