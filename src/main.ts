import './assets/css/main.css'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHashHistory } from 'vue-router'
import VueKonva from 'vue-konva'
import ui from '@nuxt/ui/vue-plugin'
import { i18n } from './i18n'
import App from './App.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/general' },
    { path: '/general', name: 'general', component: () => import('./pages/General.vue') },
    { path: '/throw', name: 'throw', component: () => import('./pages/Throw.vue') },
    { path: '/snap-editor', name: 'snap-editor', component: () => import('./pages/SnapEditor.vue') },
    { path: '/display', name: 'display', component: () => import('./pages/Display.vue') },
    { path: '/about', name: 'about', component: () => import('./pages/About.vue') },
  ],
})

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.use(ui)
app.use(VueKonva)
app.mount('#app')
