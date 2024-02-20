<script lang="ts" setup>
import BenchIcon from "@/components/basic/BenchIcon.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import ValidationMessage from "@/components/basic/ValidationMessage.vue";
import { useValidName, useValidSlug } from "@/composables/useValidation";
import { useRedirectIfNotLoggedIn, useRedirectIfWaitlisted } from "@/state/auth";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { useTitle } from "@vueuse/core";
import { computed, onMounted, ref, type Ref } from "vue";
import { useRouter } from "vue-router";

const title = useTitle();
title.value = "Start your organization";

useRedirectIfNotLoggedIn({ name: "Signup" });
useRedirectIfWaitlisted();

const name: Ref<string> = ref("Hooli, Inc.");
const slug: Ref<string> = ref("hooli");
const slugModified = ref(false);

function syncSlugIfUnmodified() {
  if (!slugModified.value) {
    let nameTrimmed = name.value.toLowerCase();
    if (nameTrimmed.includes(",")) {
      nameTrimmed = nameTrimmed.split(",")[0];
    }
    if (nameTrimmed.includes("-")) {
      nameTrimmed = nameTrimmed.split("-")[0];
    }
    slug.value = nameTrimmed.replace(/[^a-z0-9_-]/g, "-");
  }
}

const isValidName = useValidName(name);
const validSlug = useValidSlug(slug);

const canComplete = computed(
  () =>
    !creating.value &&
    !validSlug.loading.value &&
    validSlug.available.value &&
    validSlug.valid.value &&
    isValidName.valid.value
);

// auto-focus name on load
const nameRef: Ref<HTMLInputElement | null> = ref(null);
onMounted(() => {
  nameRef.value?.focus();
  // select all
  nameRef.value?.setSelectionRange(0, nameRef.value?.value?.length ?? 0);
});

const creating: Ref<boolean> = ref(false);

const ops = useOperations();
const router = useRouter();
const notifications = useNotifications();
async function createOrganization() {
  creating.value = true;
  const create = await ops.organization.create(name.value, slug.value);
  creating.value = false;
  if (create?.data?.createOrganization.__typename == "Organization") {
    router.push(`/settings/${slug.value}#members`);
    notifications.show({
      kind: "success",
      type: "organization.created",
      message: "Organization created",
      description: `It's official: ${name.value} is ready to take over the world.`,
    });
  }
}
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right>
        <ProfileButton />
      </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-2">
        <BenchIcon class="h-4 w-4 text-orange-600" />
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="-mx-32 mt-4 text-5xl font-bold">Start your organization</h1>
      <p class="-mx-4 mt-4 text-lg text-orange-700">A home to your amazing team and bots.</p>
      <!-- Fields to complete -->
      <div class="mt-10 flex flex-col gap-4">
        <!-- TODO @Incomplete: select owner -->

        <!-- Full name -->
        <div class="text-left">
          <span class="text-sm font-bold text-gray-900">Name</span>
          <input
            ref="nameRef"
            type="text"
            minlength="3"
            maxlength="128"
            v-model="name"
            @input="syncSlugIfUnmodified"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            spellcheck="false"
          />
          <ValidationMessage name="name" :valid="isValidName.valid.value" />
        </div>

        <!-- slug -->
        <div class="text-left">
          <span class="text-sm font-bold text-gray-900">Bot-era callsign</span>
          <input
            type="text"
            minlength="3"
            maxlength="128"
            pattern="[a-z0-9_-]"
            :value="slug"
            @input="(event) => ((slug = (event.target as any)?.value), (slugModified = true))"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            spellcheck="false"
          />
          <ValidationMessage
            name="slug"
            :valid="validSlug.valid.value"
            pattern="[a-z0-9_-]{3,}"
            :loading="validSlug.loading.value"
            :unavailable="!validSlug.available.value"
            :takenTo="`/${slug}`"
          />
        </div>

        <button
          :disabled="!canComplete"
          class="mt-6 w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
          :class="{ 'pointer-events-none opacity-50': !canComplete }"
          @click="createOrganization"
        >
          Start &rarr;
        </button>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
