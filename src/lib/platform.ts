// Small shims so plugin calls degrade gracefully in browser/design mode.
import { isTauri, api } from "./api";

export async function openExternal(url: string) {
  if (isTauri) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener");
  }
}

export async function saveTextFile(defaultName: string, contents: string, ext = "md") {
  if (isTauri) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({ defaultPath: defaultName, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
    if (path) await api.saveText(path, contents);
  } else {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([contents], { type: "text/plain" }));
    a.download = defaultName;
    a.click();
  }
}

/** Pick a text file and return its contents; null if cancelled. */
export async function openTextFile(ext = "json"): Promise<string | null> {
  if (isTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({ multiple: false, directory: false, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
    if (!path) return null;
    return api.readText(path as string);
  }
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = `.${ext}`;
    input.onchange = () => {
      const f = input.files?.[0];
      if (!f) return resolve(null);
      f.text().then(resolve);
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}
