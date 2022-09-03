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
  color: { type: String as PropType<"slate" | "orange" | "white">, default: "orange" },
  type: { type: String as PropType<"button" | "submit">, default: "button" },
  text: String,
  to: [String, Object] as PropType<string | object>,
});

const baseStyles: Record<string, string> = {
  solid:
    "group inline-flex items-center justify-center rounded-md py-2 px-4 text-sm font-semibold focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-2",
  outline: "group inline-flex ring-1 items-center justify-center rounded-md py-2 px-4 text-sm focus:outline-none",
};

const variantStyles: Record<string, Record<string, string>> = {
  solid: {
    slate:
      "bg-slate-700 text-white hover:bg-slate-700 hover:text-slate-100 active:bg-slate-800 active:text-slate-300 focus-visible:outline-slate-900",
    orange:
      "bg-orange-600 text-white hover:text-slate-100 hover:bg-orange-500 active:bg-orange-800 active:text-orange-100 focus-visible:outline-orange-600",
    white:
      "bg-white text-slate-900 hover:bg-orange-50 active:bg-orange-200 active:text-slate-600 focus-visible:outline-white",
  },
  outline: {
    slate:
      "ring-slate-200 text-slate-700 hover:text-slate-900 hover:ring-slate-300 active:bg-slate-100 active:text-slate-600 focus-visible:outline-orange-600 focus-visible:ring-slate-300",
    white:
      "ring-slate-700 text-white hover:ring-slate-500 active:ring-slate-700 active:text-slate-400 focus-visible:outline-white",
  },
};

const style = computed(() => baseStyles[props.variant] + " " + variantStyles[props.variant][props.color]);
</script>
