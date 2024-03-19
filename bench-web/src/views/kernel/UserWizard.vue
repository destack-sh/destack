<script lang="tsx" setup>
import { Region, Variant, type NodeReferenceData, ViewData } from "@/proto/wire";
import { useLoadedGraph } from "@/system/connection";
import { makeIcon } from "@/system/icon";
import { removeView } from "@/system/space";
import { logIn, signUp, user } from "@/system/user";
import { viewEmits } from "@/views/common";
import PlainText from "@/views/content/PlainText.vue";
import Button from "@/views/controls/Button.vue";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { watch, ref, toRef, type Ref } from "vue";

const props = defineProps<{ self: NodeReferenceData } & {}>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));

const state: Ref<"sign-up" | "log-in" | "all-set"> = ref("log-in");
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.EUROPE_CENTRAL);
const isActive = ref(false);
const lastError = ref<RpcError | null>(null);

// sync
watch(
  user,
  () => {
    if (user.value) {
      state.value = "all-set";
    } else {
      state.value = "log-in";
    }
  },
  { immediate: true },
);

async function submit() {
  isActive.value = true;
  try {
    if (state.value == "sign-up") {
      await signUp({ name: name.value, slug: slug.value, email: email.value }, password.value);
    } else if (state.value == "log-in") {
      await logIn({ slug: slug.value }, password.value);
    } else {
      throw new Error(`unexpected registration state: ${state.value}`);
    }
  } catch (e) {
    lastError.value = e as RpcError;
  } finally {
    isActive.value = false;
  }
}

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div
    class="m-4 min-w-80 max-w-96 rounded-md border border-gray-300 bg-white px-9 py-7 text-gray-900 shadow-md shadow-gray-300"
  >
    <!-- Header -->
    <div>
      <h2 class="text-2xl font-semibold">
        <span v-if="state === 'log-in'">Log In</span>
        <span v-else-if="state === 'sign-up'">Sign Up</span>
        <span v-else-if="state === 'all-set'">All Set</span>
      </h2>
      <p class="mt-2 text-gray-500">
        <span v-if="state === 'log-in'">Log into an existing Bench account.</span>
        <span v-else-if="state === 'sign-up'">Create a new Bench account.</span>
        <span v-else-if="state === 'all-set'">You're already logged in.</span>
      </p>
    </div>
    <!-- Data -->
    <div v-if="state == 'sign-up' || state == 'log-in'" class="mt-5 flex w-full flex-col gap-y-3">
      <PlainText
        v-if="state === 'sign-up'"
        :icon="makeIcon({ name: 'fas fa-user' })"
        name="name"
        title="Name"
        is-input
        v-model="name"
      />
      <PlainText :icon="makeIcon({ name: 'fas fa-at' })" name="slug" title="Username" is-input v-model="slug" />
      <PlainText
        v-if="state === 'sign-up'"
        :icon="makeIcon({ name: 'fas fa-envelope' })"
        name="email"
        title="Email"
        is-input
        v-model="email"
      />
      <PlainText
        :icon="makeIcon({ name: 'fas fa-key' })"
        name="password"
        title="Password"
        is-input
        is-secret
        v-model="password"
      />
    </div>
    <!-- Actions -->
    <div class="mt-7">
      <Button
        v-if="state === 'log-in' || state === 'sign-up'"
        name="submit"
        :icon="makeIcon({ name: 'fas fa-arrow-right-from-bracket' })"
        :title="state === 'log-in' ? 'Log in' : 'Sign up'"
        class="w-full"
        @click="submit"
        :is-disabled="isActive"
        :is-loading="isActive"
      />
      <Button
        v-if="state === 'log-in' || state === 'sign-up'"
        name="switch"
        :icon="makeIcon({ name: 'fas fa-shuffle' })"
        :title="state === 'log-in' ? 'Sign up instead' : 'Log in instead'"
        class="mt-2 w-full"
        :variant="Variant.V3"
        @click="() => (state = state === 'log-in' ? 'sign-up' : 'log-in')"
      />
      <Button
        v-if="state == 'all-set'"
        name="close"
        :icon="makeIcon({ name: 'fas fa-xmark' })"
        :title="'Close'"
        class="w-full"
        :variant="Variant.V3"
        @click="() => removeView(spaceConnection.sideTx, spaceGraph, spaceGraph.get(self) as ViewData)"
      />
      <!-- Error -->
      <p v-if="lastError" class="mt-4 text-sm font-semibold text-danger-500">
        {{ lastError.code }}: {{ lastError.message }}
      </p>
    </div>
  </div>
</template>
