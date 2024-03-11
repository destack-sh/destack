<script lang="tsx" setup>
import { Variant, type NodeReferenceData, type ViewData } from "@/proto/wire";
import { viewEmits } from "@/views/common";
import String from "@/views/content/String.vue";
import Button from "@/views/controls/Button.vue";
import { ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self: NodeReferenceData } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr"> & {}
>();
const emit = defineEmits(viewEmits());

const status: Ref<"sign-up" | "log-in" | "welcome"> = ref("log-in");
const name: Ref<string> = ref("");
const username: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div>
    <h2 class="text-3xl text-gray-900">Sign in / ...</h2>
    <String name="name" title="Name" is-input v-model="name" />
    <String name="username" title="Username" is-input v-model="username" />
    <String name="email" title="Email" is-input v-model="email" />
    <String name="password" title="Password" is-input is-secret v-model="password" />
    <div class="flex flex-row justify-between">
      <Button name="cancel" title="Cancel" :variant="Variant.SECONDARY" />
      <Button name="confirm" title="Sign in" />
    </div>
  </div>
</template>
