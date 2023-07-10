<script lang="ts" setup>
import { getClientColor } from "@/state/client";

const props = defineProps<{
  size: "large" | "medium" | "small";
  clientId?: string;
  user: { username: string; name: string };
}>();
</script>
<template>
  <div
    class="rounded-sm border border-orange-900 border-opacity-[15%] bg-orange-100"
    :class="{
      'px-2.5 py-1': props.size === 'large',
      'px-1.5 py-[3px]': props.size === 'medium',
      'px-1 font-light': props.size === 'small',
    }"
    :style="{
      backgroundColor: clientId != null ? getClientColor(clientId) : '#fb923c',
    }"
  >
    <span class="text-sm text-gray-900">
      {{ user.username.slice(0, 2).toLocaleUpperCase() }}
    </span>
  </div>
</template>
