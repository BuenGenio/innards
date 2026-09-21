// UI chrome strings (tab labels, buttons, paywall copy, settings labels, empty
// states, error messages). Product/report strings come from the Rust i18n
// catalogs; this file only covers the shell around them.
//
// Adding a language: add a table with exactly the same keys as `en`.
// `scripts/check-ui-strings.mjs` verifies that and fails on leftover inline
// bilingual strings in components.

export const STRINGS: Record<string, Record<string, string>> = {
  en: {
    // Top bar
    "nav.report": "Report",
    "nav.advisor": "Upgrade advisor",
    "nav.settings": "Settings",
    "level.plain": "Plain",
    "level.informed": "Informed",
    "level.expert": "Expert",
    "topbar.rescan": "Rescan",

    // Layout error card
    "error.title": "Something went wrong while inspecting this machine.",
    "error.retry": "Try again",

    // Scanning
    "scanning.title": "Looking inside…",
    "scanning.sub": "Reading CPU, memory, drives, battery and sensors. Nothing leaves this machine.",

    // Paywall
    "paywall.supporter.title": "Support Innards — $9 once",
    "paywall.supporter.body": "Unlocks the interactive upgrade advisor, narrated summaries, and your machine's history. One-time, pay what you want from $5.",
    "paywall.supporter.cta": "Get Supporter",
    "paywall.pro.title": "Innards Pro — $29/year",
    "paywall.pro.body": "Where to buy each recommendation — new, refurbished or used, priced for your region. Multi-machine view.",
    "paywall.pro.cta": "Get Pro",
    "paywall.have_key": "I have a key",

    // Report
    "report.other_machine": "Another machine's report",
    "report.back": "Back to this machine",
    "report.export_md": "Export Markdown",
    "report.export_json": "Export JSON",
    "report.open": "Open report…",
    "report.requires_pro": "Requires Pro.",
    "finding.evidence": "Evidence",

    // Narrative
    "narrative.title": "In a few words",
    "narrative.no_key": "Add your Anthropic API key in Settings to generate the summary.",
    "narrative.open_settings": "Open Settings",
    "narrative.intro": "A narrated summary of this report, in your language and at your level.",
    "narrative.writing": "Writing…",
    "narrative.regenerate": "Regenerate",
    "narrative.write": "Write summary",

    // History
    "history.title": "Over time",
    "history.other_machine": "History is per machine; you're viewing another machine's report.",
    "history.empty": "Every scan adds a point. Come back after a few.",
    "history.scans_over": "scans over",
    "history.days": "days",
    "history.health": "Health",
    "history.battery": "Battery",
    "history.disk_free": "Disk free",
    "history.ram_free": "RAM free",

    // Advisor
    "advisor.title": "Upgrade advisor",
    "advisor.intro": "Five questions. Then a ranked list of what is — and isn't — worth buying for this specific machine.",
    "advisor.show": "Show recommendations",
    "advisor.recommendations": "Recommendations",
    "advisor.view.cards": "Cards",
    "advisor.view.visual": "Visual",
    "advisor.over_budget": "Over your budget",
    "advisor.helps": "Helps:",
    "advisor.where_to_buy": "Where to buy",
    "advisor.where_to_buy_pro": "Where to buy (new · refurbished · used) — Pro",
    "advisor.refine": "Refine",
    "advisor.cond.new": "New",
    "advisor.cond.refurbished": "Refurbished",
    "advisor.cond.used": "Used",

    // Recommendation map
    "recmap.aria": "Impact versus cost map",
    "recmap.cost_axis": "estimated cost →",
    "recmap.budget": "budget",

    // Settings
    "settings.license.title": "License",
    "settings.license.current": "Current tier:",
    "settings.license.expires": "expires",
    "settings.license.activate": "Activate",
    "settings.license.activated": "Activated: {tier}",
    "settings.license.invalid": "That key isn't valid.",
    "settings.narrative.title": "Narrated summaries",
    "settings.narrative.body": "Summaries are generated with the Claude API using your own key. Only the rendered findings are sent — never serial numbers, hostnames, or process lists.",
    "settings.analysis.title": "Analysis",
    "settings.analysis.smart": "Ask for administrator rights to read drive health (SMART)",
    "settings.analysis.smart_sub": "Shows the system password prompt on each scan. Requires smartmontools.",
    "settings.analysis.region": "Shopping region (Pro)",
    "settings.team.title": "Team & Enterprise",
    "settings.team.body": "Send this machine's reports to your organisation's fleet dashboard (Threadwise with the innards.fleet plugin). Serial numbers, hostname, MAC addresses and process lists are stripped before upload.",
    "settings.team.available": "Available on Team ($4/machine/month) and Enterprise.",
    "settings.team.contact": "Get in touch",
    "settings.team.url": "Threadwise URL",
    "settings.team.api_key": "API key (tw_…)",
    "settings.team.label": "Label for this machine",
    "settings.team.label_placeholder": "Ana's laptop",
    "settings.team.auto_upload": "Upload automatically after every scan and on a schedule",
    "settings.team.interval": "Every (hours), while the app is open",
    "settings.team.upload_now": "Upload report now",
    "settings.team.open_dashboard": "Open in dashboard",
    "settings.team.uploaded": "Report uploaded.",
    "settings.team.requires_team": "Requires Team or Enterprise.",
    "settings.team.missing_endpoint": "Endpoint or API key missing.",
    "settings.save": "Save",
  },
  es: {
    // Top bar
    "nav.report": "Informe",
    "nav.advisor": "Asesor de mejoras",
    "nav.settings": "Ajustes",
    "level.plain": "Sencillo",
    "level.informed": "Informado",
    "level.expert": "Experto",
    "topbar.rescan": "Reanalizar",

    // Layout error card
    "error.title": "Algo ha fallado al inspeccionar este equipo.",
    "error.retry": "Reintentar",

    // Scanning
    "scanning.title": "Mirando por dentro…",
    "scanning.sub": "Leyendo CPU, memoria, discos, batería y sensores. Nada sale de este equipo.",

    // Paywall
    "paywall.supporter.title": "Apoya Innards — 9 $ una vez",
    "paywall.supporter.body": "Desbloquea el asesor de mejoras interactivo, los resúmenes narrados y el historial de tu equipo. Pago único, paga lo que quieras desde 5 $.",
    "paywall.supporter.cta": "Conseguir Supporter",
    "paywall.pro.title": "Innards Pro — 29 $/año",
    "paywall.pro.body": "Dónde comprar cada recomendación: nuevo, reacondicionado o de segunda mano, con precios de tu región. Vista de varios equipos.",
    "paywall.pro.cta": "Conseguir Pro",
    "paywall.have_key": "Ya tengo una clave",

    // Report
    "report.other_machine": "Informe de otra máquina",
    "report.back": "Volver a esta máquina",
    "report.export_md": "Exportar Markdown",
    "report.export_json": "Exportar JSON",
    "report.open": "Abrir informe…",
    "report.requires_pro": "Requiere Pro.",
    "finding.evidence": "Evidencia",

    // Narrative
    "narrative.title": "En pocas palabras",
    "narrative.no_key": "Añade tu clave de API de Anthropic en Ajustes para generar el resumen.",
    "narrative.open_settings": "Ir a Ajustes",
    "narrative.intro": "Un resumen narrado de este informe, en tu idioma y a tu nivel.",
    "narrative.writing": "Escribiendo…",
    "narrative.regenerate": "Regenerar",
    "narrative.write": "Escribir resumen",

    // History
    "history.title": "Con el tiempo",
    "history.other_machine": "El historial es de esta máquina; estás viendo el informe de otra.",
    "history.empty": "Cada análisis añade un punto. Vuelve después de unos cuantos.",
    "history.scans_over": "análisis en",
    "history.days": "días",
    "history.health": "Salud",
    "history.battery": "Batería",
    "history.disk_free": "Disco libre",
    "history.ram_free": "RAM libre",

    // Advisor
    "advisor.title": "Asesor de mejoras",
    "advisor.intro": "Cinco preguntas. Después, una lista priorizada de lo que merece la pena comprar (o no) para este equipo en concreto.",
    "advisor.show": "Ver recomendaciones",
    "advisor.recommendations": "Recomendaciones",
    "advisor.view.cards": "Tarjetas",
    "advisor.view.visual": "Visual",
    "advisor.over_budget": "Por encima de tu presupuesto",
    "advisor.helps": "Ayuda a:",
    "advisor.where_to_buy": "Dónde comprar",
    "advisor.where_to_buy_pro": "Dónde comprar (nuevo · reacondicionado · usado) — Pro",
    "advisor.refine": "Afinar",
    "advisor.cond.new": "Nuevo",
    "advisor.cond.refurbished": "Reacondicionado",
    "advisor.cond.used": "Segunda mano",

    // Recommendation map
    "recmap.aria": "Mapa de impacto frente a coste",
    "recmap.cost_axis": "coste estimado →",
    "recmap.budget": "presupuesto",

    // Settings
    "settings.license.title": "Licencia",
    "settings.license.current": "Nivel actual:",
    "settings.license.expires": "caduca",
    "settings.license.activate": "Activar",
    "settings.license.activated": "Activado: {tier}",
    "settings.license.invalid": "Clave no válida.",
    "settings.narrative.title": "Resúmenes narrados",
    "settings.narrative.body": "Los resúmenes se generan con la API de Claude usando tu propia clave. Solo se envían los hallazgos ya redactados; nunca números de serie, nombres de equipo ni procesos.",
    "settings.analysis.title": "Análisis",
    "settings.analysis.smart": "Pedir permisos de administrador para leer la salud de los discos (SMART)",
    "settings.analysis.smart_sub": "Muestra el diálogo de contraseña del sistema en cada análisis. Requiere smartmontools.",
    "settings.analysis.region": "Región para tiendas (Pro)",
    "settings.team.title": "Equipo y empresa",
    "settings.team.body": "Envía los informes de esta máquina al panel de flota de tu organización (Threadwise con el plugin innards.fleet). Se eliminan números de serie, nombre de equipo, direcciones MAC y procesos antes de enviar.",
    "settings.team.available": "Disponible en Team (4 $/máquina/mes) y Enterprise.",
    "settings.team.contact": "Contactar",
    "settings.team.url": "URL de Threadwise",
    "settings.team.api_key": "Clave de API (tw_…)",
    "settings.team.label": "Etiqueta de esta máquina",
    "settings.team.label_placeholder": "Portátil de Ana",
    "settings.team.auto_upload": "Enviar automáticamente tras cada análisis y de forma programada",
    "settings.team.interval": "Cada (horas), mientras la app esté abierta",
    "settings.team.upload_now": "Enviar informe ahora",
    "settings.team.open_dashboard": "Ver en el panel",
    "settings.team.uploaded": "Informe enviado.",
    "settings.team.requires_team": "Requiere Team o Enterprise.",
    "settings.team.missing_endpoint": "Falta la URL o la clave de API.",
    "settings.save": "Guardar",
  },
};

/** Look up a UI chrome string. Falls back to English, then to the key itself. */
export function ui(lang: string, key: string, params?: Record<string, string | number>): string {
  const s = STRINGS[lang]?.[key] ?? STRINGS.en[key] ?? key;
  if (!params) return s;
  return s.replace(/\{(\w+)\}/g, (m, name: string) => (name in params ? String(params[name]) : m));
}
