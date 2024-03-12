<script lang="tsx" setup>
import type { OperationError } from "@/proto/services";
import { Region, UserStatus, Variant, type NodeReferenceData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { logIn, signUp, user } from "@/system/user";
import { viewEmits } from "@/views/common";
import String from "@/views/content/String.vue";
import Button from "@/views/controls/Button.vue";
import type { RpcError } from "@protobuf-ts/runtime-rpc";
import { ref, toRef, watch, type Ref } from "vue";

const props = defineProps<{ self: NodeReferenceData } & {}>();
const emit = defineEmits(viewEmits());

const state: Ref<"sign-up" | "log-in" | "create-bench" | "all-set"> = ref("log-in");
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.EUROPE_CENTRAL);
const isActive = ref(false);
const lastError = ref<RpcError | null>(null);

// sync state with user status
watch(user, () => {
  if (user.value != null) {
    if (user.value.status == UserStatus.REGISTERED) {
      state.value = "create-bench";
    } else {
      state.value = "all-set";
    }
  } else {
    state.value = "log-in";
  }
});

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
    class="m-4 min-w-80 max-w-96 rounded-md border border-gray-300 bg-white px-10 py-8 text-gray-900 shadow-md shadow-gray-300"
  >
    <h2 class="text-2xl font-semibold">
      {{
        {
          "sign-up": "Sign up",
          "log-in": "Log in",
          "create-bench": "Create your Bench",
        }[state]
      }}
    </h2>
    <!-- Data -->
    <div class="mt-5 flex w-full flex-col gap-y-3">
      <String
        v-if="state === 'sign-up'"
        :icon="makeIcon({ name: 'fas fa-envelope' })"
        name="name"
        title="Name"
        is-input
        v-model="name"
      />
      <String
        v-if="state == 'sign-up' || state == 'log-in'"
        :icon="makeIcon({ name: 'fas fa-at' })"
        name="username"
        title="Username"
        is-input
        v-model="slug"
      />
      <String
        v-if="state === 'sign-up'"
        :icon="makeIcon({ name: 'fas fa-envelope' })"
        name="email"
        title="Email"
        is-input
        v-model="email"
      />
      <String
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
        :icon="makeIcon({ name: 'fas fa-arrow-right-from-bracket' })"
        name="submit"
        :title="state === 'log-in' ? 'Log in' : 'Sign up'"
        class="w-full"
        @click="submit"
        :is-disabled="isActive"
        :is-loading="isActive"
      />
      <Button
        v-if="state === 'log-in' || state === 'sign-up'"
        :icon="makeIcon({ name: 'fas fa-shuffle' })"
        name="switch"
        :title="state === 'log-in' ? 'Sign up instead' : 'Log in instead'"
        class="mt-2 w-full"
        :variant="Variant.V3"
        @click="() => (state = state === 'log-in' ? 'sign-up' : 'log-in')"
      />
      <p v-if="lastError" class="mt-4 text-sm font-semibold text-danger-500">
        {{ lastError.code }}: {{ lastError.message }}
      </p>
    </div>
  </div>
</template>
