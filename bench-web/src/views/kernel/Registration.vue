<script lang="tsx" setup>
import { Variant, type NodeReferenceData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { logIn } from "@/system/user";
import { viewEmits } from "@/views/common";
import String from "@/views/content/String.vue";
import Button from "@/views/controls/Button.vue";
import { ref, toRef, type Ref } from "vue";

const props = defineProps<{ self: NodeReferenceData } & {}>();
const emit = defineEmits(viewEmits());

const state: Ref<"sign-up" | "log-in"> = ref("log-in");
const name: Ref<string> = ref("");
const username: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const isActive = ref(false);

defineExpose({ self: toRef(props, "self") });

async function submit() {
  isActive.value = true;
  try {
    if (state.value == "sign-up") {
      throw new Error("not implemented");
    } else if (state.value == "log-in") {
      await logIn({ username: username.value }, password.value);
    } else {
      throw new Error(`unexpected registration state: ${state.value}`);
    }
  } finally {
    isActive.value = false;
  }
}
</script>
<template>
  <div
    class="m-4 min-w-40 max-w-96 rounded-md border border-gray-300 bg-white px-10 py-8 text-gray-900 shadow-sm shadow-gray-300"
  >
    <h2 class="text-2xl font-semibold">{{ state === "log-in" ? "Log in" : "Sign up" }}</h2>
    <div class="mt-5 flex w-full flex-col gap-y-3">
      <String
        v-if="state === 'sign-up'"
        :icon="makeIcon({ name: 'fas fa-envelope' })"
        name="name"
        title="Name"
        is-input
        v-model="name"
      />
      <String :icon="makeIcon({ name: 'fas fa-at' })" name="username" title="Username" is-input v-model="username" />
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
        :icon="makeIcon({ name: 'fas fa-shuffle' })"
        name="switch"
        :title="state === 'log-in' ? 'Sign up instead' : 'Log in instead'"
        class="mt-2 w-full"
        :variant="Variant.V3"
        @click="() => (state = state === 'log-in' ? 'sign-up' : 'log-in')"
      />
    </div>
  </div>
</template>
