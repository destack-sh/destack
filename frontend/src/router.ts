import NotFound from "@/routes/NotFound.vue";
import ProjectHome from "@/routes/ProjectHome.vue";

import qs from "qs";
import { createRouter, createWebHistory, type RouteLocationNormalized } from "vue-router";

const forwardQuery = (route: RouteLocationNormalized) => route.query;
const forwardQueryAndParams = (route: RouteLocationNormalized) => ({
  ...route.query,
  ...route.params,
});

const routes = [
  { path: "/:organization/:project", component: ProjectHome, props: forwardQueryAndParams },
  // catch all
  { path: "/:pathMatch(.*)*", name: "NotFound", component: NotFound },
];

// use custom query string decode to auto-coerce bools & numbers
// based on https://github.com/ljharb/qs/issues/91#issuecomment-864680091
function qsCoerceDecoder(str: string, decoder: any, charset: string) {
  const strWithoutPlus = str.replace(/\+/g, " ");
  if (charset === "iso-8859-1") {
    return strWithoutPlus.replace(/%[0-9a-f]{2}/gi, unescape);
  }

  if (/^(\d+|\d*\.\d+)$/.test(str)) {
    return parseFloat(str);
  }

  const keywords: Record<any, any> = {
    true: true,
    false: false,
    null: null,
    undefined,
  };
  if (str in keywords) {
    return keywords[str];
  }

  try {
    return decodeURIComponent(strWithoutPlus);
  } catch (e) {
    return strWithoutPlus;
  }
}

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: routes,
  // @ts-expect-error for some reason, parseQuery is incompatible, even though this is straight from the docs
  parseQuery: (search) => qs.parse(search, { decoder: qsCoerceDecoder }),
  stringifyQuery: qs.stringify,
});
export default router;
