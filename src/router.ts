import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', redirect: '/general' },
  { path: '/general', name: 'general', component: () => import('./pages/General.vue') },
  { path: '/throw', name: 'throw', component: () => import('./pages/Throw.vue') },
  { path: '/snap-editor', name: 'snap-editor', component: () => import('./pages/SnapEditor.vue') },
  { path: '/keyboard', name: 'keyboard', component: () => import('./pages/Keyboard.vue') },
  { path: '/display', name: 'display', component: () => import('./pages/Display.vue') },
  { path: '/about', name: 'about', component: () => import('./pages/About.vue') },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})
