"""Builds the GitHub Pages site (English at the root, French under /fr/).

Edit the texts below, then run `python site/build.py` (the Pages workflow runs
it too). Output: site/index.html, site/fr/index.html, site/sitemap.xml.
"""

import colorsys
import html
import json
from datetime import date
from pathlib import Path

SITE = Path(__file__).parent
BASE = "https://alxs-github.github.io/ALXS-RL-Mod/"
REPO = "https://github.com/ALXS-GitHub/ALXS-RL-Mod"
RELEASES = f"{REPO}/releases/latest"

GITHUB = '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2.2c-3.3.7-4-1.4-4-1.4-.6-1.4-1.4-1.8-1.4-1.8-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C17.3 4.6 18.3 5 18.3 5c.7 1.7.2 2.9.1 3.2.8.8 1.2 1.9 1.2 3.2 0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .5z"/></svg>'
DOWNLOAD = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/></svg>'
ARROW = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12h14"/><path d="m13 6 6 6-6 6"/></svg>'
# Top view of an arena, hairlines only (maps tile).
PITCH = (
    '<svg class="pitch" viewBox="0 0 240 150" fill="none" aria-hidden="true">'
    '<rect x="14" y="8" width="212" height="134" rx="26" />'
    '<path d="M120 8v134" /><circle cx="120" cy="75" r="20" />'
    '<path class="blue" d="M14 56h-8v38h8" /><path class="orange" d="M226 56h8v38h-8" />'
    '<circle class="dot" cx="120" cy="75" r="3" />'
    "</svg>"
)

# The two in-game captures of the showcase: same hybrid decal, two paints.
PAINTS = [("#eb1412", "#ce00ff"), ("#e3a23b", "#2b1245")]

TEXT = {
    "en": {
        "path": "",
        "html_lang": "en",
        "title": "ALXS-RL-Mod — Rocket League item swaps, custom decals & maps (no injection)",
        "description": "Free open-source app for Rocket League on PC (Epic Games): item swaps, AlphaConsole-style custom decals in real colors, custom ball, team color palettes, Workshop maps, presets and a match tracker. File-based, no DLL injection.",
        "og_locale": "en_US",
        "nav": ["Features", "How it works", "FAQ"],
        "eyebrow": "Free and open source · Windows · Epic Games",
        "h1": 'Customize Rocket League, <span class="dim">file by file.</span>',
        "lead": "Item swaps, custom decals in real colors, your own ball, team color palettes and Workshop maps — straight from your game files. Nothing injected, every change undone in one click.",
        "download": "Download for Windows",
        "github": "Source on GitHub",
        "meta": 'Latest <span data-version>release</span> · auto-updates · GPL-3.0',
        "show_alt": "Octane in Rocket League with a hybrid custom decal: the artwork keeps its colors, the painted zones follow the player's paint",
        "show_label": "Paint in game",
        "show_paints": ["Red, violet", "Gold, plum"],
        "show_caption": "Real capture. The artwork keeps its own colors, the painted zones follow your primary and accent.",
        "features_title": "Everything the plugins did. Without the plugins.",
        "features_sub": "What AlphaConsole and BakkesMod plugins used to do, rebuilt on top of your local game files.",
        "decals_title": "Custom decals in real colors",
        "decals_text": "AlphaConsole-style packs where the artwork keeps its own colors while chosen zones take your paint. Your existing packs are converted in one click.",
        "mask": [("Artwork", "kept as is"), ("Red", "primary color"), ("Green", "accent color")],
        "bodies": ["Octane", "Dominus", "Fennec", "Universal"],
        "swaps_title": "Item swaps",
        "swaps_text": "Equip what you own, see what you want. The catalog is read from your own install, every season.",
        "swaps": [("Boost", "Standard", "Gold Rush"), ("Wheels", "Cristiano", "Goldstone")],
        "ball_title": "Custom ball",
        "ball_text": "Your own image on the standard ball.",
        "palette_title": "Color palettes",
        "palette_text": "Replace the in-game primary and accent pickers with your own colors.",
        "tracker_title": "Match tracker",
        "tracker_text": "Session record, live score and an in-game overlay from the official Stats API.",
        "tracker_session": "This session",
        "maps_title": "Workshop maps",
        "maps_text": "Community maps from bakkesplugins, Lethamyr or your own files, swapped in even while the game runs.",
        "presets_title": "Presets",
        "presets_text": "A whole loadout in one click. Share it as a code.",
        "import_title": "BakkesMod import",
        "import_text": "Maps, AlphaConsole packs and balls moved into the app and converted.",
        "files_title": "Only files. Nothing injected.",
        "files_text": "Rocket League loads cosmetics from cooked packages. ALXS-RL-Mod rebuilds the few you customize and drops them where the game looks for overrides. The originals stay backed up and come back on demand, after a game update or when you uninstall.",
        "facts": [
            "No DLL injection, no network interception, no certificates",
            "Only you see the result, other players see your real items",
            "Rebuilt automatically after a game update",
            "One click, or uninstalling, restores the stock game",
        ],
        "tree_notes": ["stock, never edited", "your decal texture", "rebuilt override", "hashed originals"],
        "steps_title": "Three steps to the garage",
        "steps": [
            ("Install the app", "Run the installer. The app finds your Epic Games install of Rocket League and keeps itself up to date."),
            ("Import your keys.txt", "Rocket League encrypts part of its packages. Import a community <code>keys.txt</code> to enable swaps, decals and the ball. The app cannot ship these keys."),
            ("Customize and play", "Pick swaps, a decal pack, a ball or a map, then launch the game. One click puts the originals back."),
        ],
        "faq_title": "Questions",
        "faq_sub": 'Something else? <a href="https://github.com/ALXS-GitHub/ALXS-RL-Mod/issues">Open an issue</a>.',
        "faq": [
            ("Is ALXS-RL-Mod free?", "Yes. It is free and open source under the GPL-3.0 license. The code is on GitHub."),
            ("Does it inject into the game like BakkesMod?", "No. It never injects code and never touches network traffic: it only edits local game files, like a manual file mod, with a backup of each one."),
            ("Can other players see my items?", "No. Swaps, decals, palettes and the ball are local: only you see them. Other players see your real items."),
            ("Can I get banned?", "The app works on local files only. Modifying game files still goes against the game's terms of use, so use it at your own risk."),
            ("Where do I get keys.txt?", "The Rocket League modding community shares an up-to-date list of package keys (for example alongside package tools such as RLUPKTool). The app does not ship them; import the file from the welcome screen or the settings."),
            ("Does it work on Steam, Mac or console?", "Windows with the Epic Games version of Rocket League only, for now."),
            ("How do I remove everything?", "Settings → Restore the stock game, or uninstall the app: the uninstaller puts the original files back first."),
        ],
        "cta_title": "Free. Open source.<br>Ready for kickoff.",
        "cta_sub": "Windows 10 and 11 · Rocket League on Epic Games",
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
        "eyebrow": "Gratuit et open source · Windows · Epic Games",
        "h1": 'Personnalise Rocket League, <span class="dim">fichier par fichier.</span>',
        "lead": "Swaps d'items, stickers custom en vraies couleurs, ta propre balle, palettes de couleurs d'équipe et maps Workshop — directement depuis tes fichiers du jeu. Rien d'injecté, tout s'annule en un clic.",
        "download": "Télécharger pour Windows",
        "github": "Code source sur GitHub",
        "meta": 'Dernière <span data-version>version</span> · mises à jour auto · GPL-3.0',
        "show_alt": "Octane dans Rocket League avec un sticker custom hybride : l'image garde ses couleurs, les zones peintes suivent la peinture du joueur",
        "show_label": "Peinture en jeu",
        "show_paints": ["Rouge, violet", "Or, prune"],
        "show_caption": "Capture réelle. L'image garde ses propres couleurs, les zones peintes suivent ta couleur principale et ton accent.",
        "features_title": "Tout ce que faisaient les plugins. Sans les plugins.",
        "features_sub": "Ce que faisaient AlphaConsole et les plugins BakkesMod, reconstruit à partir de tes fichiers du jeu.",
        "decals_title": "Stickers custom en vraies couleurs",
        "decals_text": "Des packs façon AlphaConsole où l'image garde ses couleurs pendant que certaines zones prennent ta peinture. Tes packs existants se convertissent en un clic.",
        "mask": [("Image", "gardée telle quelle"), ("Rouge", "couleur principale"), ("Vert", "couleur d'accent")],
        "bodies": ["Octane", "Dominus", "Fennec", "Universel"],
        "swaps_title": "Swaps d'items",
        "swaps_text": "Équipe ce que tu as, vois ce que tu veux. Le catalogue est lu dans ton installation, à chaque saison.",
        "swaps": [("Boost", "Standard", "Ruée vers l'or"), ("Roues", "Cristiano", "Pierre d'or")],
        "ball_title": "Balle custom",
        "ball_text": "Ta propre image sur la balle standard.",
        "palette_title": "Palettes de couleurs",
        "palette_text": "Remplace les nuanciers principal et d'accent du jeu par tes couleurs.",
        "tracker_title": "Tracker de match",
        "tracker_text": "Bilan de session, score en direct et overlay en jeu via la Stats API officielle.",
        "tracker_session": "Cette session",
        "maps_title": "Maps Workshop",
        "maps_text": "Des maps de bakkesplugins, Lethamyr ou tes fichiers, changées même jeu lancé.",
        "presets_title": "Presets",
        "presets_text": "Un loadout complet en un clic. Partage-le avec un code.",
        "import_title": "Import BakkesMod",
        "import_text": "Maps, packs AlphaConsole et balles récupérés dans l'app et convertis.",
        "files_title": "Seulement des fichiers. Rien d'injecté.",
        "files_text": "Rocket League charge ses cosmétiques depuis des packages cuits. ALXS-RL-Mod reconstruit les quelques-uns que tu personnalises et les dépose là où le jeu cherche ses surcharges. Les originaux restent sauvegardés et reviennent à la demande, après une mise à jour du jeu ou à la désinstallation.",
        "facts": [
            "Pas d'injection de DLL, pas d'interception réseau, pas de certificat",
            "Toi seul vois le résultat, les autres voient tes vrais items",
            "Reconstruit automatiquement après une mise à jour du jeu",
            "Un clic, ou la désinstallation, remet le jeu d'origine",
        ],
        "tree_notes": ["d'origine, jamais modifié", "la texture de ton sticker", "surcharge reconstruite", "originaux hachés"],
        "steps_title": "Trois étapes jusqu'au garage",
        "steps": [
            ("Installe l'app", "Lance l'installeur. L'app trouve ton installation Epic Games de Rocket League et se met à jour toute seule."),
            ("Importe ton keys.txt", "Rocket League chiffre une partie de ses packages. Importe un <code>keys.txt</code> de la communauté pour activer les swaps, les stickers et la balle. L'app ne peut pas fournir ces clés."),
            ("Personnalise et joue", "Choisis tes swaps, un pack de stickers, une balle ou une map, puis lance le jeu. Un clic remet les originaux."),
        ],
        "faq_title": "Questions",
        "faq_sub": 'Autre chose ? <a href="https://github.com/ALXS-GitHub/ALXS-RL-Mod/issues">Ouvre une issue</a>.',
        "faq": [
            ("ALXS-RL-Mod est-il gratuit ?", "Oui. Il est gratuit et open source sous licence GPL-3.0. Le code est sur GitHub."),
            ("Est-ce que ça s'injecte dans le jeu comme BakkesMod ?", "Non. L'app n'injecte jamais de code et ne touche pas au trafic réseau : elle modifie seulement des fichiers locaux, comme un mod de fichiers manuel, avec une sauvegarde de chacun."),
            ("Les autres joueurs voient-ils mes items ?", "Non. Les swaps, stickers, palettes et la balle sont locaux : toi seul les vois. Les autres joueurs voient tes vrais items."),
            ("Est-ce que je risque un ban ?", "L'app travaille seulement sur des fichiers locaux. Modifier les fichiers du jeu va quand même à l'encontre de ses conditions d'utilisation : utilise-la à tes risques."),
            ("Où trouver keys.txt ?", "La communauté de modding Rocket League partage une liste à jour des clés des packages (par exemple avec des outils de packages comme RLUPKTool). L'app ne les fournit pas ; importe le fichier depuis l'écran d'accueil ou les réglages."),
            ("Steam, Mac ou console ?", "Windows avec la version Epic Games de Rocket League uniquement, pour l'instant."),
            ("Comment tout enlever ?", "Réglages → Restaurer le jeu d'origine, ou désinstalle l'app : le désinstalleur remet d'abord les fichiers d'origine."),
        ],
        "cta_title": "Gratuit. Open source.<br>Prêt pour le coup d'envoi.",
        "cta_sub": "Windows 10 et 11 · Rocket League sur Epic Games",
        "footer": "ALXS-RL-Mod est un projet de fan, ni affilié ni approuvé par Psyonix ou Epic Games. Rocket League est une marque de Psyonix LLC. Aucun fichier du jeu ni aucune clé de chiffrement n'est distribué.",
        "other_lang": ("English", "../"),
        "license": "Licence GPL-3.0",
    },
}


def esc(s):
    return html.escape(s, quote=True)


def swatches():
    """A palette-like grid: 3 rows of 12 hues, light to deep."""
    cells = []
    for light, sat in ((0.62, 0.9), (0.5, 0.95), (0.36, 0.85)):
        for i in range(12):
            r, g, b = colorsys.hls_to_rgb(i / 12, light, sat)
            cells.append(f'<i style="background:#{int(r * 255):02x}{int(g * 255):02x}{int(b * 255):02x}"></i>')
    return "".join(cells)


def session_dots():
    # W/L of an illustrative session.
    return "".join(f'<i class="{c}"></i>' for c in "wwlwwwlwlw")


def page(lang):
    t = TEXT[lang]
    up = "../" if t["path"] else ""
    url = BASE + t["path"]
    other_name, other_href = t["other_lang"]

    paints = "\n".join(
        f'            <button type="button" class="paint" data-paint="{i}" aria-pressed="{"true" if i == 0 else "false"}">'
        f'<span class="dots"><i style="background:{a}"></i><i style="background:{b}"></i></span>{esc(label)}</button>'
        for i, ((a, b), label) in enumerate(zip(PAINTS, t["show_paints"]))
    )
    mask = "".join(
        f'<li><i class="m{i}"></i><span>{esc(name)}</span><em>{esc(role)}</em></li>' for i, (name, role) in enumerate(t["mask"])
    )
    bodies = "".join(f"<li>{esc(b)}</li>" for b in t["bodies"])
    swaps = "".join(
        f'<li><span class="slot">{esc(slot)}</span><span class="own">{esc(own)}</span>{ARROW}<span class="see">{esc(see)}</span></li>'
        for slot, own, see in t["swaps"]
    )
    facts = "".join(f"<li>{esc(f)}</li>" for f in t["facts"])
    n = t["tree_notes"]
    tree = (
        f'<span class="dir">rocketleague/TAGame/CookedPCConsole/</span>\n'
        f'├── Body_Octane_SF.upk          <span class="note"># {esc(n[0])}</span>\n'
        f'├── <span class="new">AlxsDecal0.tfc</span>              <span class="note"># {esc(n[1])}</span>\n'
        f'└── <span class="dir">mods/</span>\n'
        f'    └── <span class="new">Skin_Octane_*_SF.upk</span>  <span class="note"># {esc(n[2])}</span>\n'
        f'\n'
        f'<span class="dir">%LOCALAPPDATA%/ALXS-RL-Mod/</span>\n'
        f'└── <span class="dir">backups/</span>                  <span class="note"># {esc(n[3])}</span>'
    )
    steps = "\n".join(
        f'        <li data-reveal><span class="num">0{i + 1}</span><h3>{esc(h)}</h3><p>{p}</p></li>'
        for i, (h, p) in enumerate(t["steps"])
    )
    faq = "\n".join(
        f"          <details><summary>{esc(q)}</summary><p>{esc(a)}</p></details>" for q, a in t["faq"]
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

    return f"""<!doctype html>
<html lang="{t['html_lang']}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{esc(t['title'])}</title>
  <meta name="description" content="{esc(t['description'])}" />
  <meta name="keywords" content="Rocket League, Rocket League mod, item swap, custom decals, AlphaConsole alternative, BakkesMod alternative, Rocket League custom ball, Rocket League workshop maps, Rocket League palette, Epic Games" />
  <link rel="canonical" href="{url}" />
  <link rel="alternate" hreflang="en" href="{BASE}" />
  <link rel="alternate" hreflang="fr" href="{BASE}fr/" />
  <link rel="alternate" hreflang="x-default" href="{BASE}" />
  <meta name="theme-color" content="#07080c" />
  <link rel="icon" href="{up}favicon.ico" sizes="any" />
  <link rel="icon" type="image/png" href="{up}favicon.png" />
  <link rel="apple-touch-icon" href="{up}assets/apple-touch-icon.png" />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="ALXS-RL-Mod" />
  <meta property="og:title" content="{esc(t['title'])}" />
  <meta property="og:description" content="{esc(t['description'])}" />
  <meta property="og:url" content="{url}" />
  <meta property="og:image" content="{BASE}assets/og-image.png" />
  <meta property="og:image:width" content="1280" />
  <meta property="og:image:height" content="640" />
  <meta property="og:locale" content="{t['og_locale']}" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="{esc(t['title'])}" />
  <meta name="twitter:description" content="{esc(t['description'])}" />
  <meta name="twitter:image" content="{BASE}assets/og-image.png" />
  <meta name="google-site-verification" content="Kb-Gvoigv3NTgHJs2SlbKMZcJZoq9qtoNNlRDVpTiys" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Archivo:wdth,wght@62..125,500..900&family=Inter:wght@400..700&family=JetBrains+Mono:wght@400;500&display=swap" />
  <link rel="stylesheet" href="{up}styles.css" />
  <script>document.documentElement.classList.add("js")</script>
  <script type="application/ld+json">{json.dumps(software, ensure_ascii=False)}</script>
  <script type="application/ld+json">{json.dumps(faq_ld, ensure_ascii=False)}</script>
</head>
<body>
  <header class="top">
    <div class="wrap">
      <a class="brand" href="{url}"><img src="{up}assets/logo.png" alt="" width="28" height="28" />ALXS-RL-Mod</a>
      <nav class="links">
        <a class="hide-sm" href="#features">{t['nav'][0]}</a>
        <a class="hide-sm" href="#files">{t['nav'][1]}</a>
        <a class="hide-sm" href="#faq">{t['nav'][2]}</a>
        <a href="{other_href}" hreflang="{'fr' if lang == 'en' else 'en'}">{other_name}</a>
        <a class="gh" href="{REPO}" rel="noopener" aria-label="GitHub">{GITHUB}</a>
      </nav>
    </div>
  </header>

  <main>
    <section class="hero">
      <div class="stage" aria-hidden="true"><canvas data-field="hero"></canvas></div>
      <div class="wrap hero-grid">
        <div class="hero-copy">
          <p class="eyebrow">{esc(t['eyebrow'])}</p>
          <h1>{t['h1']}</h1>
          <p class="lead">{esc(t['lead'])}</p>
          <div class="cta">
            <a class="btn primary" href="{RELEASES}" data-download>{DOWNLOAD}{esc(t['download'])}</a>
            <a class="btn ghost" href="{REPO}" rel="noopener">{GITHUB}{esc(t['github'])}</a>
          </div>
          <p class="meta">{t['meta']}</p>
        </div>
        <figure class="showcase" data-tilt>
          <div class="frame">
            <img class="shot on" src="{up}assets/decal-a.jpg" alt="{esc(t['show_alt'])}" width="1080" height="840" fetchpriority="high" />
            <img class="shot" src="{up}assets/decal-b.jpg" alt="" width="1080" height="840" loading="lazy" />
            <div class="paints" role="group" aria-label="{esc(t['show_label'])}">
              <span class="label">{esc(t['show_label'])}</span>
{paints}
            </div>
          </div>
          <figcaption>{esc(t['show_caption'])}</figcaption>
        </figure>
      </div>
    </section>

    <section id="features" class="features">
      <div class="wrap">
        <header class="head" data-reveal>
          <h2>{esc(t['features_title'])}</h2>
          <p>{esc(t['features_sub'])}</p>
        </header>
        <div class="bento">
          <article class="tile t-decals" data-reveal>
            <div class="copy">
              <div><h3>{esc(t['decals_title'])}</h3><p>{esc(t['decals_text'])}</p></div>
              <ul class="bodies">{bodies}</ul>
            </div>
            <div class="decal-viz"><ul class="mask">{mask}</ul></div>
          </article>
          <article class="tile t-swaps" data-reveal>
            <div class="copy"><h3>{esc(t['swaps_title'])}</h3><p>{esc(t['swaps_text'])}</p></div>
            <ul class="swaps">{swaps}</ul>
          </article>
          <article class="tile t-ball" data-reveal>
            <div class="ball" aria-hidden="true"></div>
            <div class="copy"><h3>{esc(t['ball_title'])}</h3><p>{esc(t['ball_text'])}</p></div>
          </article>
          <article class="tile t-palette" data-reveal>
            <div class="swatches" aria-hidden="true">{swatches()}</div>
            <div class="copy"><h3>{esc(t['palette_title'])}</h3><p>{esc(t['palette_text'])}</p></div>
          </article>
          <article class="tile t-tracker" data-reveal>
            <div class="score" aria-hidden="true">
              <div class="line"><span>{esc(t['tracker_session'])}</span><b>7 – 3</b></div>
              <div class="wl">{session_dots()}</div>
            </div>
            <div class="copy"><h3>{esc(t['tracker_title'])}</h3><p>{esc(t['tracker_text'])}</p></div>
          </article>
          <article class="tile t-maps" data-reveal>
            {PITCH}
            <div class="copy"><h3>{esc(t['maps_title'])}</h3><p>{esc(t['maps_text'])}</p></div>
          </article>
          <article class="tile t-presets" data-reveal>
            <div class="copy"><h3>{esc(t['presets_title'])}</h3><p>{esc(t['presets_text'])}</p></div>
            <code class="share" aria-hidden="true">ALXS1·k3Fq9·Zr2Lw·p7Tn</code>
          </article>
          <article class="tile t-import" data-reveal>
            <div class="copy"><h3>{esc(t['import_title'])}</h3><p>{esc(t['import_text'])}</p></div>
            <div class="flow" aria-hidden="true"><code>bakkesmod/data</code>{ARROW}<code>ALXS-RL-Mod</code></div>
          </article>
        </div>
      </div>
    </section>

    <section id="files" class="files">
      <div class="wrap files-grid">
        <div data-reveal>
          <h2>{esc(t['files_title'])}</h2>
          <p class="body">{esc(t['files_text'])}</p>
          <ul class="facts">{facts}</ul>
        </div>
        <pre class="tree" data-reveal><code>{tree}</code></pre>
      </div>
    </section>

    <section class="steps-sec">
      <div class="wrap">
        <h2 data-reveal>{esc(t['steps_title'])}</h2>
        <ol class="steps">
{steps}
        </ol>
      </div>
    </section>

    <section id="faq" class="faq">
      <div class="wrap faq-grid">
        <div>
          <h2>{esc(t['faq_title'])}</h2>
          <p class="sub">{t['faq_sub']}</p>
        </div>
        <div class="qa">
{faq}
        </div>
      </div>
    </section>

    <section class="finale">
      <div class="stage" aria-hidden="true"><canvas data-field="finale"></canvas></div>
      <div class="wrap">
        <h2>{t['cta_title']}</h2>
        <div class="cta">
          <a class="btn primary" href="{RELEASES}" data-download>{DOWNLOAD}{esc(t['download'])}</a>
          <a class="btn ghost" href="{REPO}" rel="noopener">{GITHUB}GitHub</a>
        </div>
        <p class="meta">{esc(t['cta_sub'])}</p>
      </div>
    </section>
  </main>

  <footer>
    <div class="wrap">
      <a class="brand" href="{url}"><img src="{up}assets/logo.png" alt="" width="22" height="22" />ALXS-RL-Mod</a>
      <p class="legal">{esc(t['footer'])}</p>
      <p class="links">© <span data-year></span> ALXS · <a href="{REPO}/blob/main/LICENSE">{t['license']}</a> · <a href="{REPO}">GitHub</a> · <a href="{other_href}">{other_name}</a></p>
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
