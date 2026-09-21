<script lang="ts">
  import { app } from "$lib/state.svelte";
  import { api } from "$lib/api";
  import { onMount } from "svelte";

  let key = $state("");
  let keyMsg = $state<string | null>(null);
  let info = $state<{ version: string; core_version: string; os: string } | null>(null);
  let saved = $state(false);

  onMount(async () => { info = await api.appInfo(); });

  async function activate() {
    keyMsg = null;
    try {
      app.license = await api.activateLicense(key.trim());
      keyMsg = app.u("settings.license.activated", { tier: app.license.tier });
      key = "";
    } catch {
      keyMsg = app.u("settings.license.invalid");
    }
  }
  async function save() {
    await app.saveSettings();
    saved = true;
    setTimeout(() => (saved = false), 1500);
  }
  const regions = ["us", "uk", "de", "fr", "es", "it", "ca", "au", "jp"];

  let uploading = $state(false);
  let uploadMsg = $state<string | null>(null);
  let uploadUrl = $state<string | null>(null);
  async function upload() {
    uploading = true; uploadMsg = null; uploadUrl = null;
    try {
      await app.saveSettings();
      const r = await api.cloudUpload();
      uploadMsg = app.u("settings.team.uploaded");
      uploadUrl = r.dashboard_url;
    } catch (e) {
      const code = String(e);
      uploadMsg = code === "team" ? app.u("settings.team.requires_team")
        : code === "no_endpoint" || code === "no_token" ? app.u("settings.team.missing_endpoint")
        : code;
    } finally {
      uploading = false;
    }
  }
</script>

<div class="settings fade-in">
  <section class="card">
    <h2>{app.u("settings.license.title")}</h2>
    <p class="muted small">
      {app.u("settings.license.current")} <strong>{app.license.tier}</strong>
      {#if app.license.email}· {app.license.email}{/if}
      {#if app.license.org}· org <strong>{app.license.org}</strong>{/if}
      {#if app.license.expires}· {app.u("settings.license.expires")} {app.license.expires}{/if}
      {#if app.license.source === "env"}· <code>INNARDS_TIER</code>{/if}
    </p>
    <div class="row">
      <input type="text" placeholder="INNARDS-…" bind:value={key} style="flex:1; font-family: var(--mono)" />
      <button class="btn primary" onclick={activate} disabled={!key.trim()}>{app.u("settings.license.activate")}</button>
    </div>
    {#if keyMsg}<p class="small">{keyMsg}</p>{/if}
  </section>

  <section class="card">
    <h2>{app.u("settings.narrative.title")}</h2>
    <p class="muted small">
      {app.u("settings.narrative.body")}
    </p>
    <div class="row">
      <input type="password" placeholder="sk-ant-…" bind:value={app.settings.anthropic_api_key} style="flex:1; font-family: var(--mono)" />
    </div>
  </section>

  <section class="card">
    <h2>{app.u("settings.analysis.title")}</h2>
    <label class="check">
      <input type="checkbox" bind:checked={app.settings.elevate_for_smart} />
      <span>
        {app.u("settings.analysis.smart")}
        <span class="faint small block">{app.u("settings.analysis.smart_sub")}</span>
      </span>
    </label>
    <label class="check">
      <span>{app.u("settings.analysis.region")}</span>
      <select bind:value={app.settings.region}>
        {#each regions as r}<option value={r}>{r.toUpperCase()}</option>{/each}
      </select>
    </label>
  </section>

  <section class="card">
    <div class="head">
      <h2>{app.u("settings.team.title")}</h2>
      <span class="pill accent">Team</span>
    </div>
    <p class="muted small">
      {app.u("settings.team.body")}
    </p>
    {#if !app.has("team")}
      <p class="small faint">{app.u("settings.team.available")} <a href="mailto:eugene.trotsan@gmail.com?subject=Innards%20Team">{app.u("settings.team.contact")}</a></p>
    {/if}
    <label class="field"><span>{app.u("settings.team.url")}</span><input type="text" placeholder="https://crm.example.com" bind:value={app.settings.cloud_endpoint} disabled={!app.has("team")} /></label>
    <label class="field"><span>{app.u("settings.team.api_key")}</span><input type="password" placeholder="tw_…" bind:value={app.settings.cloud_token} disabled={!app.has("team")} style="font-family: var(--mono)" /></label>
    <label class="field"><span>{app.u("settings.team.label")}</span><input type="text" placeholder={app.u("settings.team.label_placeholder")} bind:value={app.settings.machine_label} disabled={!app.has("team")} /></label>
    <label class="check">
      <input type="checkbox" bind:checked={app.settings.cloud_auto_upload} disabled={!app.has("team")} />
      <span>{app.u("settings.team.auto_upload")}</span>
    </label>
    <label class="check">
      <span>{app.u("settings.team.interval")}</span>
      <select bind:value={app.settings.cloud_interval_hours} disabled={!app.has("team")}>
        {#each [1, 6, 12, 24, 168] as h}<option value={h}>{h}</option>{/each}
      </select>
    </label>
    <div class="row">
      <button class="btn" onclick={upload} disabled={uploading || !app.has("team")}>{uploading ? "…" : app.u("settings.team.upload_now")}</button>
      {#if uploadMsg}<span class="small">{uploadMsg}</span>{/if}
      {#if uploadUrl}<a class="small" href={uploadUrl} target="_blank" rel="noopener">{app.u("settings.team.open_dashboard")}</a>{/if}
    </div>
  </section>

  <div class="row">
    <button class="btn primary" onclick={save}>{saved ? "✓" : app.u("settings.save")}</button>
    {#if info}<span class="faint small">Innards {info.version} · core {info.core_version} · {info.os}</span>{/if}
  </div>
</div>

<style>
  .settings { max-width: 720px; margin: 0 auto; display: grid; gap: 16px; }
  section { padding: 16px 20px; display: grid; gap: 10px; }
  .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .head { display: flex; align-items: center; justify-content: space-between; }
  .field { display: grid; gap: 4px; font-size: 12px; color: var(--ink-2); }
  .field input { font-size: 14px; color: var(--ink); }
  input:disabled { opacity: 0.5; }
  .check { display: flex; gap: 10px; align-items: flex-start; }
  .check:has(select) { justify-content: space-between; align-items: center; }
  .check input[type="checkbox"] { margin-top: 3px; }
  .block { display: block; }
</style>
