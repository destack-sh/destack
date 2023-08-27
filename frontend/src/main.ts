/* eslint-disable no-console */

import { RetryLink } from "@apollo/client/link/retry";
import { GraphQLWsLink } from "@apollo/client/link/subscriptions";
import { createApp, h, provide, watch } from "vue";

import {
  ACTIVE_SHARING_TOKEN,
  API_BASE_URL,
  COMMIT,
  HTTP_API_BASE_URL,
  IS_LOCALHOST,
  VERSION,
  WS_API_BASE_URL,
  WS_CONNECTED,
} from "@/utils/globals";
import { TYPE_POLICIES } from "@/utils/policies";
import { applyShortcuts } from "@/utils/shortcuts";
import { ApolloClient, ApolloLink, HttpLink, InMemoryCache, split } from "@apollo/client/core";
import { getMainDefinition } from "@apollo/client/utilities";
import monacoLoader from "@monaco-editor/loader";
import { BrowserTracing } from "@sentry/tracing";
import * as Sentry from "@sentry/vue";
import { DefaultApolloClient } from "@vue/apollo-composable";
import { createClient } from "graphql-ws";
import { createPinia } from "pinia";
import posthog from "posthog-js";
import { createMetaManager } from "vue-meta";
import App from "./App.vue";
import router from "./router";
import { CLIENT_NONCE } from "@/state/client";

const MAX_RETRY_TIME_MS = 10000;
function createApolloClient() {
  // split requests between http and ws
  // see https://www.apollographql.com/docs/react/data/subscriptions
  const httpLink = new HttpLink({
    uri: `${HTTP_API_BASE_URL}/graphql`,
    credentials: "include",
    headers: {
      "X-Client-Nonce": CLIENT_NONCE,
    },
  });
  const wsLink = new GraphQLWsLink(
    createClient({
      url: `${WS_API_BASE_URL}/graphql`,
      retryAttempts: Infinity,
      shouldRetry: () => true,
      // websocket retry, backoff from 1 to 5s
      retryWait: async (retries: number) => {
        const timeout = Math.min(MAX_RETRY_TIME_MS, 1000 + retries * 1000);
        const jitter = Math.random() * 1000;
        await new Promise((r) => setTimeout(r, timeout + jitter));
      },
      on: {
        connected: () => (WS_CONNECTED.value = true),
        closed: () => (WS_CONNECTED.value = false),
      },
      connectionParams: () => ({
        headers: {
          "X-Client-Nonce": CLIENT_NONCE,
          ...(ACTIVE_SHARING_TOKEN.value ? { "X-Sharing-Token": ACTIVE_SHARING_TOKEN.value } : {}),
        },
      }),
    })
  );
  // set the active sharing token as X-Sharing-Token header (if set)
  const httpAuthLink = new ApolloLink((operation, forward) => {
    operation.setContext(({ headers = {} }) => ({
      headers: {
        ...headers,
        ...(ACTIVE_SHARING_TOKEN.value ? { "X-Sharing-Token": ACTIVE_SHARING_TOKEN.value } : {}),
      },
    }));
    return forward(operation);
  });

  const splitLink = split(
    ({ query }) => {
      const definition = getMainDefinition(query);
      return definition.kind === "OperationDefinition" && definition.operation === "subscription";
    },
    wsLink,
    httpAuthLink.concat(httpLink)
  );

  // auto-retry requests when failed due to network errors
  const retryLink = new RetryLink({
    delay: { initial: 300, max: MAX_RETRY_TIME_MS, jitter: true },
    attempts: { max: 5, retryIf: (error) => !!error },
  });

  return new ApolloClient({
    link: retryLink.concat(splitLink),
    cache: new InMemoryCache({ typePolicies: TYPE_POLICIES }),
  });
}

async function init() {
  console.info("Starting Bench...");
  const apolloClient = createApolloClient();
  const pinia = createPinia();

  const app = createApp({
    setup() {
      provide(DefaultApolloClient, apolloClient);
    },
    render: () => h(App),
  });

  // only set in staging/prod
  if (!IS_LOCALHOST) {
    console.info("Setting up Sentry...", import.meta.env.VITE_APP_SENTRY_DSN != null);
    Sentry.init({
      app,
      dsn: import.meta.env.VITE_APP_SENTRY_DSN,
      integrations: [
        new BrowserTracing({
          routingInstrumentation: Sentry.vueRouterInstrumentation(router),
          tracePropagationTargets: ["localhost", "127.0.0.1", "api.bench.is", /^\//],
        }),
      ],
      tracesSampleRate: 1.0,
      logErrors: true,
    });
  }

  // this is the public key, it's fine to put it here
  // always init posthog (even in dev) since it errors otherwise
  posthog.init("phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma", {
    api_host: "https://eu.posthog.com",
    enable_recording_console_log: true,
  });
  if (IS_LOCALHOST) {
    posthog.opt_out_capturing();
  } else {
    console.info("Setting up Posthog...");
    posthog.opt_in_capturing();
  }

  app.use(router);
  app.use(pinia);
  app.use(createMetaManager());

  app.mount("#app");

  console.group(`%cBench Build`, "color:orangered");
  console.info(`%cVersion: ${VERSION} (${COMMIT})`, "color:orangered");

  console.info(`%cAPI: ${API_BASE_URL}`, "color:orangered");
  console.info(`%cEnvironment: ${import.meta.env.MODE}`, "color:orangered");
  console.groupEnd();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);

  // handle shortcuts
  applyShortcuts();

  // init monaco once the app is mounted (for faster response on first monaco editor open)
  monacoLoader.init();
}

init();
