// Site-wide constants. SITE_URL is baked in at build time (canonical, OG, sitemap).
export const SITE_URL = (import.meta.env.VITE_SITE_URL as string | undefined)?.replace(/\/$/, '') ?? 'https://innards.app';
export const SITE_NAME = 'innards';
export const GITHUB_URL = 'https://github.com/buengenio/innards';
export const RELEASES_API = 'https://api.github.com/repos/buengenio/innards/releases/latest';
export const VERSION = '0.1.0';
