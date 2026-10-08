# Plugins de métriques

Un plugin de métriques est une DLL native Windows **x64**. On la dépose dans `plugins/` à côté de `er_overlay.dll` et l'overlay la charge au démarrage. Le plugin déclare des ids de métriques et renvoie un nombre (ou une courte chaîne) à chaque tick. Le **fichier de layout** choisit le label, l'icône, `show_max` et la position de la tuile. L'overlay ne fournit aucune donnée de jeu au plugin : le plugin lit ce dont il a besoin, comme il veut.

> **Un plugin est du code arbitraire dans le processus d'Elden Ring.** Un plugin bogué fait planter le jeu. Il n'y a pas de bac à sable. Ne chargez que des plugins auxquels vous faites confiance.

## Quelle API utiliser ?

Il n'y a **qu'un** contrat : six exports C définis dans [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h) (ABI version `1`). Tout le reste est un habillage autour de cette ABI.

| Langage | Ce que vous utilisez | Exemple |
| --- | --- | --- |
| **Rust** | Crate `er_overlay_plugin_sdk` : implémenter `MetricPlugin`, appeler `declare_plugin!` | [`examples/plugins/demo_rust`](../examples/plugins/demo_rust) |
| **C / C++** | Inclure `er_overlay_plugin.h`, implémenter les six exports | [`examples/plugins/demo_cpp`](../examples/plugins/demo_cpp) |
| **.NET** | NativeAOT (`PublishAot`) + `[UnmanagedCallersOnly]` ; recopier les structures depuis l'exemple | [`examples/plugins/demo_dotnet`](../examples/plugins/demo_dotnet) |

Le .NET managé classique (héberger CoreCLR dans le jeu) **n'est pas** supporté. Go en `c-shared` est déconseillé : il démarre un runtime et un GC dans le processus du jeu.

Les trois démos exposent la même forme de métriques sous des préfixes d'id différents, pour pouvoir les installer côte à côte :

| Métrique | Type | Sens |
| --- | --- | --- |
| `<prefixe>.uptime` | `time_ms` | `tick_ms` fourni par l'hôte |
| `<prefixe>.progress` | `count` avec max `60` | `(tick_ms / 1000) % 61` |
| `<prefixe>.rank` | `text` | `C` / `B` / `A` / `S` selon progress |

Préfixes : `demo_rust`, `demo_cpp`, `demo_dotnet`.

Pour le plus petit plugin possible (renvoie toujours `42`), voir [`examples/plugins/answer`](../examples/plugins/answer).

## Intégrer un plugin (tous langages)

1. **Compiler** une DLL Windows x64 qui exporte les six symboles (voir les guides par langage ci-dessous).
2. **Livrer un manifeste de métriques** à côté de la DLL : `votre_plugin.metrics.json` (même radical que la DLL). Même format que ci-dessous. Gardez-le aligné avec les ids déclarés dans le code — l'éditeur de layout ne sait pas lire les DLL.
3. **Copier** à la fois `votre_plugin.dll` et `votre_plugin.metrics.json` dans le dossier `plugins/` à côté de `er_overlay.dll`. Créez le dossier s'il n'existe pas.
4. **Activer** les plugins dans `er_overlay.toml` :

```toml
[plugins]
enabled = true
dir = "plugins"
disabled = []   # par ex. ["demo_rust"] pour ignorer demo_rust.dll sans la supprimer
```

5. **Éditeur de layout (sans lancer le jeu) :** dans `layout_editor.html`, utilisez **Importer des métriques de plugin** et sélectionnez un ou plusieurs fichiers `*.metrics.json` (le sidecar que vous venez de copier, ou plusieurs d'un coup). Les ids apparaissent dans la palette. Vous pouvez éditer et exporter un layout avant de lancer Elden Ring.
6. **Redémarrer le jeu** quand vous voulez que l'overlay charge la DLL. Les ids de métriques sont figés au démarrage ; une DLL rejetée est journalisée puis ignorée.
7. **Ajouter une tuile** dans votre layout (ou finissez le layout dans l'éditeur). Les layouts fournis utilisent des sections :

```toml
[[section.tile]]
kind = "metric"
metric = "demo_rust.progress"
label = "PROGRESS"
col = 0
row = 0
show_max = true
```

Pour un layout sans `[[section]]`, utilisez `[[tile]]` à la place (mêmes champs). Le layout est rechargé à chaud, sans redémarrer. Si le plugin est absent, la tuile affiche `---` et rien d'autre ne casse.

8. **Vérifier le log.** Avec `log_enabled = true`, `logs/er_overlay.log` indique chaque plugin chargé et le nombre de métriques enregistrées. Les échecs (mauvaise ABI, export manquant, `create` a renvoyé null) y apparaissent aussi.
9. **Icônes** (optionnel) : placez un PNG sous `plugins/icons/` (ou `assets/icons/`). Référencez-le avec `icon = "mon_icone"` (sans `.png`).

### Manifeste de métriques (`*.metrics.json`)

Livrez ce fichier à côté de la DLL pour que les auteurs et l'éditeur de layout connaissent la liste des métriques **sans** lancer le jeu :

```json
{
  "metrics": [
    { "id": "demo_rust.uptime", "kind": "time_ms" },
    { "id": "demo_rust.progress", "kind": "count" },
    { "id": "demo_rust.rank", "kind": "text" }
  ]
}
```

`kind` vaut `count`, `time_ms` ou `text`. Exemples : [`answer.metrics.json`](../examples/plugins/answer/answer.metrics.json), [`demo_rust.metrics.json`](../examples/plugins/demo_rust/demo_rust.metrics.json).

Après avoir chargé des plugins en jeu, l'overlay écrit aussi un `plugins/metrics.json` agrégé. Ce fichier est optionnel pour éditer un layout — le sidecar par plugin suffit.

## Guide Rust

### Dépendances

`Cargo.toml` d'un crate autonome (hors du workspace de l'overlay) :

```toml
[workspace]

[package]
name = "demo_rust"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Depuis un clone local :
er_overlay_plugin_sdk = { path = "chemin/vers/ER_DynamicOverlay/sdk/er_overlay_plugin_sdk" }
# Ou depuis git (tag qui contient le SDK) :
# er_overlay_plugin_sdk = { git = "https://github.com/Sully-/ER_DynamicOverlay.git", tag = "v1.4.3", package = "er_overlay_plugin_sdk" }

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

`panic = "abort"` est obligatoire : une panic ne doit pas se dérouler à travers la frontière C. `crate-type = ["cdylib"]` produit une DLL.

### Implémenter le plugin

```rust
use er_overlay_plugin_sdk::{
    abi, declare_plugin, HostInfo, MetricKind, MetricPlugin, MetricSample,
};

struct Demo {
    tick_ms: u64,
}

impl Demo {
    fn new() -> Self {
        Self { tick_ms: 0 }
    }
}

impl MetricPlugin for Demo {
    fn metrics() -> &'static [&'static str] {
        &["demo_rust.uptime", "demo_rust.progress", "demo_rust.rank"]
    }

    // Obligatoire pour time_ms / text : MetricSample::time_ms ne fixe PAS le type.
    fn kind(index: usize) -> MetricKind {
        match index {
            0 => MetricKind::TimeMs,
            1 => MetricKind::Count,
            2 => MetricKind::Text,
            _ => MetricKind::Count,
        }
    }

    fn on_create(&mut self, host: &HostInfo) {
        host.log(
            abi::ER_LOG_INFO,
            &format!("demo_rust: overlay {} @ {}", host.overlay_version, host.base_dir),
        );
    }

    fn poll(&mut self, tick_ms: u64) {
        self.tick_ms = tick_ms;
    }

    fn sample(&mut self, index: usize) -> MetricSample {
        let progress = ((self.tick_ms / 1000) % 61) as i64;
        match index {
            0 => MetricSample::time_ms(self.tick_ms as i64),
            1 => MetricSample::count_with_max(progress, 60),
            2 => MetricSample::text(if progress <= 14 { "C" } else if progress <= 29 { "B" } else if progress <= 44 { "A" } else { "S" }),
            _ => MetricSample::unavailable(),
        }
    }
}

declare_plugin!(Demo, Demo::new());
```

Sources complètes : [`examples/plugins/demo_rust`](../examples/plugins/demo_rust).

### Compiler

```bash
cargo build --release
```

DLL : `target/release/demo_rust.dll` (ou `target/x86_64-pc-windows-msvc/release/demo_rust.dll` si cette cible est utilisée).

Livrez [`demo_rust.metrics.json`](../examples/plugins/demo_rust/demo_rust.metrics.json) à côté de la DLL (copiez les deux dans `plugins/`).

## Guide C++

### Prérequis

- MSVC (Visual Studio « Développement Desktop en C++ ») ou MinGW-w64.
- CMake 3.20+ (recommandé), ou une ligne `cl`.

### Sources

Incluez [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h). Le header définit `ER_OVERLAY_PLUGIN_API` comme `__declspec(dllexport)` sous Windows, donc pas besoin de fichier `.def`.

Règles spécifiques au C++ :

- Attrapez les exceptions dans chaque export ; ne les laissez jamais sortir.
- Gardez les littéraux (ou buffers) d'ids de métriques vivants jusqu'à `destroy`.
- Gardez le `text` d'un échantillon valide jusqu'au **prochain** `poll` (la démo le stocke dans un `std::string` membre vidé au début de `poll`).
- Préférez le CRT statique (`/MT`) pour ne pas dépendre d'un `VCRUNTIME` correspondant à côté du jeu.

Sources complètes et CMake : [`examples/plugins/demo_cpp`](../examples/plugins/demo_cpp).

### Compiler avec CMake

Depuis `examples/plugins/demo_cpp` :

```bash
cmake -S . -B build -A x64
cmake --build build --config Release
```

DLL : `build/Release/demo_cpp.dll` (MSVC) ou `build/demo_cpp.dll` (Ninja / MinGW).

Livrez [`demo_cpp.metrics.json`](../examples/plugins/demo_cpp/demo_cpp.metrics.json) à côté de la DLL (copiez les deux dans `plugins/`).

### Compiler avec `cl` (invite de commandes développeur MSVC)

```bat
cl /LD /EHsc /MT /O2 /I..\..\..\sdk demo_cpp.cpp /Fe:demo_cpp.dll
```

## Guide .NET (NativeAOT)

Le .NET Framework / CoreCLR classique hébergé dans Elden Ring **n'est pas** supporté. Il faut publier une DLL **native** avec NativeAOT.

### Prérequis

- SDK .NET 8 ou plus récent.
- La charge de travail Visual Studio **Développement Desktop en C++** (NativeAOT a besoin du linker MSVC).

### Réglages du projet

Propriétés clés (voir [`examples/plugins/demo_dotnet/DemoDotnet.csproj`](../examples/plugins/demo_dotnet/DemoDotnet.csproj)) :

```xml
<PropertyGroup>
  <TargetFramework>net8.0</TargetFramework>
  <RuntimeIdentifier>win-x64</RuntimeIdentifier>
  <PublishAot>true</PublishAot>
  <NativeLib>Shared</NativeLib>
  <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
  <AssemblyName>demo_dotnet</AssemblyName>
</PropertyGroup>
```

Exportez les six points d'entrée avec `[UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_...")]`. Recopiez `ErMetricDesc` / `ErMetricSample` / `ErHostInfo` champ par champ (`[StructLayout(LayoutKind.Sequential)]`). Gardez les buffers UTF-8 des ids jusqu'à `destroy` ; libérez le texte d'échantillon au prochain `poll`.

NativeAOT embarque son propre GC dans le processus du jeu. Gardez `poll` / `sample` légers et évitez les allocations lourdes à chaque tick.

Sources complètes : [`examples/plugins/demo_dotnet`](../examples/plugins/demo_dotnet).

### Compiler

```bash
dotnet publish -c Release
```

DLL : `bin/Release/net8.0/win-x64/native/demo_dotnet.dll` (le chemin peut légèrement varier selon la version du SDK ; cherchez sous `native/`).

Livrez [`demo_dotnet.metrics.json`](../examples/plugins/demo_dotnet/demo_dotnet.metrics.json) à côté de la DLL (copiez les deux dans `plugins/`).

## Référence ABI (version 1)

Header du contrat : [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h). Les tailles sont verrouillées par les tests de `er_overlay_plugin_abi` et par des `static_assert` dans le header :

| Structure | Taille |
| --- | --- |
| `ErMetricDesc` | 24 octets |
| `ErMetricSample` | 32 octets |
| `ErHostInfo` | 32 octets |

### Ordre des appels

Tous les appels ont lieu sur le **thread de poll** de l'overlay (~250 ms), jamais sur le thread de rendu. `poll` / `sample` ne tournent que lorsque le lecteur d'état du jeu est prêt (pas sur l'écran titre).

| Export | Moment |
| --- | --- |
| `er_overlay_plugin_abi_version` | Une fois, avant tout le reste. Doit renvoyer `1`. |
| `er_overlay_plugin_create` | Une fois. Renvoyer null refuse le chargement. |
| `er_overlay_plugin_metrics` | Une fois, juste après create. La liste d'ids est ensuite figée. |
| `er_overlay_plugin_poll` | À chaque tick, avant les lectures. L'argument `tick_ms` est le nombre de millisecondes depuis le démarrage du thread de poll. |
| `er_overlay_plugin_sample` | Une fois par métrique acceptée et par tick. L'argument est un **index** dans le tableau metrics, pas une chaîne. |
| `er_overlay_plugin_destroy` | Au déchargement de l'overlay. |

`poll` est un battement de cœur, pas une garantie d'échantillonnage. Pour observer quelque chose de plus court que ~250 ms, lancez votre propre thread et faites lire à `sample` la dernière valeur publiée. Un `poll` qui dure **50 ms ou plus** est journalisé comme lent.

### Propriété des pointeurs

- Ne **retenez pas** le pointeur `ErHostInfo*`. Copiez `overlay_version` / `base_dir` si besoin ; `log` reste valide jusqu'au retour de `destroy`.
- Les ids de métriques que vous renvoyez doivent rester valides jusqu'à `destroy`.
- Le `text` d'un échantillon doit rester valide jusqu'au prochain `poll`.
- L'état (compteurs, fichiers sous `base_dir`) appartient au plugin. L'overlay ne le sauvegarde ni ne le réinitialise.

### Types de valeur

Le descripteur porte un id et un type. Le type dit à l'overlay comment afficher :

| Type | Valeur | Affichage |
| --- | --- | --- |
| `count` | `0` | Entier. Avec `has_max` sur l'échantillon et `show_max` sur la tuile : `1250/4000`. Atteindre le max passe la bordure au vert. |
| `time_ms` | `1` | Millisecondes, affichées `HH:MM:SS`. |
| `text` | `2` | Chaîne telle quelle (`S`, `A+`). Inutilisable comme source de record de challenge. |

`available = 0` affiche `---`.

**Plage côté hôte :** pour `count` et `time_ms`, l'overlay convertit `value` (et `max` le cas échéant) avec `u32::try_from`. Une valeur négative ou au-delà de `u32::MAX` devient `---`.

Dans le SDK Rust : `MetricSample::count`, `count_with_max`, `time_ms`, `text`, `unavailable`. Surchargez `MetricPlugin::kind` quand la métrique n'est pas un simple compteur — `time_ms()` ne remplit que le champ numérique.

### Ids de métriques

Forme : `nom_du_plugin.nom_de_la_metrique`. Règles :

- ASCII minuscule ; chiffres et `_` autorisés après le premier caractère de chaque segment ;
- au moins un point (deux segments ou plus) ;
- 3 à 128 caractères.

Valides : `answer.value`, `score.total`. Invalides : `Score.Total`, `score.`, `igt`.

Réservés (ignorés avec un avertissement) : `igt`, `deaths`, `ng_cycle`, `scadutree_blessing`, `bosses`, `checks`, `checks_base`, `checks_dlc`, `pb`, `challenge_pb`, `nbtries`, `tries`, `challenge_tries`.

Ids en double : le premier plugin chargé gagne (noms de DLL triés), les suivants sont ignorés avec un avertissement. Un plugin ne peut pas non plus masquer un objet ou un groupe agrégé qui porte déjà cet id.

Une métrique de plugin numérique peut servir de source au record ou au budget d'un challenge ; une métrique texte ne le peut pas.

## Checklist des pièges

- Windows **x64** uniquement ; disposition des structures identique au bit près.
- Aucune exception C++ / panic Rust / exception .NET ne doit traverser la frontière.
- Pas de travail lourd dans `DllMain` (verrou du chargeur).
- Ne pas garder le pointeur `ErHostInfo*` après le retour de `create`.
- Garder `poll` sous 50 ms (sinon avertissement) ; ne pas bloquer le thread de poll.
- Redémarrer le jeu après avoir changé la liste des métriques déclarées par un plugin.
- Activer les logs pour déboguer les échecs de chargement.

## Lire le jeu depuis Rust

Un plugin peut dépendre du crate `er_game_state` et utiliser `GameStateSource` (flags d'événement, table des boss, scan RVA). C'est une dépendance à la compilation uniquement : l'overlay ne lui passe toujours rien à l'exécution. Chaque DLL refait le scan RVA dans ses propres statiques (idempotent, sans risque). En revanche ça vous couple à la version de ce crate.

## Autres langages

Tout langage qui produit une DLL Windows x64 exportant des symboles C peut implémenter l'ABI.

- **Sans friction :** Rust (via le SDK), C, C++, Zig, D, Odin, Nim, Delphi.
- **Possible, avec une contrainte :** C# uniquement en NativeAOT (voir ci-dessus). Go en `c-shared` démarre un runtime/GC dans le processus — non supporté ici.
- **Pas directement :** Python, JavaScript, Lua, Java (il faut un interpréteur). Rien n'empêche de livrer un plugin qui embarque CPython et expose des scripts comme métriques ; l'overlay n'a pas à le savoir.

## Référence de configuration

```toml
[plugins]
enabled = true       # false ne charge rien
dir = "plugins"      # relatif à er_overlay.dll, ou un chemin absolu
disabled = []        # nom de fichier ou radical, insensible à la casse : "answer" ou "answer.dll"
```

`dir` n'est pas créé pour vous. Une installation sans dossier `plugins` se comporte comme avant (aucun plugin).
