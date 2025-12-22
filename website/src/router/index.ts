import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '@/views/HomeView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'home', component: HomeView },
    { path: '/cli', name: 'cli', component: () => import('@/views/CliView.vue') },
    { path: '/desktop', name: 'desktop', component: () => import('@/views/DesktopView.vue') },
    { path: '/mobile', name: 'mobile', component: () => import('@/views/MobileView.vue') },
    { path: '/faq', name: 'faq', component: () => import('@/views/FaqView.vue') },
    { path: '/downloads', name: 'downloads', component: () => import('@/views/DownloadsView.vue') },
  ],
})

export default router
