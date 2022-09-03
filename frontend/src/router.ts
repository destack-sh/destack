import DatasetDetail from "@/routes/DatasetDetail.vue";
import DatasetNew from "@/routes/DatasetNew.vue";
import Datasets from "@/routes/Datasets.vue";
import FlowDetail from "@/routes/FlowDetail.vue";
import FlowDetailEdit from "@/routes/FlowDetailEdit.vue";
import Flows from "@/routes/Flows.vue";
import Home from "@/routes/Home.vue";
import ModelDetail from "@/routes/ModelDetail.vue";
import ModelDetailEdit from "@/routes/ModelDetailEdit.vue";
import ModelNew from "@/routes/ModelNew.vue";
import Models from "@/routes/Models.vue";
import NotFound from "@/routes/NotFound.vue";
import { useFlowsStore } from "@/stores";
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
  { path: "/models/new", component: ModelNew },
  { path: "/models/:model", component: ModelDetail, props: forwardQueryAndParams },
  { path: "/models/:model/edit", component: ModelDetailEdit, props: forwardQueryAndParams },
  { path: "/datasets", component: Datasets, props: forwardQueryAndParams },
  { path: "/datasets/new", component: DatasetNew, props: forwardQueryAndParams },
  { path: "/datasets/:dataset", component: DatasetDetail, props: forwardQueryAndParams },
  // { path: "/datasets/:dataset/edit", component: ModelEdit, props: forwardQueryAndParams },
  {
    path: "/playground",
    name: "playground",
    redirect: (route: RouteLocationNormalized) => {
      // playground is just a flow editor view with automatic init/recovery to last playground
      const flowsStore = useFlowsStore();
      let flowName = flowsStore.lastOpenedFlow?.flow;
      if (flowName == null || route.query.new) {
        flowName = flowsStore.newName();
      }

      // unset query
      const query = { ...route.query };
      delete query.new;
      delete query.models;

      return {
        path: `/flows/${flowName}/edit`,
        query: { flow: flowName, playground: true, ...query },
      };
    },
  },
  { path: "/flows", component: Flows },
  { path: "/flows/:flow", component: FlowDetail, props: forwardQueryAndParams },
  { path: "/flows/:flow/edit", component: FlowDetailEdit, props: forwardQueryAndParams },
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
