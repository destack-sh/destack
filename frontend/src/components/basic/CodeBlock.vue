<script lang="ts" setup>
import { ref, watch, computed, type Ref } from "vue";
import hljs from "highlight.js/lib/common";

const props = defineProps<{
  modelValue: string;
  language?: string;
  allowIllegals?: boolean;
}>();

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#x27;");
}

const language: Ref<string | null> = ref(props.language ?? null);

// sync in language from props
watch(
  () => props.language,
  (newLanguage) => {
    language.value = newLanguage ?? null;
  }
);

const cannotDetectLanguage = computed(() => language.value != null && !hljs.getLanguage(language.value));
const className = computed(() => {
  if (cannotDetectLanguage.value || language.value == null) {
    return "";
  } else {
    return `hljs ${language.value}`;
  }
});

const highlightedCode = ref<string>("");
watch(
  () => [props.modelValue, props.language, props.allowIllegals],
  () => {
    if (cannotDetectLanguage.value) {
      console.warn(`"${language.value}" does not exist in highlight.js`);
      highlightedCode.value = escapeHtml(props.modelValue);
    }

    if (language.value == null) {
      const result = hljs.highlightAuto(props.modelValue);
      language.value = result.language ?? null;
      highlightedCode.value = result.value;
    } else {
      const result = hljs.highlight(props.modelValue, {
        language: language.value,
        ignoreIllegals: !props.allowIllegals,
      });
      highlightedCode.value = result.value;
    }
  },
  { immediate: true }
);
</script>

<template>
  <!-- TODO @UX: ensure highlight js styling matches our monaco styling -->
  <pre><code :class="className" v-html="highlightedCode" tabindex="0"></code></pre>
</template>

<style scoped>
.hljs {
  background: none !important;
}
pre,
code {
  background: none !important;
  padding: 0 !important;
  margin: 0 !important;
}
</style>
