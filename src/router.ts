import { createRouter, createWebHistory } from "vue-router";
import HomeView from "./views/HomeView.vue";
import ParseView from "./views/ParseView.vue";
import ShowView from "./views/ShowView.vue";

const routes = [
  { path: "/", component: HomeView },
  { path: "/parse", component: ParseView },
  { path: "/show", component: ShowView },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
