# Server Directory

**This folder should be deleted once `@nuxtjs/i18n` properly respects `warnHtmlMessage: false` during SSR.**

This is a temporary workaround to suppress noisy intlify HTML warnings during build. A bit over-engineered for what it does, but those warnings were annoying.

## Why does this exist?

Even though this website is fully static (no runtime server), Nuxt uses [Nitro](https://nitro.build) to prerender pages at build time. Nitro plugins run during that process, letting us patch `console.warn` to filter out the warnings.

## When to delete

Check if the warnings are gone by removing this folder and running `npm run build`. If no `[intlify] Detected HTML` warnings appear, delete this folder.

Related: https://github.com/nuxt-modules/i18n/discussions/1968

## vue-i18n Deprecation Warning

The `npm warn deprecated vue-i18n@10.0.8` warning during install comes from a transitive dependency
of `@nuxtjs/i18n`. This is a known issue - waiting for `@nuxtjs/i18n` to upgrade to vue-i18n v11 internally.
No action needed on our part.
