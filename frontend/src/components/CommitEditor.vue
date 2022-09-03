<template>
  <div>
    <h3 class="mt-3 text-lg font-medium leading-6 text-gray-900">Commit changes</h3>
    <TextInput class="mt-2 sm:col-span-4" v-model="title" label="Commit title" label-hidden />
    <TextInput
      class="sm:col-span-6"
      v-model="description"
      label="Commit description"
      label-hidden
      placeholder="Describe your change in more detail"
      :rows="3"
    />
    <div class="pt-3">
      <div class="flex justify-end">
        <SButton type="submit" @click.prevent="$emit('commit', { title, description })"> Commit </SButton>
      </div>
    </div>
  </div>
</template>
<script lang="ts" setup>
import { ref, toRef, watch, type Ref } from "vue";
import SButton from "./basic/SButton.vue";
import TextInput from "./basic/TextInput.vue";

const props = defineProps<{ suggestedTitle?: string; suggestedDescription?: string }>();
const title: Ref<string> = ref("");
const description: Ref<string> = ref("");

watch(toRef(props, "suggestedTitle"), (newTitle) => (title.value = newTitle || ""), { immediate: true });
watch(toRef(props, "suggestedDescription"), (newDescription) => (description.value = newDescription || ""), {
  immediate: true,
});

defineEmits<{ (e: "commit", value: { commitTitle: string; commitDescription?: string }): void }>();
</script>
