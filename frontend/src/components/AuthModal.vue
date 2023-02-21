<script lang="ts" setup>
import PopupDialog from "@/components/container/PopupDialog.vue";
import { type Ref, ref } from "vue";

const dialogRef: Ref<InstanceType<typeof PopupDialog> | null> = ref(null);

defineExpose({
  show: () => dialogRef.value?.show(),
  hide: () => dialogRef.value?.hide(),
});
</script>
<template>
  <PopupDialog ref="dialogRef" title="Log in">
    <div class="font-mono text-2xl font-bold">
      <span class="-mx-0.5 text-gray-900">[</span>
      <span class="text-3xl text-orange-600">x</span>
      <span class="-mx-0.5 text-gray-900">]</span>
    </div>
    auth here
  </PopupDialog>
</template>
