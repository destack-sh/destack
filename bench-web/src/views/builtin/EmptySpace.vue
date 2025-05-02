<script lang="ts" setup>
import { UserStatus } from "@/proto/wire";
import { assignSpaceInPackage, bench, canvas, pkg, space } from "@/system/space";
import { user } from "@/system/user";
import { fireCommandById } from "@/ui/command";
import { makeIcon } from "@/ui/icon";
import { createDesktopDefaultSpace } from "@/ui/space";
import { DISCORD_URL } from "@/utils/globals";
import Button from "@/views/controls/Button.vue";
</script>
<template>
  <div>
    <div v-if="space && bench" class="flex w-fit flex-col gap-y-2 self-center">
      <!-- Space empty for some reason -->
      <span>
        <i class="fas fa-empty-set mr-1.5 text-gray-500" />
        <span class="text-gray-600">Space Is Empty</span>
      </span>
      <Button
        id="create"
        name="Create"
        :icon="makeIcon('fas fa-redo-alt')"
        title="Restore Default"
        @click="() => createDesktopDefaultSpace(canvas.tx(), space!)"
      />
    </div>
    <div v-else-if="bench" class="flex w-fit flex-col self-center">
      <!-- Space inaccessible for some reason -->
      <span>
        <i class="fas fa-circle-exclamation mr-1.5 text-gray-500" />
        <span class="text-gray-600"
          >Space not found in <span class="font-medium">@{{ bench.slug }}</span></span
        >
      </span>
      <span class="text-gray-400">
        (<span v-if="user"
          >Logged in as <span class="font-medium">{{ user.slug }}</span></span
        >
        <span v-else>Not logged in</span>)
      </span>
      <Button
        v-if="user"
        id="createspace"
        class="mt-2"
        name="CreateSpace"
        :icon="makeIcon('fas fa-plus')"
        title="Create Space"
        @click="() => pkg && assignSpaceInPackage(pkg)"
      />
      <Button
        v-else
        id="login"
        class="mt-2"
        name="LogIn"
        :icon="makeIcon('fas fa-arrow-right-to-bracket')"
        title="Log In"
        @click="fireCommandById('user.auth.login')"
      />
    </div>
    <div v-else-if="user && user.status == UserStatus.WAITLISTED" class="flex flex-col gap-y-2 self-center">
      <!-- Logged in but waitlisted -->
      <span>
        <i class="fas fa-clock mr-1.5 text-gray-500" />
        <span class="text-gray-600"
          ><span class="font-semibold">{{ user.slug }}</span> is on the waitlist</span
        >
      </span>
      <a :href="DISCORD_URL" class="mt-1 underline">Discord</a>
    </div>
    <div v-else-if="user" class="flex flex-col gap-y-2 self-center">
      <!-- Logged in but not on any space (not sure if this should even show or just auto-redirect?) -->
      <span>
        <i class="fas fa-circle-exclamation mr-1.5 text-gray-500" />
        <span class="text-gray-600">You're Lost in Space</span>
      </span>
      <Button
        v-if="user.status == UserStatus.ACTIVATED"
        id="home"
        name="GoHome"
        :icon="makeIcon('fas fa-home')"
        title="Go Home"
        @click="fireCommandById('user.navigate.goToHome')"
      />
      <Button
        v-else
        id="activate"
        name="Activate"
        :icon="makeIcon('fas fa-plus')"
        title="Create Bench"
        @click="fireCommandById('user.navigate.activate')"
      />
    </div>
    <div v-else class="flex flex-col gap-y-2 self-center">
      <!-- Not logged in, not on a space  -->
      <h2 class="mb-1.5 text-2xl font-bold">Bench</h2>
      <Button
        id="login"
        name="LogIn"
        :icon="makeIcon('fas fa-arrow-right-to-bracket')"
        title="Log In"
        @click="fireCommandById('user.auth.login')"
      />
    </div>
  </div>
</template>
