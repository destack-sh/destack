<script lang="ts" setup>
import { Dialog, DialogPanel, TransitionChild, TransitionRoot } from "@headlessui/vue";
import { ref } from "vue";

const open = ref(false);
defineProps<{ title: string }>();

function show() {
  open.value = true;
}

function hide() {
  open.value = false;
}

defineExpose({ show, hide });
</script>
<template>
  <TransitionRoot :show="open" as="template" appear>
    <Dialog as="div" class="relative z-10" @close="open = false">
      <TransitionChild
        as="template"
        enter="ease-out duration-300"
        enter-from="opacity-0"
        enter-to="opacity-100"
        leave="ease-in duration-200"
        leave-from="opacity-100"
        leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-gray-500 bg-opacity-25 transition-opacity" />
      </TransitionChild>

      <div class="fixed inset-0 z-10 overflow-y-auto p-4 sm:p-6 md:p-20">
        <TransitionChild
          as="template"
          enter="ease-out duration-300"
          enter-from="opacity-0 scale-95"
          enter-to="opacity-100 scale-100"
          leave="ease-in duration-200"
          leave-from="opacity-100 scale-100"
          leave-to="opacity-0 scale-95"
        >
          <DialogPanel
            class="mx-auto max-w-xl transform rounded-sm bg-white p-2 shadow-md ring-1 ring-black ring-opacity-5 transition-all"
          >
            <slot name="title">
              <h3 v-if="title" as="h3" class="px-4 pt-1 pb-2 text-lg font-medium leading-6 text-gray-900">
                {{ title }}
              </h3>
            </slot>
            <slot>
              <!-- put content here -->
              <div class="absolute inset-0 px-4 sm:px-6">
                <div class="h-full border-2 border-dashed border-gray-200" aria-hidden="true" />
              </div>
            </slot>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
