<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { CheckCircleIcon, XCircleIcon } from "@heroicons/vue/24/solid";

defineProps<{
  name: string;
  valid: boolean;
  pattern?: string;
  loading?: boolean;
  unavailable?: boolean;
  takenTo?: any;
}>();
</script>
<template>
  <FadeTransition as="div" mode="out-in">
    <span v-if="!valid" class="mt-1 flex flex-row items-center gap-1 text-sm text-red-600">
      <XCircleIcon class="inline-block h-4 w-4" />
      <template v-if="pattern"
        >The bots want a {{ name }} like <span class="font-mono text-xs text-gray-500">{{ pattern }}</span></template
      >
      <template v-else>The bots don't like this {{ name }}.</template>
    </span>
    <span v-else-if="loading" class="mt-1">&nbsp;</span>
    <span v-else-if="unavailable" class="mt-1 flex flex-row items-center text-sm text-red-600">
      <XCircleIcon class="mr-1 inline-block h-4 w-4" />
      That {{ name }} is
      <router-link
        v-if="takenTo"
        :to="takenTo"
        class="ml-1 underline decoration-dotted underline-offset-2 hover:decoration-solid focus:decoration-solid focus:outline-none"
        >taken</router-link
      ><template v-else>taken</template>.
    </span>
    <span v-else class="mt-1 flex flex-row items-center gap-1 text-sm text-green-700">
      <CheckCircleIcon class="inline-block h-4 w-4" />
      Great {{ name }}.</span
    >
  </FadeTransition>
</template>
