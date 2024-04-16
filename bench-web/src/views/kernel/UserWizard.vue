<script lang="tsx" setup>
import { NodeType, Region, Variant, ViewData, type NodeReferenceData } from "@/proto/wire";
import { benchPtr } from "@/system/client";
import { useExistingConnection } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { canvas, goToBench } from "@/system/space";
import { logIn, signUp, user } from "@/system/user";
import { makeTypeInfo } from "@/system/value";
import { reverseRecord } from "@/utils/functools";
import { getViewComponentChildren, isVueInstanceOf } from "@/views/canvas";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import PlainText from "@/views/content/PlainText.vue";
import Button from "@/views/controls/Button.vue";
import { ref, toRef, watch, type Ref } from "vue";

const props = defineProps<{ self: NodeReferenceData } & Pick<ViewData, "title">>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(toRef(props, "self"));

type State = "sign-up" | "log-in" | "all-set";
const TITLE_BY_STATE: Record<State, string> = {
  "sign-up": "Sign Up",
  "log-in": "Log In",
  "all-set": "All Set",
};
const STATE_BY_TITLE = reverseRecord(TITLE_BY_STATE);

// determine initial state from title (not great but we only use this internally)
const state: Ref<State> = ref(STATE_BY_TITLE[props.title ?? ""] ?? "log-in");
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.EUROPE_CENTRAL);
const isActive = ref(false);

function setState(newState: State) {
  if (newState == state.value) return;
  state.value = newState;
  spaceConnection.tx.update(spaceGraph.getOrFail(self.value) as ViewData, { title: TITLE_BY_STATE[newState] });
  canvas.focusInComponent(self.value);
}

// sync state from user & title
watch(
  [user, () => props.title],
  () => {
    if (user.value) {
      setState("all-set");
    } else if (state.value == "all-set") {
      setState("log-in");
    } else if (STATE_BY_TITLE[props.title ?? ""] != null) {
      setState(STATE_BY_TITLE[props.title ?? ""]!);
    }
  },
  { immediate: true },
);

function clear() {
  name.value = "";
  slug.value = "";
  email.value = "";
  password.value = "";
}

async function submit() {
  isActive.value = true;
  try {
    if (state.value == "sign-up") {
      await signUp({ name: name.value, slug: slug.value, email: email.value }, password.value);
    } else if (state.value == "log-in") {
      const { user } = await logIn({ slug: slug.value }, password.value);
      // if we're outside a Bench and have a Bench, go home
      if (user.mainBenchPtr != null && benchPtr.value == null) {
        await goToBench({ bench: user.mainBenchPtr });
      }
    } else {
      throw new Error(`unexpected registration state: ${state.value}`);
    }
    clear();
  } finally {
    isActive.value = false;
  }
}

const instance = canvas.registerView(self);
function focus(anchor: FocusAnchor | NodeReferenceData) {
  const childViews = getViewComponentChildren(instance);
  if (anchor != "bottom") {
    return childViews.find((v) => isVueInstanceOf(v, PlainText));
  } else {
    return childViews.reverse().find((v) => isVueInstanceOf(v, Button));
  }
}

defineExpose<ViewExposed>({ self, focus });
</script>
<template>
  <div
    class="mx-auto mt-24 h-fit min-w-80 max-w-96 rounded-md border border-gray-300 bg-white px-9 py-7 text-gray-900 shadow-md shadow-gray-300"
  >
    <!-- Header -->
    <div>
      <h2 class="text-2xl font-semibold">
        {{ title }}
      </h2>
      <p class="mt-2 text-gray-500">
        <span v-if="state === 'log-in'">Log into an existing Bench account.</span>
        <span v-else-if="state === 'sign-up'">Create a new Bench account.</span>
        <span v-else-if="state === 'all-set'">You're logged in and good to go.</span>
      </p>
    </div>
    <!-- Data -->
    <div v-if="state == 'sign-up' || state == 'log-in'" class="mt-5 flex w-full flex-col gap-y-3">
      <PlainText
        v-if="state === 'sign-up'"
        :icon="makeIcon({ faName: 'fas fa-user' })"
        name="Name"
        title="Name"
        is-input
        v-model="name"
      />
      <PlainText :icon="makeIcon({ faName: 'fas fa-at' })" name="slug" title="Username" is-input v-model="slug" />
      <PlainText
        v-if="state === 'sign-up'"
        :icon="makeIcon({ faName: 'fas fa-envelope' })"
        name="Email"
        title="Email"
        is-input
        v-model="email"
      />
      <PlainText
        :icon="makeIcon({ faName: 'fas fa-key' })"
        name="Password"
        title="Password"
        is-input
        :value-type="makeTypeInfo({ isSecret: true })"
        v-model="password"
      />
    </div>
    <!-- Actions -->
    <div class="mt-7">
      <Button
        v-if="state === 'log-in' || state === 'sign-up'"
        name="Submit"
        :icon="makeIcon({ faName: 'fas fa-arrow-right-from-bracket' })"
        :title="state === 'log-in' ? 'Log in' : 'Sign up'"
        class="w-full"
        @click="submit"
        :is-disabled="isActive"
        :is-loading="isActive"
      />
      <Button
        v-if="state === 'log-in' || state === 'sign-up'"
        name="Switch"
        :icon="makeIcon({ faName: 'fas fa-shuffle' })"
        :title="state === 'log-in' ? 'Sign up instead' : 'Log in instead'"
        class="mt-2 w-full"
        :variant="Variant.ALTERNATE"
        @click="() => setState(state == 'log-in' ? 'sign-up' : 'log-in')"
      />
      <Button
        v-if="state == 'all-set'"
        name="Close"
        :icon="makeIcon({ faName: 'fas fa-xmark' })"
        :title="'Close'"
        class="w-full"
        :variant="Variant.ALTERNATE"
        @click="() => canvas.removeView(spaceConnection.tx, spaceGraph, spaceGraph.get(self) as ViewData)"
      />
    </div>
  </div>
</template>
