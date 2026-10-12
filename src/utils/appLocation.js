export function getAppLocationPath(app) {
  for (const value of [app.target_path, app.data || app.path]) {
    if (typeof value !== "string") continue;
    const path = value.trim().replace(/^"|"$/g, "");
    // Shell namespace IDs and MSI icon metadata are not application folders.
    if (/\.ico$/i.test(path)) continue;
    if (/^[a-z]:[\\/]/i.test(path) || path.startsWith("\\\\") || path.startsWith("/")) {
      return path;
    }
  }
  return "";
}
