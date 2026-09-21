import { SITE_URL } from '$lib/site';
export const prerender = true;
const pages = ['', '/download', '/privacy'];
export function GET() {
  const body = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${pages.map((p) => `  <url><loc>${SITE_URL}${p}</loc></url>`).join('\n')}\n</urlset>\n`;
  return new Response(body, { headers: { 'Content-Type': 'application/xml' } });
}
