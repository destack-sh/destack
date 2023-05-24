<script lang="ts" setup>
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type EditorContext, type RunEditor } from "@/state/editor";
import { computed } from "vue";

const props = defineProps<{ editor: EditorContext<RunEditor>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const actions = useActions();
const editor = computed(() => props.editor.editor.value);
</script>
<template>
  <div>run it</div>
</template>
