# Elden Ring Overlay (hors ligne, lecture seule)

Overlay Rust injecté dans un `eldenring.exe` **déjà lancé**, en **lecture seule**. Il affiche un tableau de bord personnalisable : IGT, compteur de **boss**, Grandes Runes, morts, NG+, objets clés et **checklists boss/loot** (avec prise en charge de l'item randomizer)...

![Elden Ring Overlay](docs/overlay.png)

> **Lecture seule, hors ligne, pas de triche.** Aucune écriture mémoire, aucun contournement anti-cheat. Utilisation uniquement en solo hors ligne.

> **Note sur le développement.** Ce projet a été développé en grande partie avec l'aide d'un LLM (génération de code, refactorisation, documentation). Le code est relu et testé, mais gardez cela en tête si vous l'auditez ou le réutilisez.

---

## Sommaire

**Guide utilisateur**
- [Démarrage rapide (release GitHub)](#démarrage-rapide-release-github)
- [Avertissements](#avertissements)
- [Installation](#installation)
- [Configuration (`er_overlay.toml`)](#configuration-er_overlaytoml)
- [Checks (compatible randomizer)](#panneau-des-checks-compatible-randomizer)
- [Mode challenge](#mode-challenge)
- [Personnaliser l'affichage](#personnaliser-laffichage)
- [Éditeur de layout](#éditeur-de-layout)
- [Dépannage](#dépannage)

**Référence technique**
- [Architecture](#architecture)
- [Format de layout (référence)](#format-de-layout-référence)
- [Métriques disponibles](#métriques-disponibles)
- [Données du jeu (tables)](#données-du-jeu-tables)
- [Ajouter un good](#ajouter-un-good)
- [Icônes](#icônes)
- [Développement](#développement)
- [Références](#références)

---

# Guide utilisateur

## Démarrage rapide (release GitHub)

**Vous n'avez rien à compiler.** Téléchargez un zip précompilé depuis la page [GitHub Releases](https://github.com/Sully-/ER_DynamicOverlay/releases) (`er-overlay-vX.X.X.zip`), extrayez-le où vous voulez, puis suivez les étapes ci-dessous.

### 1. Lancez Elden Ring hors ligne

L'overlay **ne fonctionne pas avec EasyAntiCheat activé**. Lancez le jeu en mode hors ligne, par exemple :

- Lancez directement `eldenring.exe` (pas via le launcher EAC), avec un fichier `steam_appid.txt` contenant `1245620` à côté de l'exe, **ou**
- Utilisez votre méthode habituelle hors ligne / sans EAC.

Gardez le jeu lancé sur l'écran titre ou dans une sauvegarde : l'injecteur s'attache à un processus déjà en cours.

### 2. Lancez l'overlay

Après extraction du zip, vous devez avoir un seul dossier contenant au minimum :

| Fichier / dossier | Rôle |
|---------------|------|
| `er_overlay_injector.exe` | Lanceur — **double-cliquez dessus** |
| `er_overlay.dll` | Overlay (injecté dans le jeu) |
| `er_overlay.toml` | Paramètres (position, échelle, raccourcis clavier, fichier de layout…) |
| `layouts/` | Fichiers de layout du tableau de bord |
| `tables/` | Listes de boss / checks par langue |
| `assets/` | Icônes d'objets |
| `companion/er_checks_extractor.exe` | Assistant qui lit un `regulation.bin` de randomizer (voir [Panneau des checks](#panneau-des-checks-compatible-randomizer)) |
| `layout_editor.html` | Éditeur visuel de layout (voir étape 3) |
| `challenge_state.toml` | *(runtime)* PB / essais du challenge — créé quand `[challenge] enabled = true` |
| `checks_flags.toml` | *(runtime)* Flags randomizer par seed — créé uniquement quand `regulation_path` est défini |

**Ne séparez pas ces fichiers** : ils doivent rester dans le même dossier.

1. Avec Elden Ring déjà lancé hors ligne, **double-cliquez sur `er_overlay_injector.exe`**.
2. L'overlay apparaît en jeu (par défaut : HUD minimal en haut à droite). Ouvrez un panneau latéral avec `F7` (boss) ou `F6` (checks).

C'est tout. Relancez l'injecteur après chaque redémarrage du jeu (l'overlay ne persiste pas entre deux lancements).

**Raccourcis utiles** (valeurs par défaut dans `er_overlay.toml`, rechargées à chaud toutes les 2 s) :

| Touche | Action |
|-----|--------|
| `F8` | Changer de section de layout (`minimalist` → `extended` → `challenge`, …) |
| `F7` | Afficher / masquer le panneau de checklist des boss |
| `F6` | Afficher / masquer le panneau des checks (boss + checklist de loot, compatible randomizer) |
| `F9` | Afficher / masquer tout l'overlay (par défaut ; voir `hide_all_hotkey`) |

Le **panneau des boss**, le **panneau des checks** et la section de layout **extended** sont mutuellement exclusifs : en ouvrir un ferme les autres.

Si quelque chose se passe mal, consultez `logs/er_injector.log` et `logs/er_overlay.log` dans le même dossier.

### 3. Personnalisez votre tableau de bord (éditeur de layout)

Le zip de release inclut un **éditeur visuel** : pas besoin d'apprendre la syntaxe TOML.

![Éditeur de layout](docs/layout-editor.png)

1. Ouvrez **`layout_editor.html`** depuis le dossier extrait dans votre navigateur (Chrome, Edge, Firefox…).
   - Si l'import/export est bloqué, servez plutôt le dossier : ouvrez un terminal dans le dossier et lancez `python -m http.server`, puis allez sur `http://localhost:8000/layout_editor.html`.
2. **Glissez-déposez** des métriques, labels et objets depuis la palette de gauche vers la grille.
3. Ajustez la grille (colonnes, lignes, taille de cellule, espacement) et chaque tuile dans le panneau de droite.
4. Utilisez **Import layout file** pour modifier le fichier fourni `layouts/dashboard.toml`, ou partez de **New**.
5. Cliquez sur **Export layout file** et enregistrez le `.toml` dans le dossier `layouts/` (par exemple `layouts/my_run.toml`).
6. Modifiez `er_overlay.toml` et définissez `layout_file = "layouts/my_run.toml"`. L'overlay recharge automatiquement le fichier sous ~2 secondes (même pendant que le jeu tourne).

**Astuces :** créez plusieurs **sections** dans un même fichier (par exemple une vue compacte et une vue complète) et passez de l'une à l'autre avec `F8`. Voir [Personnaliser l'affichage](#personnaliser-laffichage) pour comprendre le rôle de chaque type de tuile.

### 4. Ajustez l'apparence et le comportement

Ouvrez `er_overlay.toml` dans n'importe quel éditeur de texte. Options courantes :

- `anchor`, `offset_x`, `offset_y` — position à l'écran
- `scale`, `text_size`, `icon_size` — taille
- `background_opacity`, `gray_tint` — apparence des objets non possédés
- `boss_panel_visible`, `boss_panel_hotkey`, `boss_locale` — checklist des boss
- `checks_panel_visible`, `checks_panel_hotkey`, `checks_panel_scope` — panneau des checks
- `regulation_path` — pointe vers un `regulation.bin` de randomizer pour suivre le loot déplacé ; voir [Panneau des checks](#panneau-des-checks-compatible-randomizer)
- `[challenge]` — **mode challenge** optionnel (PB / runs échouées) ; voir [Mode challenge](#mode-challenge)

Référence complète : [Configuration](#configuration-er_overlaytoml).

---

## Avertissements

- **Hors ligne uniquement** — aucun support multijoueur / en ligne.
- **Ne contourne pas EAC** — lancez le jeu sans EasyAntiCheat (par exemple en lançant directement `eldenring.exe` avec `steam_appid.txt`).
- **Lecture seule** — aucune écriture mémoire, ce n'est pas un trainer.
- **Injection transparente et documentée** (`LoadLibraryW` via `CreateRemoteThread`), sans furtivité.

## Installation

### Depuis une release GitHub (recommandé)

Voir le **[Démarrage rapide](#démarrage-rapide-release-github)** ci-dessus. Prérequis :

- Windows **x64**
- Elden Ring **hors ligne**, version prise en charge par la release (actuellement **2.6.2.0 (WW)** et **2.6.2.1 (JP)** — voir [Dépannage](#dépannage) si les valeurs affichent `---`)

### Compiler depuis les sources

Pour les développeurs qui veulent compiler localement :

- Windows **x64**
- Une version d'Elden Ring prise en charge par [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) (`eldenring` 0.14, par exemple 2.6.x)
- Rust **1.85+**

```powershell
cd Overlay
cargo build --release
```

Artefacts dans `target/release/` :

- `er_overlay_injector.exe` — l'injecteur
- `er_overlay.dll` — l'overlay lui-même

La compilation copie `er_overlay.toml`, `layouts/`, `tables/<lang>/bosses.toml`, `tables/<lang>/checks.toml` et `assets/icons/` à côté des binaires. Pour produire localement un zip similaire à une release, avec l'assistant randomizer inclus : `.\tools\bundle_release.ps1`.

L'assistant randomizer (`companion/er_checks_extractor`) est un projet .NET séparé, publié en self-contained :

```powershell
dotnet publish companion/er_checks_extractor/er_checks_extractor.csproj -c Release
```

Le bundle de release le publie automatiquement dans `companion/er_checks_extractor.exe` à côté de la DLL (ou pointez `checks_extractor_path` vers un build personnalisé).

### Injecteur avancé (ligne de commande)

Pour des cas spécifiques, vous pouvez lancer l'injecteur depuis un terminal avec des options :

```powershell
# cibler un id de processus spécifique
.\er_overlay_injector.exe --pid 12345
# chemin explicite vers la DLL
.\er_overlay_injector.exe --dll ".\er_overlay.dll"
# tout valider sans injecter
.\er_overlay_injector.exe --dry-run
```

## Configuration (`er_overlay.toml`)

Lu à côté de la DLL, **rechargé à chaud toutes les 2 secondes** (vous pouvez le modifier pendant que le jeu tourne). Les valeurs hors limites sont ramenées à leur valeur par défaut avec un avertissement dans le log.

| Option | Type | Défaut | Description |
|--------|------|---------|-------------|
| `layout_file` | chemin | `layouts/dashboard.toml` | Fichier de layout à afficher (voir [Personnaliser l'affichage](#personnaliser-laffichage)). |
| `default_layout_section` | string | — | Section affichée au démarrage (remplace le `default_section` du layout). |
| `layout_section_hotkey` | string | — | Touche pour parcourir les sections, par exemple `"F8"`, `"Ctrl+Shift+F1"`. |
| `anchor` | enum | `top-right` | Coin d'ancrage : `top-left`, `top-right`, `bottom-left`, `bottom-right`. |
| `offset_x`, `offset_y` | px | `16`, `16` | Décalage depuis le coin d'ancrage. |
| `scale` | 0–4 | `1.0` | Échelle globale de l'overlay. |
| `text_size` | px (≤72) | `18` | Taille de police de base. |
| `icon_size` | px (≤128) | `24` | Taille d'icône de référence. |
| `background_opacity` | 0–1 | `0.65` | Opacité du fond de fenêtre. |
| `gray_tint` | 0–1 | `0.40` | Teinte des objets **non possédés** (plus bas = plus sombre). |
| `use_item_icons` | bool | `true` | `true` = vraies icônes PNG quand elles existent, sinon points colorés. |
| `icons_dir` | chemin | `assets/icons` | Dossier PNG (relatif à la DLL). |
| `show_debug` | bool | `false` | Affiche une fenêtre de diagnostic (backend, pointeurs résolus, flags chargés). |
| `boss_panel_hotkey` | string | `F7` | Afficher / masquer le panneau de checklist des boss. |
| `boss_panel_scope` | enum | `current-region` | `current-region` ou `all-regions`. |
| `boss_panel_visible` | bool | `true` | Afficher le panneau des boss au démarrage (le `er_overlay.toml` fourni met `false`). Un seul panneau boss / checks peut être affiché au démarrage ; boss gagne si les deux valent `true`. |
| `boss_panel_layout` | string | — | Panneau `x,y,width,height` (pixels ou `%`). Omettre ou `auto` = `"-5, 10, 25%, 92%"` (aligné à droite), déplacé sous le HUD minimal. x/y négatifs = décalage depuis le bord droit/bas. |
| `boss_locale` | string | `auto` | Langue de la table des boss (`en`, `fr`, …). `auto` lit la langue du jeu via Steam ; fallback sur `en`. |
| `checks_panel_hotkey` | string | `F6` | Afficher / masquer le panneau des checks (boss + checklist de loot). |
| `checks_panel_scope` | enum | `current-region` | `current-region` ou `all-regions` (le `er_overlay.toml` fourni met `all-regions`). |
| `checks_panel_visible` | bool | `false` | Afficher le panneau des checks au démarrage. Mutuellement exclusif avec le panneau des boss (boss gagne si les deux valent `true`). |
| `checks_panel_layout` | string | — | Panneau `x,y,width,height` (pixels ou `%`). Omettre ou `auto` = `"5, 10, 25%, 92%"` (aligné à gauche, miroir du panneau des boss). |
| `regulation_path` | chemin | — | Chemin vers le `regulation.bin` que le jeu **charge** (votre randomizer / mod ModEngine). Active la résolution par seed des flags de loot randomisé. Vide/omis = flags vanilla. Voir [Panneau des checks](#panneau-des-checks-compatible-randomizer). |
| `checks_extractor_path` | chemin | — | Remplace l'emplacement de l'exe assistant. Omettre pour auto-détecter `companion/er_checks_extractor.exe` (puis `er_checks_extractor.exe`) à côté de la DLL. |

### Mode challenge (`[challenge]`)

Ruleset optionnel inspiré du mode boss challenge de [EROverlay](https://github.com/soarqin/EROverlay). **Désactivé par défaut.**

| Option | Type | Défaut | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Quand `false`, les métriques challenge affichent `---` et aucune progression n'est suivie. |
| `max_deaths` | u32 | `0` | Morts autorisées **par run** (inclusif). La run échoue quand les morts de la run dépassent cette valeur. `0` = deathless. |
| `start_flag` | u32 | `101` | Event flag qui marque le **début d'une run** (flag `101` = sortie de la Grotte de la connaissance / Cimetière abandonné, comme EROverlay). |

Exemple :

```toml
[challenge]
enabled = true
max_deaths = 0      # deathless : une mort termine la run
start_flag = 101
```

**Fichier de progression :** `challenge_state.toml` (à côté de `er_overlay.dll`, créé à l'exécution). Stocke le record personnel (`pb`), le nombre de runs échouées (`nbtries` / `tries`) et l'état interne de la run. Supprimez ce fichier pour réinitialiser le PB et les essais.

Voir [Mode challenge](#mode-challenge) pour le comportement et les tuiles de layout.

## Checks (compatible randomizer)

Le **panneau des checks** est une checklist unique de tout ce qui vaut la peine d'être complété dans une run. Un *check* correspond à une action : un **boss à tuer** ou un **objet important à récupérer**. Voyez-le comme le panneau des boss, mais avec les items clés en plus.

### Utilisation de base

1. Lancez Elden Ring et lancez l'overlay (voir [Démarrage rapide](#démarrage-rapide-release-github)).
2. Appuyez sur **`F6`** pour ouvrir ou fermer le panneau.
3. Jouez normalement. Chaque ligne se coche automatiquement au moment où vous tuez le boss ou récupérez l'objet — vous n'avez rien à cliquer.

Ce que vous voyez :

- Les checks sont **groupés par région** (Nécrolimbe, Liurnia, …), pour voir ce qu'il reste là où vous êtes.
- Un check complété est **coché / surligné** ; un check non complété est grisé.
- Survolez une ligne pour voir un **indice d'emplacement** (où le trouver).
- Par défaut, le panneau affiche la **région où vous vous trouvez actuellement**. Pour lister toutes les régions en même temps, définissez `checks_panel_scope = "all-regions"` dans `er_overlay.toml`.

C'est tout ce dont la plupart des gens ont besoin. Le reste de cette section concerne **uniquement les joueurs randomizer**.

### Vanilla (sans mods) : rien à faire

Si vous jouez à Elden Ring normal, c'est prêt : la checklist est intégrée et fonctionne immédiatement. Laissez `regulation_path` vide dans `er_overlay.toml` et appuyez simplement sur `F6`.

### Avec l'item randomizer (thefifthmatt, [Nexus #428](https://www.nexusmods.com/eldenring/mods/428))

Le randomizer **mélange les emplacements des objets**, donc un emplacement donné au sol contient un objet différent selon la seed. Pour cocher correctement ces objets, l'overlay doit lire le **même `regulation.bin` que votre jeu charge réellement** (celui du mod, pas le fichier vanilla du jeu).

À faire une fois par configuration :

1. **Trouvez votre `regulation.bin` moddé.** C'est le fichier généré par le randomizer pour votre seed, généralement dans le dossier de mod avec lequel vous lancez le jeu, par exemple :
   - ModEngine 2 : `…\ModEngine2\mod\regulation.bin`
   - Dossier de sortie du randomizer : là où vous avez demandé au randomizer d'écrire, à côté de ses autres fichiers.

   Si vous n'êtes pas sûr, c'est le `regulation.bin` pointé par votre profil de lancement / config ModEngine — **pas** celui de votre installation Steam `Game\`.

2. **Indiquez son emplacement à l'overlay.** Ouvrez `er_overlay.toml` et définissez `regulation_path` avec ce chemin complet. Utilisez des **guillemets simples** pour ne pas avoir à doubler les antislashs :

```toml
regulation_path = 'C:\Games\ModEngine2\mod\regulation.bin'
```

3. **Enregistrez le fichier.** Sous ~2 secondes, l'overlay lit tout seul la regulation moddé (via `companion/er_checks_extractor.exe` fourni) et commence à suivre les bons objets pour votre seed.

4. **Vérifiez que ça fonctionne :** l'en-tête du panneau affiche un tag **`[seed]`** quand le mapping de seed est actif. Pas de tag = il n'a pas chargé (voir ci-dessous).

Vous ne faites cela qu'une fois. Quand vous **changez de seed**, pointez simplement `regulation_path` vers le nouveau `regulation.bin` (ou remplacez le fichier au même chemin) puis enregistrez : l'overlay le relit automatiquement. Vous n'avez jamais besoin de lancer l'assistant à la main.

**Si le tag `[seed]` n'apparaît pas :**

- Vérifiez que le chemin pointe bien vers le `regulation.bin` **moddé** et que le fichier existe (typos, mauvais dossier).
- Vérifiez que `companion/er_checks_extractor.exe` est présent à côté de `er_overlay.dll` (ne déplacez pas les fichiers hors du dossier extrait).
- Consultez `logs/er_overlay.log` : il indique si l'extractor a tourné et écrit `checks_flags.toml`.

**Bon à savoir**

- Les boss et le loot de coffres utilisent des flags fixes, donc ils se cochent de la même manière avec ou sans randomizer. Seul le **loot au sol** nécessite l'étape de seed ci-dessus.
- Si votre seed place un objet avec **aucun flag de suivi** sur un emplacement randomisé, la ligne est grisée et marquée **"Untraceable this seed"**. C'est normal, pas un bug : le jeu ne donne tout simplement rien à surveiller à l'overlay pour ce ramassage.
- Pour revenir au suivi vanilla, videz ou supprimez `regulation_path`, puis enregistrez.

## Mode challenge

Suivez un **record personnel** (le plus grand nombre de boss tués dans une run tout en respectant votre limite de morts) et le nombre de fois où la run a **échoué**, sans modifier les sauvegardes du jeu.

### Métriques

Ajoutez-les à un layout comme tuiles `kind = "metric"` (le `layouts/dashboard.toml` fourni inclut une section **`challenge`** avec les deux) :

| Métrique | Idée de label | Signification |
|--------|------------|---------|
| `pb` | PB | Plus haut nombre de boss tués enregistré pendant que la run actuelle respecte encore la limite de morts. |
| `nbtries` | TRIES | Nombre de runs échouées (augmente une fois quand les morts dépassent `max_deaths`, pas une fois par mort supplémentaire). |

Quand `[challenge].enabled = false`, les deux affichent `---`.

### Run deathless typique (`max_deaths = 0`)

| Événement | PB | TRIES |
|-------|-----|-------|
| Tuer 1 boss, aucune mort | 1 | 0 |
| Première mort (run échouée) | 1 (figé) | 1 |
| Tuer un autre boss sur la même sauvegarde | 1 | 1 |

Après une run échouée, le PB reste figé jusqu'à ce que vous commenciez une **nouvelle partie** (le flag `101` se remet à zéro avec zéro mort sur le personnage). Vous pouvez continuer à jouer sur la même sauvegarde ; l'overlay arrête simplement de compter un nouveau PB pour cette run échouée.

### Activation en jeu

1. Définissez `enabled = true` sous `[challenge]` dans `er_overlay.toml` (rechargé à chaud environ toutes les 2 s).
2. Appuyez sur **`F8`** jusqu'à ce que la section de layout **`challenge`** soit visible, ou ajoutez des tuiles `pb` / `nbtries` à votre propre layout.
3. Quittez la grotte tutoriel : le suivi de run commence quand le flag `101` devient actif.

### Notes

- Le compteur de boss utilise la même table de 207 boss que la métrique principale `bosses` (flags de kill sur toute la sauvegarde).
- Les mises à jour du challenge sont mises en pause pendant les écrans de chargement / quand le temps en jeu ne tourne pas (même idée qu'EROverlay), donc les fondus de respawn ne corrompent pas l'état de run.
- Compatible avec `boss_panel_scope` et le reste du HUD ; le challenge est indépendant du panneau de checklist des boss.

## Personnaliser l'affichage

Ce qui est affiché est entièrement piloté par le **fichier de layout** (`layout_file`), pas par le code. Un layout est une **grille** de tuiles ; chaque tuile occupe une ou plusieurs cellules.

Trois types de tuiles :

| Type | Affiche |
|------|-------|
| `metric` | Un compteur ou un temps : IGT, morts, NG+, boss tués, challenge **PB** / **TRIES**, progression de groupe, quantité d'objet. |
| `item` | Un objet suivi (icône **en couleur** si possédé, **grisée** sinon ; quantité pour les consommables). `track_equipped = true` optionnel ajoute une **bordure verte** pendant que l'objet est équipé. |
| `label` | Texte décoratif simple (titre, séparateur). |

### Sections

Un layout peut contenir plusieurs **sections** ; une seule est visible à la fois. Passez de l'une à l'autre avec `layout_section_hotkey`. Pratique pour garder une vue "minimalist" et une vue "full" sur la même touche.

Deux façons d'écrire un layout :

- **Layout simple** : une liste plate d'entrées `[[tile]]` (forme une section unique `"default"`).
- **Layout multi-section** : des blocs `[[section]]` (chacun avec un `name`) contenant des entrées `[[section.tile]]`.

La syntaxe complète se trouve dans [Format de layout](#format-de-layout-référence). Un layout invalide (tuiles superposées, dépassement de grille, section vide…) est **refusé au chargement** et signalé dans le log.

Layout fourni : `layouts/dashboard.toml` (trois sections : `minimalist`, `extended` et `challenge` avec `pb` / `nbtries`).

## Éditeur de layout

Le zip de release inclut **`layout_editor.html`** à la racine (avec `layout_editor_assets/`). Quand vous compilez depuis les sources, les mêmes fichiers se trouvent dans `tools/layout_editor/`.

Voir **[Démarrage rapide § 3](#3-personnalisez-votre-tableau-de-bord-éditeur-de-layout)** pour le workflow étape par étape. En bref : ouvrez le fichier HTML dans un navigateur, glissez les tuiles sur la grille, exportez un `.toml` dans `layouts/`, puis définissez `layout_file` dans `er_overlay.toml`.

**Développeurs :** la palette d'objets est générée depuis `goods.toml` ; après modification, lancez `python tools/goods/gen_catalog.py` (voir [Goods toolkit](tools/goods/README.md)).

## Dépannage

| Problème | Indice |
|---------|------|
| Injecteur : "process not found" | Lancez Elden Ring d'abord. |
| L'injection échoue | EAC est actif → lancez le jeu hors ligne ; essayez de lancer l'injecteur en administrateur. |
| "LoadLibraryW returned NULL" | DLL manquante / dépendance manquante / mauvaise architecture — vérifiez le chemin de la DLL. |
| Toutes les valeurs affichent `---` | Version du jeu non prise en charge — consultez `logs/er_overlay.log` pour `Unsupported game executable` ou définissez `show_debug = true`. Builds pris en charge : **2.6.2.0 (WW), 2.6.2.1 (JP)** (`eldenring` 0.14). |
| Le jeu crash à l'injection | Consultez `logs/er_overlay.log` : la dernière ligne avant le crash indique l'étape (`Hudhook::apply`, `build_view_model`, etc.). Mettez le jeu à jour si le log indique un exécutable non pris en charge. |
| Pas d'icônes (seulement des points) | PNG manquants dans `assets/icons` — voir [Icônes](#icônes). |
| Crash de l'overlay | Conflit avec un autre hook DX12 (RTSS, etc.). |
| Les métriques challenge restent toujours à `---` | Définissez `[challenge] enabled = true` dans `er_overlay.toml`. |
| PB / essais semblent faux après des tests | Supprimez `challenge_state.toml` à côté de la DLL et réessayez sur une run propre. |
| Loot au sol randomisé non suivi | Définissez `regulation_path` vers le `regulation.bin` chargé par le jeu ; vérifiez `logs/er_overlay.log` pour le résultat de l'extractor et que `checks_flags.toml` a été écrit. |
| L'en-tête des checks n'a pas de tag `[seed]` | Aucun mapping de seed actif — `regulation_path` est absent/incorrect, ou `er_checks_extractor.exe` manque à côté de la DLL. |

### Logs et diagnostics

Toute la sortie runtime va dans **`logs/`** à côté de `er_overlay.dll` :

| Fichier | Contenu |
|------|----------|
| `er_overlay.log` | Init DLL, détection de version du jeu, Hudhook, résolution de pointeurs, erreurs |
| `er_injector.log` | Recherche du processus, avertissement EAC, résultat d'injection |

Activez **`show_debug = true`** dans `er_overlay.toml` pour une fenêtre en jeu (backend, version de l'exe du jeu, pointeurs résolus).

Logs verbeux : définissez la variable d'environnement `RUST_LOG=debug` avant de lancer l'injecteur.

Les builds du jeu pris en charge sont loggés au démarrage (`Game executable supported` vs `Unsupported game executable`).

---

# Référence technique

## Architecture

Workspace Cargo de 5 crates :

| Crate | Rôle |
|-------|------|
| `er_overlay_common` | Config TOML, format de layout, raccourcis clavier, logs, types partagés. |
| `er_game_state` | Lectures du jeu via **fromsoftware-rs** (`GameDataMan`, `CSEventFlagMan`, `WorldChrMan`) + tables de données. Trait `GameStateSource` (impl live + mock testable). |
| `er_overlay_ui` | View model + rendu ImGui (tuiles, icônes, texte). |
| `er_overlay_dll` | DLL injectée, hook DX12 via [hudhook](https://github.com/veeenu/hudhook). |
| `er_overlay_injector` | Injecteur `LoadLibraryW` documenté. |

Boucle : `er_overlay_dll` interroge `er_game_state` (throttlé à ~250 ms), construit un `OverlayViewModel`, puis `er_overlay_ui` l'affiche selon le layout actif.

## Format de layout (référence)

```toml
[grid]
columns = 8          # largeur maximale de placement (validation)
unit_size = 64       # côté d'une cellule carrée, en px
gap = 4              # espacement entre cellules
border_radius = 6
window_padding = 8

[style]
border_default  = [100, 100, 110, 200]  # RGBA
border_complete = [60, 200, 90, 255]     # bordure quand une métrique est "complete"
tile_bg         = [12, 12, 18, 180]
label_scale = 0.65   # taille du label relative au texte
value_scale = 1.15   # taille de la valeur relative au texte

default_section = "minimalist"   # optionnel
```

Puis soit une liste plate de tuiles :

```toml
[[tile]]
kind = "metric"
metric = "igt"
col = 0
row = 0
w = 2       # alias de col_span
h = 1       # alias de row_span
label = "IGT"
```

…soit des sections :

```toml
[[section]]
name = "minimalist"

[[section.tile]]
kind = "label"
col = 0
row = 0
w = 2
h = 1
label = "RUN"
```

**Champs par type de tuile** (tous : `col`, `row`, `w`/`col_span`, `h`/`row_span`, `id` optionnel) :

- `metric` : `metric` (id de métrique), `label`, `show_max` (bool, affiche `N/total`), `icon` (clé PNG optionnelle affichée au-dessus du texte).
- `item` : `key` (clé de good depuis `goods.toml`). Icône colorée si possédé, grisée sinon, quantité pour les consommables. `track_equipped = true` optionnel ajoute une bordure verte pendant que l'objet est équipé (talismans, grandes runes, consommables en raccourci).
- `label` : `label` (texte).

**Règles de validation** : `columns > 0`, spans `> 0`, aucune tuile superposée *dans une même section*, `col + col_span ≤ columns`, noms de sections uniques et non vides, sections non vides. Le fichier est revalidé à chaque rechargement (toutes les 2 s).

## Métriques disponibles

Le champ `metric` d'une tuile `metric` accepte :

| Métrique | Signification |
|--------|---------|
| `igt` | Temps en jeu (`HH:MM:SS`). |
| `deaths` | Nombre de morts. |
| `ng_cycle` | Cycle New Game (`NG+N`). |
| `bosses` | Boss tués sur 207. |
| `pb` | Record personnel du challenge (nécessite `[challenge] enabled = true`). |
| `nbtries` | Nombre de runs challenge échouées (`tries` dans EROverlay ; mêmes alias : `tries`, `challenge_pb`, `challenge_tries`). |
| `scadutree_blessing` | Niveau de Bénédiction de l'Arbre-Occulte dépensé aux Sites de grâce (`N/20`). Distinct de la clé de good `scadutree` (nombre de fragments en inventaire). |
| *nom de groupe* | Progression `owned/total` d'un groupe agrégé depuis `goods.toml` (par exemple `great_runes`). |
| *clé de good* | Quantité (consommable `count = true`) ou état possédé `0/1` pour un objet unique. |

Toute clé inconnue affiche `---` (indisponible).

## Données du jeu (tables)

### Boss — `tables/<lang>/bosses.toml`

Une table complète de boss par langue (`tables/en/bosses.toml`, `tables/fr/bosses.toml`, …) : 207 entrées (165 base + 42 Shadow of the Erdtree), régions, ordre d'affichage, flags, icônes. Copiée à côté de la DLL au build. **Rechargée à chaud** quand le fichier change (même poll de 2 s que `er_overlay.toml`) ; si le fichier de locale est absent, fallback sur `tables/en/bosses.toml` (embarqué dans la DLL). Définissez `boss_locale = "auto"` pour correspondre à la langue du jeu, ou forcez `fr`. Régénérez une locale avec `python tools/gen_boss_locale_toml.py fr` (depuis `en/bosses.toml` + JSON de ER_boss_checklist_R).

### Checks — `tables/<lang>/checks.toml`

La checklist derrière le [panneau des checks](#panneau-des-checks-compatible-randomizer). Une entrée `[[check]]` par ligne ; chacune déclare si elle est `dynamic` (loot au sol sensible au randomizer) ou non. Embarquée dans la DLL (`en`) et copiée à côté au build ; rechargée à chaud comme la table des boss.

| Champ | Requis | Description |
|-------|:--------:|-------------|
| `region` | oui | Région à laquelle le check appartient (groupe le panneau). |
| `name` | oui | Nom affiché (boss ou objet). |
| `place` | — | Indice d'emplacement (affiché en tooltip). |
| `dlc` | — | `true` pour taguer l'entrée `[DLC]`. |
| `dynamic` | oui | `false` = `flag` fixe. `true` = loot au sol sensible au randomizer, résolu par seed. |
| `flag` | pour static | Event flag vérifié quand `dynamic = false`. |
| `vanilla_flag` | pour dynamic | Flag d'acquisition vanilla ; utilisé comme fallback quand aucun mapping de seed n'est chargé. |
| `lot_id` | pour dynamic | Id de ligne `ItemLotParam_map` stable utilisé pour chercher le flag actuel dans une regulation randomizer. |

Quand `regulation_path` est défini, le companion écrit un `checks_flags.toml` (`lot_id → flag actuel` + hash de regulation) que l'overlay recharge à chaud pour résoudre les checks dynamiques de la seed active.

### Goods — `crates/er_game_state/tables/goods.toml`

Une ligne `[[good]]` par objet suivi. Champs :

| Champ | Requis | Description |
|-------|:--------:|-------------|
| `key` | oui | Id unique (et nom PNG par défaut `{key}.png`). |
| `item_id` | oui | `param_id` de l'objet (`EquipParamGoods` ou `EquipParamAccessory`). |
| `name` | — | Nom affiché. |
| `category` | — | `goods` (défaut) ou `accessory` (talismans). Évite les collisions de `param_id` entre catégories. |
| `count` | — | `true` = consommable empilable → affiche la quantité en inventaire. |
| `max` | — | Limite d'affichage pour un compteur (par exemple scadutree → `N/50`). |
| `pickup_flag` | — | Event flag de possession (fallback quand l'objet n'est plus dans l'inventaire). |
| `file` | — | Nom PNG personnalisé. |
| `icon_id` | — | Utilisé uniquement par les scripts de récupération d'icônes. |

**Groupes agrégés** : déclarés via une table `[groups.<name>]` listant les `members` (clés de good). L'overlay expose ensuite une métrique `<name>` = nombre de membres possédés / total. Exemple :

```toml
[groups.great_runes]
members = ["godrick_rune", "radahn_rune", "morgott_rune", "rykard_rune", "mohg_rune", "malenia_rune"]
```

Les talismans (category `accessory`) vivent dans un bloc délimité (`# --- talismans ---` … `# --- end talismans ---`).

**Ajouter un nouveau good** : voir **[`tools/goods/README.md`](tools/goods/README.md)**.

### Ajouter un good

Checklist complète : **[`tools/goods/README.md`](tools/goods/README.md)**.

```powershell
# après modification de goods.toml
python tools/goods/fetch_goods_icons.py --out assets/icons
python tools/goods/gen_catalog.py
cargo test -p er_game_state
```

## Icônes

Les tuiles peuvent afficher de vraies icônes du jeu (PNG) au lieu de points colorés.

Placez les fichiers PNG dans `assets/icons/`, un par good, nommé selon sa `key` (par exemple `godrick_rune.png`) ou selon le champ `file` du good. Gardez `use_item_icons = true` (défaut) dans `er_overlay.toml`. Toute icône manquante retombe sur un point coloré.

Les PNG sont **ignorés par git** (`assets/icons/*.png`). Au déploiement, copiez `assets/icons/` à côté de `er_overlay.dll`.

Générez les PNG manquants avec `python tools/goods/fetch_goods_icons.py --out assets/icons` (voir [`tools/goods/README.md`](tools/goods/README.md)).

## Développement

```powershell
cargo test --workspace      # tests
cargo clippy --workspace    # lints
cargo fmt --all             # formatting
```

La CI (`.github/workflows/ci.yml`) lance `fmt --check`, `clippy -D warnings` et `test` à chaque push/PR.

`er_game_state` expose une feature `mock` (`MockGameState`) pour tester l'UI sans le jeu.

## Références

- [EROverlay](https://github.com/soarqin/EROverlay) — overlay de boss ; référence des sémantiques du mode challenge
- [hudhook](https://github.com/veeenu/hudhook) — hook DX12 + ImGui
- [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) — accès aux structures du jeu
- [SoulSplitter](https://github.com/FrankvdStam/SoulSplitter) — référence flags / IGT
- [SmithBox](https://github.com/vawser/Smithbox) - icônes / flags

## Licence

**GNU Affero General Public License v3.0 (AGPL-3.0-only)** — voir [`LICENSE`](LICENSE).

C'est une licence à **copyleft fort**. En bref : toute personne qui distribue ce logiciel, une version modifiée ou une oeuvre dérivée — **y compris en le rendant simplement disponible sur un réseau** — doit publier le code source correspondant complet sous la même licence AGPL-3.0. Autrement dit : si vous réutilisez ce code, votre projet doit rester open source.
