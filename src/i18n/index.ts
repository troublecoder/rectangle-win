import { createI18n } from 'vue-i18n'
import { en, ko } from '@nuxt/ui/locale'
import enMessages from './locales/en'
import koMessages from './locales/ko'

export const i18n = createI18n({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages: {
    en: { ...en, ...enMessages },
    ko: { ...ko, ...koMessages },
  },
})

export const supportedLocales = [
  { code: 'en', name: 'English' },
  { code: 'ko', name: '한국어' },
] as const

export type SupportedLocale = (typeof supportedLocales)[number]['code']
