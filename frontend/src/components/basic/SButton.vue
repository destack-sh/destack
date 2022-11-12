<template>
  <router-link v-if="to" :to="to">
    <button :type="type" :class="style">
      <slot>{{ text }}</slot>
    </button>
  </router-link>
  <button v-else :type="type" :class="style">
    <slot>{{ text }}</slot>
  </button>
</template>
<script lang="ts" setup>
import { computed, type PropType } from "vue";

const props = defineProps({
  variant: { type: String as PropType<"solid" | "outline">, default: "solid" },
  color: { type: String as PropType<"white" | "orange">, default: "orange" },
  type: { type: String as PropType<"button" | "submit">, default: "button" },
  text: String,
  to: [String, Object] as PropType<string | object>,
});

const baseStyles: Record<string, string> = {
  solid:
    "group inline-flex items-center justify-center rounded-sm py-2 px-3 text-sm font-semibold focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-2",
  outline:
    "group inline-flex ring-1 ring-inset items-center justify-center rounded-sm py-2 px-3 text-sm focus:outline-none",
};

const variantStyles: Record<string, Record<string, string>> = {
  solid: {
    white:
      "bg-white text-gray-700 hover:bg-slate-700 hover:text-slate-100 active:bg-slate-800 active:text-slate-300 focus-visible:outline-slate-900",
    orange:
      "bg-orange-600 text-white hover:text-slate-100 hover:bg-orange-700 active:bg-orange-800 active:text-orange-100 focus-visible:outline-orange-600",
  },
  outline: {
    white:
      "ring-slate-400 text-slate-900 hover:bg-gray-50 active:bg-gray-100 active:text-slate-600 focus-visible:outline-orange-600 focus-visible:ring-slate-300",
  },
};

const style = computed(() => baseStyles[props.variant] + " " + variantStyles[props.variant][props.color]);
</script>
