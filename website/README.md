<div align="center">
<img src="https://raw.githubusercontent.com/frankdierolf/whis/main/crates/whis-desktop/icons/128x128.png" alt="whis" width="80" height="80" />

<h3>whis.ink</h3>
<p>
  Website for Whis.
  <br />
  <a href="https://whis.ink">Live Site</a>
  ·
  <a href="https://github.com/frankdierolf/whis/tree/main/crates/whis-cli">CLI</a>
  ·
  <a href="https://github.com/frankdierolf/whis/tree/main/crates/whis-desktop">Desktop</a>
  ·
  <a href="https://github.com/frankdierolf/whis/tree/main/crates/whis-mobile">Mobile</a>
</p>
</div>

## Introduction

Landing page for Whis. Demo, features, downloads for CLI and Desktop, and FAQ.

## Screenshot

![Screenshot](./public/screenshot.png)

## Development

```bash
npm install
npm run dev      # Start dev server
npm run build    # Build for production
npm run lint     # Check code style
```

## Deployment

Automatically deployed to GitHub Pages on push to main.
See `.github/workflows/website.yml`.

## Structure

```
app/
  pages/       # Page routes (index, cli, desktop, mobile, faq, downloads)
  components/  # Reusable UI components
  composables/ # Composable functions (GitHub API, platform detection)
  assets/      # CSS tokens and base styles
  layouts/     # Layout components (default)
public/        # Static files
.nuxt/         # Auto-generated (temporary)
```

## Notes

The installation count shown on [whis.ink](https://whis.ink) aggregates downloads from three sources:

- **[Crates.io](https://crates.io/crates/whis)** - Rust crate downloads
- **[GitHub Releases](https://github.com/frankdierolf/whis/releases)** - Binary downloads (AppImage, deb, rpm, DMG, MSI, etc.)
- **[Flathub](https://flathub.org/apps/ink.whis.Whis)** - Flatpak installs

Stats are fetched at build-time to avoid CORS issues. Implementation: [`scripts/fetch-stats.mjs`](scripts/fetch-stats.mjs) and [`app/composables/useDownloadStats.ts`](app/composables/useDownloadStats.ts)
