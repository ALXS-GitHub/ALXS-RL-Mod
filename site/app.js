// Points the download buttons at the installer of the latest release and
// shows its version. Without JavaScript (or if the GitHub API is rate
// limited) the buttons keep linking to the latest release page.
(async () => {
  const buttons = document.querySelectorAll("[data-download]");
  const versions = document.querySelectorAll("[data-version]");
  try {
    const res = await fetch("https://api.github.com/repos/ALXS-GitHub/ALXS-RL-Mod/releases/latest", {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return;
    const release = await res.json();
    const installer = (release.assets || []).find((a) => /setup\.exe$/i.test(a.name));
    if (installer) {
      for (const b of buttons) b.href = installer.browser_download_url;
    }
    for (const v of versions) v.textContent = release.tag_name;
  } catch {
    // Keep the static links.
  }
})();

for (const el of document.querySelectorAll("[data-year]")) {
  el.textContent = String(new Date().getFullYear());
}
