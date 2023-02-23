import NotFound from "@/routes/NotFound.vue";
import ProjectBench from "@/routes/ProjectBench.vue";
import UserHome from "@/routes/UserHome.vue";
import CreateProject from "@/routes/CreateProject.vue";
import Signup from "@/routes/Signup.vue";
import CompleteSignup from "@/routes/CompleteSignup.vue";
import Login from "@/routes/Login.vue";

import qs from "qs";
import { createRouter, createWebHistory, type RouteLocationNormalized } from "vue-router";

const forwardQueryAndParams = (route: RouteLocationNormalized) => ({
  ...route.query,
  ...route.params,
});

const routes = [
  { path: "/", name: "Home", component: UserHome, props: forwardQueryAndParams },
  { path: "/signup", name: "Signup", component: Signup, props: forwardQueryAndParams },
  { path: "/signup/complete", name: "CompleteSignup", component: CompleteSignup, props: forwardQueryAndParams },
  { path: "/login", name: "Login", component: Login, props: forwardQueryAndParams },
  { path: "/settings/profile", name: "SettingsProfile", component: NotFound, props: forwardQueryAndParams },
  { path: "/new", name: "CreateProject", component: CreateProject, props: forwardQueryAndParams },
  { path: "/:owner", name: "Profile", component: NotFound, props: forwardQueryAndParams },
  { path: "/:owner/:project", component: ProjectBench, props: forwardQueryAndParams },
  // catch all
  { path: "/:pathMatch(.*)*", name: "NotFound", component: NotFound },
];

// use custom query string decode to auto-coerce bools & numbers
// based on https://github.com/ljharb/qs/issues/91#issuecomment-864680091
function qsCoerceDecoder(str: string, decoder: never, charset: string) {
  const strWithoutPlus = str.replace(/\+/g, " ");
  if (charset === "iso-8859-1") {
    return strWithoutPlus.replace(/%[0-9a-f]{2}/gi, unescape);
  }

  if (/^(\d+|\d*\.\d+)$/.test(str)) {
    return parseFloat(str);
  }

  const keywords: Record<string, true | false | null | undefined> = {
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
