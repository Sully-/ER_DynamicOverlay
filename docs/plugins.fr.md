# Plugins de métriques

Un plugin de métriques est une DLL native Windows x64. On la dépose dans `plugins/` à côté de `er_overlay.dll` et l'overlay l'appelle. Il déclare des ids de métriques et renvoie un nombre (ou une courte chaîne) à chaque tick. Le fichier de layout décide du label, de l'icône, de `show_max` et de la position de la tuile. L'overlay ne fournit aucune donnée de jeu au plugin : le plugin lit ce dont il a besoin, comme il veut.

> **Un plugin est du code arbitraire dans le processus d'Elden Ring.** Un plugin bogué fait planter le jeu. Il n'y a pas de bac à sable. Ne chargez que des plugins auxquels vous faites confiance.

## Un plugin qui renvoie 42

C'est l'intégralité du plugin. Il expose une métrique, `answer.value`, qui vaut toujours `42`.

```rust
use er_overlay_plugin_sdk::{declare_plugin, MetricPlugin, MetricSample};

struct Answer;

impl MetricPlugin for Answer {
    fn metrics() -> &'static [&'static str] {
        &["answer.value"]
    }

    fn sample(&mut self, _index: usize) -> MetricSample {
        MetricSample::count(42)
    }
}

declare_plugin!(Answer, Answer);
```

Les mêmes sources sont dans [`examples/plugins/answer`](../examples/plugins/answer). Copiez ce dossier si vous ne voulez pas partir d'un crate vide.

### 1. Créer le crate

Depuis le dossier de votre choix :

```bash
cargo new --lib answer
```

`Cargo.toml` :

```toml
[package]
name = "answer"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
er_overlay_plugin_sdk = { path = "chemin/vers/ER_DynamicOverlay/sdk/er_overlay_plugin_sdk" }

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

`panic = "abort"` est obligatoire. Une panic ne doit pas se dérouler à travers la frontière C ; de toute façon ça avorterait, autant l'écrire. `crate-type = ["cdylib"]` produit une DLL plutôt qu'une bibliothèque Rust.

Le source ci-dessus va dans `src/lib.rs`.

### 2. Compiler

Sur Windows 64 bits la cible par défaut est déjà la bonne :

```bash
cargo build --release
```

La DLL est `target/release/answer.dll`. La forme explicite, utile en cross-compilation :

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

Elle écrit `target/x86_64-pc-windows-msvc/release/answer.dll`.

### 3. L'installer

Copiez `answer.dll` dans le dossier `plugins/` à côté de `er_overlay.dll`. Créez le dossier s'il n'existe pas. Redémarrez le jeu : la liste des métriques est figée au démarrage.

Dans `er_overlay.toml` :

```toml
[plugins]
enabled = true
dir = "plugins"
disabled = []   # par ex. ["answer"] pour ignorer answer.dll sans la supprimer
```

Avec les logs activés (`log_enabled = true`), `logs/er_overlay.log` indique chaque plugin chargé et le nombre de métriques enregistrées. Une DLL rejetée (mauvaise version d'ABI, export manquant, `create` a renvoyé null) est journalisée puis ignorée ; le reste de l'overlay démarre quand même.

### 4. L'afficher

Le plugin ne choisit ni le nom ni l'icône. C'est la tuile du layout, exactement comme pour `igt`. Ajoutez ceci à votre layout :

```toml
[[tile]]
kind = "metric"
metric = "answer.value"
label = "ANSWER"
col = 0
row = 0
```

Le layout est rechargé à chaud, sans redémarrer. La tuile affiche `42`. Si le plugin est absent, la tuile affiche `---` et rien d'autre ne casse.

Une icône est une clé PNG, cherchée d'abord dans `assets/icons` puis dans `plugins/icons` :

```toml
icon = "answer"
show_max = false
```

## Ce que l'overlay appelle

Six fonctions C, dans cet ordre, toutes sur le thread de poll de l'overlay (environ toutes les 250 ms), jamais sur le thread de rendu :

| Export | Moment |
| --- | --- |
| `er_overlay_plugin_abi_version` | Une fois, avant tout le reste. Doit renvoyer `1`. |
| `er_overlay_plugin_create` | Une fois. Renvoyer null refuse le chargement. |
| `er_overlay_plugin_metrics` | Une fois, juste après create. La liste d'ids est ensuite figée. |
| `er_overlay_plugin_poll` | À chaque tick, avant les lectures. |
| `er_overlay_plugin_sample` | Une fois par métrique et par tick. L'argument est un index dans cette liste, pas une chaîne. |
| `er_overlay_plugin_destroy` | Au déchargement de l'overlay. |

`poll` est un battement de cœur, pas une garantie d'échantillonnage. Pour observer quelque chose de plus court que 250 ms, lancez votre propre thread et faites lire à `sample` la dernière valeur publiée.

L'état appartient au plugin. Compteurs, flags mémorisés, fichiers sous `base_dir` (le dossier de `er_overlay.dll`) : l'overlay ne les sauvegarde ni ne les réinitialise. Un plugin dont la liste de métriques dépend d'un fichier de config doit être rechargé — redémarrage du jeu — pour en prendre une nouvelle, ce qui colle au fait que le layout doit de toute façon être édité.

## Types de valeur

Le descripteur ne porte qu'un id et un type. Le type dit à l'overlay comment afficher le nombre :

| Type | Sens |
| --- | --- |
| `count` (`0`) | Un entier. Avec `has_max` sur l'échantillon et `show_max` sur la tuile, l'affichage est `1250/4000`. Le max peut changer à chaque tick. L'atteindre passe la bordure de la tuile au vert. |
| `time_ms` (`1`) | Des millisecondes, affichées `HH:MM:SS`. |
| `text` (`2`) | Une chaîne, affichée telle quelle (`S`, `A+`). Inutilisable comme source de record de challenge. |

`available = 0` affiche `---`.

Dans le SDK Rust, `MetricSample::count`, `count_with_max`, `time_ms`, `text` et `unavailable` construisent ces valeurs. Surchargez `MetricPlugin::kind` quand une métrique n'est pas un simple compteur. `on_create` reçoit `HostInfo` (`overlay_version`, `base_dir`, et `log` qui écrit dans `logs/er_overlay.log`).

## Ids

Un id a la forme `nom_du_plugin.nom_de_la_metrique` : ASCII minuscule, chiffres et `_` autorisés après le premier caractère de chaque segment, au moins un point, 128 caractères au plus. `answer.value` et `score.total` sont valides. `Score.Total`, `score.` et `igt` ne le sont pas.

Ces ids sont réservés et ignorés, avec un avertissement : `igt`, `deaths`, `ng_cycle`, `scadutree_blessing`, `bosses`, `checks`, `checks_base`, `checks_dlc`, `pb`, `challenge_pb`, `nbtries`, `tries`, `challenge_tries`.

Deux plugins qui déclarent le même id : le premier chargé gagne (les noms de DLL sont triés), le second est ignoré avec un avertissement. Un plugin ne peut pas non plus masquer un objet ou un groupe agrégé qui porte déjà cet id.

Une métrique de plugin numérique peut servir de source au record ou au budget d'un challenge, puisqu'elle se résout comme n'importe quel compteur. Une métrique texte ne le peut pas.

## Langages

Tout langage qui produit une DLL Windows x64 exportant des symboles C peut implémenter ce contrat. La disposition des structures dans [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h) fait partie du contrat ; les tailles sont verrouillées par les tests de `er_overlay_plugin_abi`.

- **Sans friction :** Rust (via le SDK), C, C++, Zig, D, Odin, Nim, Delphi.
- **Possible, avec une contrainte :** C# uniquement en NativeAOT (`PublishAot` et `[UnmanagedCallersOnly]`), qui produit une vraie DLL native. Le .NET classique exigerait d'héberger CoreCLR dans le jeu, ce que cet overlay ne fait pas. Go peut produire du `c-shared`, mais il démarre son runtime et son ramasse-miettes dans le processus du jeu ; c'est une mauvaise idée ici et ce n'est pas supporté.
- **Pas directement :** Python, JavaScript, Lua, Java. Il leur faut un interpréteur. Rien n'empêche quelqu'un de livrer un seul plugin qui embarque CPython et expose des scripts comme métriques ; l'overlay n'a pas à le savoir.

Contraintes communes, quel que soit le langage : x64, disposition des structures identique au bit près, aucune exception ne traverse la frontière, et l'initialisation lourde se fait dans `er_overlay_plugin_create`, jamais dans `DllMain` (le verrou du chargeur y interdit à peu près tout).

## Lire le jeu depuis Rust

Un plugin peut dépendre du crate `er_game_state` comme de n'importe quel crate et utiliser `GameStateSource` (flags d'événement, table des boss, scan RVA) au lieu de les réécrire. C'est une dépendance à la compilation. L'overlay ne lui passe toujours rien. Chaque DLL refait le scan RVA dans ses propres statiques ; ces lectures sont idempotentes, donc c'est sans risque. En revanche ça vous couple à la version de ce crate.

## Éditeur de layout

Au démarrage, l'overlay écrit `plugins/metrics.json` à côté des DLL : une liste de `{ "id", "kind" }` et rien d'autre. Dans l'éditeur de layout, **Importer des métriques de plugin** lit ce fichier et ajoute les ids à la palette. En déposer un sur la grille préremplit le label à partir de l'id (`score.total` devient `TOTAL`) ; réécrivez-le. Importer un layout qui référence déjà un id inconnu conserve cet id au lieu de le remettre silencieusement à `igt`.

## Référence de configuration

```toml
[plugins]
enabled = true       # false ne charge rien
dir = "plugins"      # relatif à er_overlay.dll, ou un chemin absolu
disabled = []        # nom de fichier ou radical, insensible à la casse : "answer" ou "answer.dll"
```

`dir` n'est pas créé pour vous. Une installation sans dossier `plugins` se comporte exactement comme avant.
