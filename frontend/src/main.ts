/* eslint-disable no-console */

import { RetryLink } from "@apollo/client/link/retry";
import { GraphQLWsLink } from "@apollo/client/link/subscriptions";
import { createApp, h, provide } from "vue";
import { version } from "../../package.json";

import { WS_CONNECTED } from "@/utils/globals";
import { TYPE_POLICIES } from "@/utils/policies";
import { applyShortcuts } from "@/utils/shortcuts";
import { ApolloClient, HttpLink, InMemoryCache, split } from "@apollo/client/core";
import { getMainDefinition } from "@apollo/client/utilities";
import monacoLoader from "@monaco-editor/loader";
import { DefaultApolloClient } from "@vue/apollo-composable";
import { createClient } from "graphql-ws";
import { createPinia } from "pinia";
import { createMetaManager } from "vue-meta";
import App from "./App.vue";
import router from "./router";

const MAX_RETRY_TIME_MS = 5000;

function createApolloClient() {
  // split requests between http and ws
  // see https://www.apollographql.com/docs/react/data/subscriptions
  const httpLink = new HttpLink({
    uri: "http://localhost:8000/graphql",
  });
  const wsLink = new GraphQLWsLink(
    createClient({
      url: "ws://localhost:8000/graphql",
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
    })
  );
  const splitLink = split(
    ({ query }) => {
      const definition = getMainDefinition(query);
      return definition.kind === "OperationDefinition" && definition.operation === "subscription";
    },
    wsLink,
    httpLink
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

  app.use(router);
  app.use(pinia);
  app.use(createMetaManager());

  app.mount("#app");

  console.group(`%Bench Build Information`, "color:orangered"); // groupCollapsed

  if (import.meta.env.DEV) {
    console.info(`%cVersion: v${version}`, "color:orangered");
  }

  console.info(`%cEnvironment: ${import.meta.env.MODE}`, "color:orangered");
  console.groupEnd();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);

  // capture ctrl + s
  applyShortcuts();

  // init monaco once the app is mounted (for faster response if once a monaco editor is opened)
  monacoLoader.init();
}

init();
