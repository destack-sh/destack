import { parser } from "@lezer/rust";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of Rust. */
export default new LezerGrammar(Language.of("rust"), parser);
