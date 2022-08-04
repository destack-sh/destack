import { useFlowsStore } from "@/stores";
import FlowDetail from "@/views/FlowDetail.vue";
import FlowEdit from "@/views/FlowEdit.vue";
import Flows from "@/views/Flows.vue";
import Home from "@/views/Home.vue";
import ModelCreate from "@/views/ModelCreate.vue";
import ModelDetail from "@/views/ModelDetail.vue";
import ModelEdit from "@/views/ModelEdit.vue";
import Models from "@/views/Models.vue";
import NotFound from "@/views/NotFound.vue";
import qs from "qs";
import { createRouter, createWebHistory, type RouteLocationNormalized } from "vue-router";

const forwardQuery = (route: RouteLocationNormalized) => route.query;
const forwardQueryAndParams = (route: RouteLocationNormalized) => ({
  ...route.query,
  ...route.params,
});

const routes = [
  { path: "/", component: Home },
  { path: "/models", component: Models },
  { path: "/models/new", component: ModelCreate },
  { path: "/models/:model", component: ModelDetail, props: forwardQueryAndParams },
  { path: "/models/:model/edit", component: ModelEdit, props: forwardQueryAndParams },
  {
    path: "/playground",
    name: "playground",
    component: FlowEdit,
    props: (route: RouteLocationNormalized) => {
      // playground is just a flow editor view with automatic init/recovery to last playground
      const flowsStore = useFlowsStore();
      let flowName = flowsStore.lastOpenedFlow?.flow;
      if (flowName == null || route.query.new) {
        flowName = flowsStore.newName();
      }

      return { flow: flowName, playground: true };
    },
  },
  { path: "/flows", component: Flows },
  { path: "/flows/:flow", component: FlowDetail, props: forwardQueryAndParams },
  { path: "/flows/:flow/edit", component: FlowEdit, props: forwardQueryAndParams },
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
