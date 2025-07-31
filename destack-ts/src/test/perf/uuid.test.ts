import { uuid7 } from "@destack/utils/uuid";
import { bench, do_not_optimize, run } from "mitata";

bench("uuid-v7", () => {
  do_not_optimize(uuid7());
});

await run();
