import { defineService } from "@destack/service";

/** The relay's service: its copies of the rows names resolve with, while machines reach it through its runtime's WebSockets. */
export const relayService = defineService("relay", {});
