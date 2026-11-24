import { toml } from "@codemirror/legacy-modes/mode/toml";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of TOML. */
export default new StreamGrammar(Language.of("toml"), toml);
