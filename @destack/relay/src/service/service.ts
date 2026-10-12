import { defineService } from "@destack/service";
import { connection, tunnel } from "../object/index.ts";

/** The relay's service. */
export const relayService = defineService("relay", { objects: { tunnel, connection } });
