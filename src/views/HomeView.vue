<script lang="ts" setup>
import { RouterLink } from "vue-router";
</script>
<template>
    <div class="flex flex-col items-center w-full h-full bg-gray-50">
        <h1 class="text-3xl w-full text-center pt-10 text-gray-800">Pyrite</h1>
        <div class="flex h-full w-full items-center">
            <div class="flex grow"></div>
            <div class="flex flex-col gap-10 h-full justify-center">
                <router-link
                    to="/parse"
                    class="text-center px-6 py-2 font-medium tracking-wide text-white capitalize transition-colors duration-300 transform bg-blue-500 rounded-lg hover:bg-blue-400 focus:outline-none focus:ring focus:ring-blue-200 focus:ring-opacity-80"
                >
                    Parse Event Feed
                </router-link>
                <router-link
                    to="/show"
                    class="text-center px-6 py-2 font-medium tracking-wide text-white capitalize transition-colors duration-300 transform bg-green-500 rounded-lg hover:bg-green-400 focus:outline-none focus:ring focus:ring-green-200 focus:ring-opacity-80"
                >
                    Roll Board
                </router-link>
            </div>
            <div class="flex grow"></div>
        </div>
    </div>
</template>
