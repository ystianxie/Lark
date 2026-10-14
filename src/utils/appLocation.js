export function getAppLocationPath(app) {
  for (const value of [app.target_path, app.data || app.path]) {
    if (typeof value !== "string") continue;
    const path = value.trim().replace(/^"|"$/g, "");
    // Shell namespace IDs are virtual paths, but Explorer can open them.
    if (/\.ico$/i.test(path)) continue;
    if (/^shell:AppsFolder\\/i.test(path)
      || /^[a-z]:[\\/]/i.test(path) || path.startsWith("\\\\") || path.startsWith("/")) {
      return path;
    }
  }
  return "";
}
