"""Builds the GitHub Pages site (English at the root, French under /fr/).

Edit the texts below, then run `python site/build.py` (the Pages workflow runs
it too). Output: site/index.html, site/fr/index.html, site/sitemap.xml.
"""

import html
import json
from datetime import date
from pathlib import Path

SITE = Path(__file__).parent
BASE = "https://alxs-github.github.io/ALXS-RL-Mod/"
REPO = "https://github.com/ALXS-GitHub/ALXS-RL-Mod"
RELEASES = f"{REPO}/releases/latest"

ICONS = {
    "swap": '<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>',
    "brush": '<path d="m9.06 11.9 8.07-8.06a2.85 2.85 0 1 1 4.03 4.03l-8.06 8.08"/><path d="M7.07 14.94c-1.66 0-3 1.35-3 3.02 0 1.33-2.5 1.52-2 2.02 1.08 1.1 2.49 2.02 4 2.02 2.2 0 4-1.8 4-4.04a3.01 3.01 0 0 0-3-3.02z"/>',
    "ball": '<circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20"/><path d="M2 12h20"/>',
    "palette": '<circle cx="13.5" cy="6.5" r=".5"/><circle cx="17.5" cy="10.5" r=".5"/><circle cx="8.5" cy="7.5" r=".5"/><circle cx="6.5" cy="12.5" r=".5"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.93 0 1.65-.75 1.65-1.69 0-.44-.18-.84-.44-1.13-.29-.29-.44-.65-.44-1.13a1.64 1.64 0 0 1 1.67-1.67h2c3.05 0 5.55-2.5 5.55-5.55C21.97 6.01 17.46 2 12 2z"/>',
    "map": '<path d="M14.1 5.55a2 2 0 0 0 1.8 0l3.65-1.83A1 1 0 0 1 21 4.62v12.76a1 1 0 0 1-.55.9l-4.56 2.27a2 2 0 0 1-1.78 0l-4.22-2.1a2 2 0 0 0-1.78 0l-3.66 1.83A1 1 0 0 1 3 19.38V6.62a1 1 0 0 1 .55-.9l4.56-2.27a2 2 0 0 1 1.78 0z"/><path d="M15 5.76v15"/><path d="M9 3.24v15"/>',
    "layers": '<path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65"/><path d="m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"/>',
    "activity": '<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>',
    "import": '<path d="M12 3v12"/><path d="m8 11 4 4 4-4"/><path d="M8 5H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-4"/>',
    "shield": '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>',
    "download": '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/><path d="M12 15V3"/>',
}
GITHUB = '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2.2c-3.3.7-4-1.4-4-1.4-.6-1.4-1.4-1.8-1.4-1.8-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C17.3 4.6 18.3 5 18.3 5c.7 1.7.2 2.9.1 3.2.8.8 1.2 1.9 1.2 3.2 0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .5z"/></svg>'


def icon(name, cls=""):
    return (
        f'<span class="icon {cls}"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" '
        f'stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{ICONS[name]}</svg></span>'
    )


def svg(name):
    return (
        f'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" '
        f'stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{ICONS[name]}</svg>'
    )


TEXT = {
    "en": {
        "path": "",
        "html_lang": "en",
        "title": "ALXS-RL-Mod — Rocket League item swaps, custom decals & maps (no injection)",
        "description": "Free open-source app for Rocket League on PC (Epic Games): item swaps, AlphaConsole-style custom decals in real colors, custom ball, team color palettes, Workshop maps, presets and a match tracker. File-based, no DLL injection.",
        "og_locale": "en_US",
        "nav": ["Features", "How it works", "FAQ"],
        "pill": "Free · Open source · Windows",
        "h1": 'Customize <span class="accent">Rocket League</span> without injection',
        "lead": "Item swaps, custom decals in real colors, your own ball, team color palettes and Workshop maps — straight from your game files. Every change is backed up and undone in one click.",
        "download": "Download for Windows",
        "github": "View on GitHub",
        "meta": 'Latest version: <span data-version>see releases</span> · Rocket League on PC, Epic Games version',
        "hero_alt": "Rocket League Octane with a custom full-color decal whose painted zones follow the player's colors",
        "features_title": "Everything AlphaConsole and BakkesMod plugins did — file-based",
        "features_sub": "ALXS-RL-Mod edits only your local game files. Nothing is injected into the game and its network traffic is never touched. Only you see the result.",
        "features": [
            ("swap", "", "Item swaps", "Equip an item you own, see any other one instead: wheels, boosts, decals, toppers, antennas, goal explosions and more."),
            ("brush", "orange", "Custom decals in real colors", "AlphaConsole-style packs where the image keeps its own colors and chosen zones take your primary and accent colors. Universal decals too."),
            ("ball", "", "Custom ball", "Put your own image on the standard ball."),
            ("palette", "orange", "Color palettes", "Replace the game's primary and accent color pickers with your own palette."),
            ("map", "", "Workshop maps", "Community maps from bakkesplugins, Lethamyr or your own files, swapped in even while the game runs."),
            ("layers", "orange", "Presets", "Save a whole loadout and switch to it in one click. Share it with a code."),
            ("activity", "", "Match tracker", "Session record, live match and in-game overlay from the game's official Stats API. Optional MMR from tracker.gg."),
            ("import", "orange", "BakkesMod import", "Bring your Workshop maps, AlphaConsole decal packs and balls into the app in one step."),
            ("shield", "", "Safe by design", "Backups for every file, one-click restore, automatic rebuild after game updates and auto-updates."),
        ],
        "how_title": "How it works",
        "how_sub": "Three steps, no configuration file to edit.",
        "how_alt": "A custom decal previewed in Rocket League's garage",
        "steps": [
            ("Install the app", "Download the installer and run it. The app finds your Epic Games install of Rocket League and updates itself."),
            ("Import your keys.txt", "Rocket League encrypts part of its packages. Import a community <code>keys.txt</code> to enable swaps, decals and the ball — the app cannot ship these keys."),
            ("Customize and play", "Pick swaps, a decal pack, a ball or a map. The app rebuilds the few files it needs; one click puts the originals back."),
        ],
        "faq_title": "Frequently asked questions",
        "faq": [
            ("Is ALXS-RL-Mod free?", "Yes. It is free and open source under the GPL-3.0 license. The code is on GitHub."),
            ("Does it inject into the game like BakkesMod?", "No. It never injects code and never touches network traffic: it only edits local game files, like a manual file mod, with a backup of each one."),
            ("Can other players see my items?", "No. Swaps, decals, palettes and the ball are local: only you see them. Other players see your real items."),
            ("Can I get banned?", "The app works on local files only. Modifying game files still goes against the game's terms of use, so use it at your own risk."),
            ("Where do I get keys.txt?", "The Rocket League modding community shares an up-to-date list of package keys (for example alongside package tools such as RLUPKTool). The app does not ship them; import the file from the welcome screen or the settings."),
            ("Does it work on Steam, Mac or console?", "Windows with the Epic Games version of Rocket League only, for now."),
            ("How do I remove everything?", "Settings → Restore the stock game, or uninstall the app: the uninstaller puts the original files back first."),
        ],
        "band_title": "Ready to customize your car?",
        "band_sub": "Free download for Windows. Open source on GitHub.",
        "footer": "ALXS-RL-Mod is a fan-made project, not affiliated with or endorsed by Psyonix or Epic Games. Rocket League is a trademark of Psyonix LLC. No game file or encryption key is distributed.",
        "other_lang": ("Français", "fr/"),
        "license": "GPL-3.0 license",
    },
    "fr": {
        "path": "fr/",
        "html_lang": "fr",
        "title": "ALXS-RL-Mod — Swaps d'items, stickers custom et maps pour Rocket League (sans injection)",
        "description": "App gratuite et open source pour Rocket League sur PC (Epic Games) : swaps d'items, stickers custom façon AlphaConsole en vraies couleurs, balle custom, palettes de couleurs, maps Workshop, presets et tracker de match. Sans injection de DLL.",
        "og_locale": "fr_FR",
        "nav": ["Fonctionnalités", "Fonctionnement", "FAQ"],
        "pill": "Gratuit · Open source · Windows",
        "h1": 'Personnalise <span class="accent">Rocket League</span> sans injection',
        "lead": "Swaps d'items, stickers custom en vraies couleurs, ta propre balle, palettes de couleurs d'équipe et maps Workshop — directement depuis tes fichiers du jeu. Chaque modification est sauvegardée et annulable en un clic.",
        "download": "Télécharger pour Windows",
        "github": "Voir sur GitHub",
        "meta": 'Dernière version : <span data-version>voir les releases</span> · Rocket League sur PC, version Epic Games',
        "hero_alt": "Octane dans Rocket League avec un sticker custom en couleurs réelles dont les zones peintes suivent les couleurs du joueur",
        "features_title": "Tout ce que faisaient AlphaConsole et les plugins BakkesMod — via les fichiers",
        "features_sub": "ALXS-RL-Mod modifie seulement tes fichiers locaux. Rien n'est injecté dans le jeu et son trafic réseau n'est jamais touché. Toi seul vois le résultat.",
        "features": [
            ("swap", "", "Swaps d'items", "Équipe un item que tu possèdes, vois-en un autre à la place : roues, boosts, stickers, chapeaux, antennes, explosions de but…"),
            ("brush", "orange", "Stickers custom en vraies couleurs", "Des packs façon AlphaConsole où l'image garde ses couleurs et des zones prennent tes couleurs principale et d'accent. Stickers universels aussi."),
            ("ball", "", "Balle custom", "Mets ta propre image sur la balle standard."),
            ("palette", "orange", "Palettes de couleurs", "Remplace les nuanciers principal et d'accent du jeu par ta propre palette."),
            ("map", "", "Maps Workshop", "Des maps de bakkesplugins, Lethamyr ou tes fichiers, changées même jeu lancé."),
            ("layers", "orange", "Presets", "Enregistre un loadout complet et passe de l'un à l'autre en un clic. Partage-le avec un code."),
            ("activity", "", "Tracker de match", "Bilan de session, match en direct et overlay en jeu via la Stats API officielle. MMR tracker.gg en option."),
            ("import", "orange", "Import BakkesMod", "Récupère tes maps Workshop, tes packs AlphaConsole et tes balles en une étape."),
            ("shield", "", "Sûr par conception", "Sauvegarde de chaque fichier, restauration en un clic, reconstruction après les mises à jour du jeu et mises à jour automatiques."),
        ],
        "how_title": "Comment ça marche",
        "how_sub": "Trois étapes, aucun fichier de configuration à modifier.",
        "how_alt": "Un sticker custom prévisualisé dans le garage de Rocket League",
        "steps": [
            ("Installe l'app", "Télécharge l'installeur et lance-le. L'app trouve ton installation Epic Games de Rocket League et se met à jour toute seule."),
            ("Importe ton keys.txt", "Rocket League chiffre une partie de ses packages. Importe un <code>keys.txt</code> de la communauté pour activer les swaps, les stickers et la balle — l'app ne peut pas fournir ces clés."),
            ("Personnalise et joue", "Choisis tes swaps, un pack de stickers, une balle ou une map. L'app reconstruit les quelques fichiers nécessaires ; un clic remet les originaux."),
        ],
        "faq_title": "Questions fréquentes",
        "faq": [
            ("ALXS-RL-Mod est-il gratuit ?", "Oui. Il est gratuit et open source sous licence GPL-3.0. Le code est sur GitHub."),
            ("Est-ce que ça s'injecte dans le jeu comme BakkesMod ?", "Non. L'app n'injecte jamais de code et ne touche pas au trafic réseau : elle modifie seulement des fichiers locaux, comme un mod de fichiers manuel, avec une sauvegarde de chacun."),
            ("Les autres joueurs voient-ils mes items ?", "Non. Les swaps, stickers, palettes et la balle sont locaux : toi seul les vois. Les autres joueurs voient tes vrais items."),
            ("Est-ce que je risque un ban ?", "L'app travaille seulement sur des fichiers locaux. Modifier les fichiers du jeu va quand même à l'encontre de ses conditions d'utilisation : utilise-la à tes risques."),
            ("Où trouver keys.txt ?", "La communauté de modding Rocket League partage une liste à jour des clés des packages (par exemple avec des outils de packages comme RLUPKTool). L'app ne les fournit pas ; importe le fichier depuis l'écran d'accueil ou les réglages."),
            ("Steam, Mac ou console ?", "Windows avec la version Epic Games de Rocket League uniquement, pour l'instant."),
            ("Comment tout enlever ?", "Réglages → Restaurer le jeu d'origine, ou désinstalle l'app : le désinstalleur remet d'abord les fichiers d'origine."),
        ],
        "band_title": "Prêt à personnaliser ta voiture ?",
        "band_sub": "Téléchargement gratuit pour Windows. Open source sur GitHub.",
        "footer": "ALXS-RL-Mod est un projet de fan, ni affilié ni approuvé par Psyonix ou Epic Games. Rocket League est une marque de Psyonix LLC. Aucun fichier du jeu ni aucune clé de chiffrement n'est distribué.",
        "other_lang": ("English", "../"),
        "license": "Licence GPL-3.0",
    },
}


def page(lang):
    t = TEXT[lang]
    up = "../" if t["path"] else ""
    url = BASE + t["path"]
    features = "\n".join(
        f'        <article class="card">{icon(i, c)}<h3>{html.escape(h)}</h3><p>{html.escape(p)}</p></article>'
        for i, c, h, p in t["features"]
    )
    steps = "\n".join(f"          <li><strong>{html.escape(h)}</strong>{p}</li>" for h, p in t["steps"])
    faq = "\n".join(
        f"        <details><summary>{html.escape(q)}</summary><p>{html.escape(a)}</p></details>" for q, a in t["faq"]
    )
    software = {
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        "name": "ALXS-RL-Mod",
        "applicationCategory": "GameApplication",
        "applicationSubCategory": "Game modding tool",
        "operatingSystem": "Windows 10, Windows 11",
        "description": t["description"],
        "url": url,
        "downloadUrl": RELEASES,
        "softwareHelp": REPO,
        "license": "https://www.gnu.org/licenses/gpl-3.0.html",
        "image": BASE + "assets/og-image.png",
        "screenshot": BASE + "assets/hybrid-paint.jpg",
        "inLanguage": ["en", "fr"],
        "isAccessibleForFree": True,
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "EUR"},
        "author": {"@type": "Person", "name": "ALXS", "url": "https://github.com/ALXS-GitHub"},
    }
    faq_ld = {
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": [
            {"@type": "Question", "name": q, "acceptedAnswer": {"@type": "Answer", "text": a}} for q, a in t["faq"]
        ],
    }
    other_name, other_href = t["other_lang"]
    return f"""<!doctype html>
<html lang="{t['html_lang']}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{html.escape(t['title'])}</title>
  <meta name="description" content="{html.escape(t['description'])}" />
  <meta name="keywords" content="Rocket League, Rocket League mod, item swap, custom decals, AlphaConsole alternative, BakkesMod alternative, Rocket League custom ball, Rocket League workshop maps, Rocket League palette, Epic Games" />
  <link rel="canonical" href="{url}" />
  <link rel="alternate" hreflang="en" href="{BASE}" />
  <link rel="alternate" hreflang="fr" href="{BASE}fr/" />
  <link rel="alternate" hreflang="x-default" href="{BASE}" />
  <meta name="theme-color" content="#06070c" />
  <link rel="icon" href="{up}favicon.ico" sizes="any" />
  <link rel="icon" type="image/png" href="{up}favicon.png" />
  <link rel="apple-touch-icon" href="{up}assets/apple-touch-icon.png" />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="ALXS-RL-Mod" />
  <meta property="og:title" content="{html.escape(t['title'])}" />
  <meta property="og:description" content="{html.escape(t['description'])}" />
  <meta property="og:url" content="{url}" />
  <meta property="og:image" content="{BASE}assets/og-image.png" />
  <meta property="og:image:width" content="1280" />
  <meta property="og:image:height" content="640" />
  <meta property="og:locale" content="{t['og_locale']}" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="{html.escape(t['title'])}" />
  <meta name="twitter:description" content="{html.escape(t['description'])}" />
  <meta name="twitter:image" content="{BASE}assets/og-image.png" />
  <!-- Google Search Console: paste the verification meta tag here. -->
  <link rel="stylesheet" href="{up}styles.css" />
  <script type="application/ld+json">{json.dumps(software, ensure_ascii=False)}</script>
  <script type="application/ld+json">{json.dumps(faq_ld, ensure_ascii=False)}</script>
</head>
<body>
  <header class="top">
    <div class="wrap">
      <a class="brand" href="{url}"><img src="{up}assets/logo.png" alt="" width="30" height="30" />ALXS-RL-Mod</a>
      <nav class="links">
        <a class="hide-sm" href="#features">{t['nav'][0]}</a>
        <a class="hide-sm" href="#how">{t['nav'][1]}</a>
        <a class="hide-sm" href="#faq">{t['nav'][2]}</a>
        <a href="{other_href}" hreflang="{'fr' if lang == 'en' else 'en'}">{other_name}</a>
        <a class="btn small" href="{REPO}" rel="noopener">{GITHUB}GitHub</a>
      </nav>
    </div>
  </header>

  <main>
    <div class="wrap hero">
      <div>
        <span class="pill"><span class="dot"></span>{t['pill']}</span>
        <h1>{t['h1']}</h1>
        <p class="lead">{html.escape(t['lead'])}</p>
        <div class="cta">
          <a class="btn primary" href="{RELEASES}" data-download>{svg('download')}{t['download']}</a>
          <a class="btn" href="{REPO}" rel="noopener">{GITHUB}{t['github']}</a>
          <span class="meta">{t['meta']}</span>
        </div>
      </div>
      <div class="shot"><img src="{up}assets/hybrid-paint.jpg" alt="{html.escape(t['hero_alt'])}" width="1600" height="900" fetchpriority="high" /></div>
    </div>

    <section id="features">
      <div class="wrap">
        <h2>{html.escape(t['features_title'])}</h2>
        <p class="sub">{html.escape(t['features_sub'])}</p>
        <div class="grid">
{features}
        </div>
      </div>
    </section>

    <section id="how">
      <div class="wrap split">
        <div>
          <h2>{html.escape(t['how_title'])}</h2>
          <p class="sub">{html.escape(t['how_sub'])}</p>
          <ol class="steps">
{steps}
          </ol>
        </div>
        <div class="shot"><img src="{up}assets/hybrid-sticker.jpg" alt="{html.escape(t['how_alt'])}" width="1600" height="900" loading="lazy" /></div>
      </div>
    </section>

    <section id="faq" class="faq">
      <div class="wrap">
        <h2>{html.escape(t['faq_title'])}</h2>
{faq}
      </div>
    </section>

    <section>
      <div class="wrap band">
        <h2>{html.escape(t['band_title'])}</h2>
        <p>{html.escape(t['band_sub'])}</p>
        <div class="cta">
          <a class="btn primary" href="{RELEASES}" data-download>{svg('download')}{t['download']}</a>
          <a class="btn" href="{REPO}" rel="noopener">{GITHUB}GitHub</a>
        </div>
      </div>
    </section>
  </main>

  <footer>
    <div class="wrap">
      <p>{html.escape(t['footer'])}</p>
      <p>© <span data-year></span> ALXS · <a href="{REPO}/blob/main/LICENSE">{t['license']}</a> · <a href="{REPO}">GitHub</a></p>
    </div>
  </footer>
  <script src="{up}app.js" defer></script>
</body>
</html>
"""


def main():
    (SITE / "index.html").write_text(page("en"), encoding="utf-8", newline="\n")
    (SITE / "fr").mkdir(exist_ok=True)
    (SITE / "fr" / "index.html").write_text(page("fr"), encoding="utf-8", newline="\n")
    today = date.today().isoformat()
    urls = "\n".join(
        f"""  <url>
    <loc>{BASE}{p}</loc>
    <lastmod>{today}</lastmod>
    <xhtml:link rel="alternate" hreflang="en" href="{BASE}" />
    <xhtml:link rel="alternate" hreflang="fr" href="{BASE}fr/" />
  </url>"""
        for p in ("", "fr/")
    )
    (SITE / "sitemap.xml").write_text(
        f"""<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
{urls}
</urlset>
""",
        encoding="utf-8",
        newline="\n",
    )
    print("site built")


if __name__ == "__main__":
    main()
