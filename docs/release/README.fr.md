# Elden Ring Overlay (hors ligne, lecture seule)

Overlay en lecture seule pour un `eldenring.exe` **déjà lancé**. Il affiche un tableau de bord personnalisable — IGT, compteur de **boss**, Grandes Runes, morts, NG+, objets clés — ainsi que des **checklists boss/loot** avec prise en charge de l'item randomizer.

> **Lecture seule, hors ligne, pas de triche.** Aucune écriture mémoire, aucun contournement anti-cheat. Utilisation uniquement en solo hors ligne.

> **Ce fichier est le guide utilisateur** fourni avec la release. La documentation complète, le code source et les notes développeur sont sur GitHub : **<https://github.com/Sully-/ER_DynamicOverlay>**.

> 🇬🇧 English version: [README.md](README.md).

---

## Sommaire

- [Démarrage rapide](#démarrage-rapide)
- [Avertissements](#avertissements)
- [Configuration (`er_overlay.toml`)](#configuration-er_overlaytoml)
- [Le tableau de bord : tuiles, modes de suivi et métriques](#le-tableau-de-bord--tuiles-modes-de-suivi-et-métriques)
- [Panneau des checks (compatible randomizer)](#panneau-des-checks-compatible-randomizer)
- [Mode challenge](#mode-challenge)
- [Éditeur de layout](#éditeur-de-layout)
- [Dépannage](#dépannage)
- [Licence](#licence)

---

## Démarrage rapide

### 1. Lancez Elden Ring hors ligne

L'overlay **ne fonctionne pas avec EasyAntiCheat activé**. Lancez le jeu en mode hors ligne, par exemple :

- Lancez directement `eldenring.exe` (pas via le launcher EAC), avec un fichier `steam_appid.txt` contenant `1245620` à côté de l'exe, **ou**
- Utilisez votre méthode habituelle hors ligne / sans EAC.

Prérequis : Windows **x64**, et un build d'Elden Ring pris en charge par cette release (actuellement **2.6.2.0 (WW)** et **2.6.2.1 (JP)** — voir [Dépannage](#dépannage) si les valeurs affichent `---`).

Gardez le jeu lancé sur l'écran titre ou dans une sauvegarde : l'injecteur s'attache à un processus déjà en cours.

### 2. Lancez l'overlay

Gardez tous les fichiers du dossier extrait ensemble — **ne les séparez pas :**

| Fichier / dossier | Rôle |
|---------------|------|
| `er_overlay_injector.exe` | Lanceur — **double-cliquez dessus** |
| `er_overlay.dll` | Overlay (injecté dans le jeu) |
| `er_overlay.toml` | Paramètres (position, échelle, raccourcis clavier, fichier de layout…) |
| `layouts/` | Fichiers de layout du tableau de bord |
| `tables/` | Listes de boss / checks par langue |
| `assets/` | Icônes d'objets |
| `companion/er_checks_extractor.exe` | Assistant qui lit un `regulation.bin` de randomizer (voir [Panneau des checks](#panneau-des-checks-compatible-randomizer)) |
| `layout_editor.html` | Éditeur visuel de layout (voir [Éditeur de layout](#éditeur-de-layout)) |
| `challenge_state.toml` | *(runtime)* PB / essais du challenge — créé quand `[challenge] enabled = true` |
| `checks_flags.toml` | *(runtime)* Flags randomizer par seed — créé uniquement quand `regulation_path` est défini |

Avec Elden Ring déjà lancé hors ligne, **double-cliquez sur `er_overlay_injector.exe`**. L'overlay apparaît en jeu (par défaut : HUD minimal en haut à droite). Relancez l'injecteur après chaque redémarrage du jeu — il ne persiste pas entre deux lancements.

**Raccourcis par défaut** (définis dans `er_overlay.toml`, rechargés à chaud toutes les 2 s) :

| Touche | Action |
|-----|--------|
| `F8` | Changer de section de layout (`minimalist` → `extended` → `challenge`, …) |
| `F7` | Afficher / masquer le panneau de checklist des boss |
| `F6` | Afficher / masquer le panneau des checks (boss + checklist de loot, compatible randomizer) |
| `F9` | Afficher / masquer tout l'overlay |

Le **panneau des boss**, le **panneau des checks** et la section de layout **extended** sont mutuellement exclusifs : en ouvrir un ferme les autres.

Si quelque chose se passe mal, consultez `logs/er_injector.log` et `logs/er_overlay.log` dans le même dossier.

### Mises à jour automatiques

Au lancement de `er_overlay_injector.exe`, celui-ci vérifie la dernière release sur GitHub. Si une version plus récente existe, il demande dans la console :

```
A new version of ER Overlay is available: v1.3.0 (installed: v1.2.0).
Download and install it now? [Y/n]
```

Appuyez sur Entrée (ou `y`) pour mettre à jour : il télécharge la release, remplace les fichiers du programme, puis se relance et injecte la nouvelle version. Répondez `n` pour garder votre version actuelle pour ce lancement.

Vos réglages ne sont **jamais écrasés** : `er_overlay.toml` et `layouts/dashboard.toml` sont conservés tels quels. Les versions de la release sont déposées à côté sous `er_overlay.toml.new` / `dashboard.toml.new` comme référence, et les nouvelles options éventuelles sont listées dans la console pour que vous puissiez recopier celles qui vous intéressent.

La vérification nécessite une connexion Internet mais ne bloque jamais l'overlay : hors ligne (ou en cas de refus), l'injection se poursuit normalement. Pour désactiver totalement la vérification, lancez avec `--skip-update`.

### 3. Personnalisez votre tableau de bord

Tout ce qui est affiché est piloté par un **fichier de layout** — une grille de tuiles. Modifiez-le visuellement avec **`layout_editor.html`** fourni (pas de TOML à apprendre), puis pointez `layout_file` vers votre fichier dans `er_overlay.toml`. Voir [Éditeur de layout](#éditeur-de-layout) pour le workflow et [Le tableau de bord](#le-tableau-de-bord--tuiles-modes-de-suivi-et-métriques) pour le rôle de chaque tuile.

### 4. Ajustez l'apparence et le comportement

Ouvrez `er_overlay.toml` dans n'importe quel éditeur de texte (rechargé à chaud ~toutes les 2 s). Les options les plus courantes sont `anchor` / `offset_x` / `offset_y` (position), `scale` / `text_size` / `icon_size` (taille) et les bascules de panneaux. Voir [Configuration](#configuration-er_overlaytoml) pour la référence complète.

### Injecteur avancé (ligne de commande)

Pour des cas spécifiques, vous pouvez lancer l'injecteur depuis un terminal avec des options :

```powershell
# cibler un id de processus spécifique
.\er_overlay_injector.exe --pid 12345
# chemin explicite vers la DLL
.\er_overlay_injector.exe --dll ".\er_overlay.dll"
# tout valider sans injecter
.\er_overlay_injector.exe --dry-run
# ignorer la vérification de mise à jour GitHub
.\er_overlay_injector.exe --skip-update
```

## Avertissements

- **Hors ligne uniquement** — aucun support multijoueur / en ligne.
- **Ne contourne pas EAC** — lancez le jeu sans EasyAntiCheat (par exemple en lançant directement `eldenring.exe` avec `steam_appid.txt`).
- **Lecture seule** — aucune écriture mémoire, ce n'est pas un trainer.
- **Injection transparente et documentée** (`LoadLibraryW` via `CreateRemoteThread`), sans furtivité.

## Configuration (`er_overlay.toml`)

Lu à côté de la DLL et **rechargé à chaud toutes les 2 secondes** — vous pouvez le modifier pendant que le jeu tourne. Les valeurs hors limites sont ramenées à leur valeur par défaut avec un avertissement dans le log.

### Apparence & position

| Option | Type | Défaut | Description |
|--------|------|---------|-------------|
| `layout_file` | chemin | `layouts/dashboard.toml` | Fichier de layout à afficher (voir [Le tableau de bord](#le-tableau-de-bord--tuiles-modes-de-suivi-et-métriques)). |
| `default_layout_section` | string | — | Section affichée au démarrage (remplace le `default_section` du layout). |
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

### Raccourcis clavier

| Option | Type | Défaut | Description |
|--------|------|---------|-------------|
| `layout_section_hotkey` | string | `F8` | Parcourir les sections de layout, par exemple `"F8"`, `"Ctrl+Shift+F1"`. |
| `boss_panel_hotkey` | string | `F7` | Afficher / masquer le panneau de checklist des boss. |
| `checks_panel_hotkey` | string | `F6` | Afficher / masquer le panneau des checks (boss + checklist de loot). |
| `hide_all_hotkey` | string | `F9` | Afficher / masquer tout l'overlay. |

### Panneaux boss & checks

| Option | Type | Défaut | Description |
|--------|------|---------|-------------|
| `boss_panel_visible` | bool | `true`* | Afficher le panneau des boss au démarrage. Un seul panneau boss / checks peut être affiché au démarrage ; boss gagne si les deux valent `true`. |
| `boss_panel_scope` | enum | `current-region` | `current-region` ou `all-regions`. |
| `boss_panel_layout` | string | — | Panneau `x,y,width,height` (pixels ou `%`). Omettre ou `auto` = `"-5, 10, 25%, 92%"` (aligné à droite), déplacé sous le HUD minimal. x/y négatifs = décalage depuis le bord droit/bas. |
| `boss_locale` | string | `auto` | Langue de la table des boss (`en`, `fr`, …). `auto` lit la langue du jeu via Steam ; fallback sur `en`. |
| `checks_panel_visible` | bool | `false` | Afficher le panneau des checks au démarrage. Mutuellement exclusif avec le panneau des boss (boss gagne si les deux valent `true`). |
| `checks_panel_scope` | enum | `current-region`* | `current-region` ou `all-regions`. |
| `checks_panel_layout` | string | — | Panneau `x,y,width,height` (pixels ou `%`). Omettre ou `auto` = `"5, 10, 25%, 92%"` (aligné à gauche, miroir du panneau des boss). |
| `regulation_path` | chemin | — | Chemin vers le `regulation.bin` que le jeu **charge** (votre randomizer / mod ModEngine). Active la résolution par seed des flags de loot randomisé. Vide/omis = flags vanilla. Voir [Panneau des checks](#panneau-des-checks-compatible-randomizer). |
| `checks_extractor_path` | chemin | — | Remplace l'emplacement de l'exe assistant. Omettre pour auto-détecter `companion/er_checks_extractor.exe` (puis `er_checks_extractor.exe`) à côté de la DLL. |

\* Le `er_overlay.toml` fourni met `boss_panel_visible = false` et `checks_panel_scope = all-regions`.

Le ruleset optionnel du challenge se configure sous `[challenge]` — voir [Mode challenge](#mode-challenge).

## Le tableau de bord : tuiles, modes de suivi et métriques

Tout ce qui est à l'écran est piloté par le **fichier de layout** (`layout_file`). Un layout est une **grille** de tuiles ; chaque tuile occupe une ou plusieurs cellules. Le plus simple pour le modifier est l'[Éditeur de layout](#éditeur-de-layout) visuel.

### Types de tuiles

| Type | Affiche |
|------|-------|
| `metric` | Un compteur ou un temps : IGT, morts, NG+, boss tués, challenge **PB** / **TRIES**, progression de groupe, quantité d'objet. Voir [Métriques disponibles](#métriques-disponibles). |
| `item` | Un objet suivi unique, avec un ou plusieurs **modes de suivi** (ci-dessous). |
| `label` | Texte décoratif simple (titre, séparateur). |

### Modes de suivi d'un objet

Une tuile `item` peut suivre jusqu'à trois aspects **indépendants** d'un objet. Vous pouvez les combiner sur la même tuile (par exemple un talisman qui s'illumine quand il est équipé *et* reste allumé une fois acquis).

| Mode | Activé avec | Effet |
|------|-------------|--------------|
| **Possédé** (par défaut) | *(toujours actif)* | Icône **en couleur** quand l'objet est actuellement dans votre inventaire (ou que son pickup flag est actif), **grisée** sinon. Les consommables affichent leur quantité à la place. |
| **Équipé** | `track_equipped = true` | Ajoute une **bordure verte** pendant que l'objet est **actuellement équipé** — talismans, Grandes Runes, consommables en raccourci, sacoche. Idéal pour voir votre build/équipement actif d'un coup d'œil. |
| **Historique** | `historic = true` | Garde l'objet marqué comme possédé **même après que vous ne le détenez plus** (consommé, vendu, jeté). Au lieu de lire seulement l'inventaire courant, il vérifie aussi le **flag d'acquisition** de l'objet, donc « l'ai-je déjà ramassé ? » reste vrai. **Compatible randomizer :** il résout le flag propre à la seed quand `regulation_path` est défini. |

**Pourquoi c'est utile**

- **Équipé** répond à *« ce talisman/cette rune est-il équipé en ce moment ? »* — parfait pour un HUD de build/équipement.
- **Historique** répond à *« ai-je obtenu cet objet au moins une fois dans cette run ? »* — essentiel pour les objets uniques ou consommables (par exemple scarseals/soreseals, charmes scorpion) que vous pourriez retirer, afin que la tuile ne s'éteigne pas dès que l'objet quitte votre inventaire.

Dans l'éditeur de layout, les deux sont de simples cases à cocher sur une tuile d'objet. En TOML, cela ressemble à ceci :

```toml
[[section.tile]]
kind = "item"
key = "fire_scorpion_charm"
track_equipped = true   # bordure verte quand porté
historic = true         # reste allumé une fois obtenu
col = 0
row = 0
```

### Métriques disponibles

Le champ `metric` d'une tuile `metric` accepte :

| Métrique | Signification |
|--------|---------|
| `igt` | Temps en jeu (`HH:MM:SS`). |
| `deaths` | Nombre de morts. |
| `ng_cycle` | Cycle New Game (`NG+N`). |
| `bosses` | Boss tués sur 207. |
| `pb` | Record personnel du challenge (nécessite `[challenge] enabled = true`). |
| `nbtries` | Nombre de runs challenge échouées (alias : `tries`, `challenge_pb`, `challenge_tries`). |
| `scadutree_blessing` | Niveau de Bénédiction de l'Arbre-Occulte dépensé aux Sites de grâce (`N/20`). |
| *nom de groupe* | Progression `owned/total` d'un groupe agrégé (par exemple `great_runes`). |
| *clé d'objet* | Quantité (pour un consommable) ou état possédé `0/1` pour un objet unique. |

Toute clé inconnue affiche `---` (indisponible).

### Sections

Un layout peut contenir plusieurs **sections** ; une seule est visible à la fois. Passez de l'une à l'autre avec `layout_section_hotkey` (`F8` par défaut) — pratique pour garder une vue « minimalist » et une vue « full » sur la même touche. Le `layouts/dashboard.toml` fourni contient trois sections : `minimalist`, `extended` et `challenge`.

## Panneau des checks (compatible randomizer)

Le **panneau des checks** est une checklist unique de tout ce qui vaut la peine d'être complété dans une run. Un *check* correspond à une action : un **boss à tuer** ou un **objet important à récupérer**. Voyez-le comme le panneau des boss, mais avec les items clés en plus — et il peut suivre l'**item randomizer**.

### Utilisation de base

1. Lancez Elden Ring et lancez l'overlay (voir [Démarrage rapide](#démarrage-rapide)).
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
- Si votre seed place un objet avec **aucun flag de suivi** sur un emplacement randomisé, la ligne est grisée et marquée **« Untraceable this seed »**. C'est normal, pas un bug.
- Pour revenir au suivi vanilla, videz ou supprimez `regulation_path`, puis enregistrez.

## Mode challenge

Suivez un **record personnel** (le plus grand nombre de boss tués dans une run tout en respectant votre limite de morts) et le nombre de fois où la run a **échoué**, sans modifier les sauvegardes du jeu. **Désactivé par défaut.**

### Configuration (`[challenge]`)

| Option | Type | Défaut | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Quand `false`, les métriques challenge affichent `---` et aucune progression n'est suivie. |
| `max_deaths` | u32 | `0` | Morts autorisées **par run** (inclusif). La run échoue quand les morts de la run dépassent cette valeur. `0` = deathless. |
| `start_flag` | u32 | `101` | Event flag qui marque le **début d'une run** (flag `101` = sortie de la Grotte de la connaissance / Cimetière abandonné). |

```toml
[challenge]
enabled = true
max_deaths = 0      # deathless : une mort termine la run
start_flag = 101
```

### Métriques

Ajoutez-les comme tuiles `kind = "metric"` (le `layouts/dashboard.toml` fourni inclut une section **`challenge`** avec les deux) :

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

**Notes**

- **Fichier de progression :** `challenge_state.toml` (à côté de `er_overlay.dll`, créé à l'exécution) stocke le record personnel, le nombre de runs échouées et l'état interne de la run. Supprimez-le pour réinitialiser le PB et les essais.
- Les mises à jour du challenge sont mises en pause pendant les écrans de chargement / quand le temps en jeu ne tourne pas, donc les fondus de respawn ne corrompent pas l'état de run.

## Éditeur de layout

Le zip inclut un **éditeur visuel** — pas besoin d'apprendre la syntaxe TOML — sous forme de **`layout_editor.html`** à la racine (avec `layout_editor_assets/`).

1. Ouvrez **`layout_editor.html`** depuis le dossier extrait dans votre navigateur (Chrome, Edge, Firefox…).
   - Si l'import/export est bloqué, servez plutôt le dossier : ouvrez un terminal dans le dossier et lancez `python -m http.server`, puis allez sur `http://localhost:8000/layout_editor.html`.
2. **Glissez-déposez** des métriques, labels et objets depuis la palette de gauche vers la grille.
3. Ajustez la grille (colonnes, lignes, taille de cellule, espacement) et chaque tuile dans le panneau de droite — y compris les bascules `track_equipped` et `historic` pour les tuiles d'objet (voir [Modes de suivi d'un objet](#modes-de-suivi-dun-objet)).
4. Utilisez **Import layout file** pour modifier le fichier fourni `layouts/dashboard.toml`, ou partez de **New**.
5. Cliquez sur **Export layout file** et enregistrez le `.toml` dans le dossier `layouts/` (par exemple `layouts/my_run.toml`).
6. Modifiez `er_overlay.toml` et définissez `layout_file = "layouts/my_run.toml"`. L'overlay recharge automatiquement le fichier sous ~2 secondes (même pendant que le jeu tourne).

**Astuce :** créez plusieurs **sections** dans un même fichier (par exemple une vue compacte et une vue complète) et passez de l'une à l'autre avec `F8`.

## Dépannage

| Problème | Indice |
|---------|------|
| Injecteur : "process not found" | Lancez Elden Ring d'abord. |
| L'injection échoue | EAC est actif → lancez le jeu hors ligne ; essayez de lancer l'injecteur en administrateur. |
| "LoadLibraryW returned NULL" | DLL manquante / dépendance manquante / mauvaise architecture — vérifiez le chemin de la DLL. |
| Toutes les valeurs affichent `---` | Version du jeu non prise en charge — consultez `logs/er_overlay.log` pour `Unsupported game executable` ou définissez `show_debug = true`. Builds pris en charge : **2.6.2.0 (WW), 2.6.2.1 (JP)**. |
| Le jeu crash à l'injection | Consultez `logs/er_overlay.log` : la dernière ligne avant le crash indique l'étape. Mettez le jeu à jour si le log indique un exécutable non pris en charge. |
| Pas d'icônes (seulement des points) | PNG manquants dans `assets/icons` — gardez le dossier à côté de `er_overlay.dll`. |
| Crash de l'overlay | Conflit avec un autre hook DX12 (RTSS, etc.). |
| Une tuile d'objet ne s'allume jamais | Mauvaise `key`, ou l'objet quitte votre inventaire — ajoutez `historic = true` pour qu'elle reste allumée après acquisition (voir [Modes de suivi d'un objet](#modes-de-suivi-dun-objet)). |
| La surbrillance « équipé » n'apparaît jamais | `track_equipped = true` ne s'allume que pendant que l'objet est réellement équipé (talismans, runes, raccourcis, sacoche). |
| Les métriques challenge restent toujours à `---` | Définissez `[challenge] enabled = true` dans `er_overlay.toml`. |
| PB / essais semblent faux après des tests | Supprimez `challenge_state.toml` à côté de la DLL et réessayez sur une run propre. |
| Loot au sol randomisé non suivi | Définissez `regulation_path` vers le `regulation.bin` chargé par le jeu ; vérifiez `logs/er_overlay.log` et que `checks_flags.toml` a été écrit. |
| L'en-tête des checks n'a pas de tag `[seed]` | Aucun mapping de seed actif — `regulation_path` est absent/incorrect, ou `er_checks_extractor.exe` manque à côté de la DLL. |

### Logs et diagnostics

Toute la sortie runtime va dans **`logs/`** à côté de `er_overlay.dll` :

| Fichier | Contenu |
|------|----------|
| `er_overlay.log` | Init DLL, détection de version du jeu, hook, résolution de pointeurs, erreurs |
| `er_injector.log` | Recherche du processus, avertissement EAC, résultat d'injection |

Activez **`show_debug = true`** dans `er_overlay.toml` pour une fenêtre de diagnostic en jeu. Pour des logs verbeux, définissez la variable d'environnement `RUST_LOG=debug` avant de lancer l'injecteur.

## Licence

**GNU Affero General Public License v3.0 (AGPL-3.0-only)** — voir [`LICENSE`](LICENSE).

C'est une licence à **copyleft fort** : toute personne qui distribue ce logiciel, une version modifiée ou une œuvre dérivée — **y compris en le rendant simplement disponible sur un réseau** — doit publier le code source correspondant complet sous la même licence AGPL-3.0.
