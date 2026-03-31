import { dep, depLabel, depRoute } from "./dep.ts";
import { log } from "./shared/log.ts";
import { renderOverview } from "./shared/render.ts";

log("main2", renderOverview(dep, depLabel, depRoute));
