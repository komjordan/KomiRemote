# Diagnostic du run 39085013061 — lock Flutter désynchronisé

## Cause et périmètre

Run réel : https://github.com/komjordan/KomiRemote/actions/runs/39085013061
Head : `21ebf40ea95a3dc796de23fd23d3060a36bb2dd4`.
API : validate/bridge success, windows failure. WindowInjection a dépassé MSBuild ;
l'échec suivant est `flutter pub get --enforce-lockfile` (exit 65), avant build.py.

Hypothèses classées : lock source désynchronisé des dépendances SDK (confirmé),
dérive plugins Git (réfutée par comparaison sémantique), contamination bridge ou
plateforme (réfutée pour ce symptôme par reproduction Linux exacte depuis le source).

`flutter_test` a été ajouté au pubspec amont par
`c8ba99d1a1c5c293e7b5ab9b3abc1bb5f3cc0cb9`, sans mise à jour du lock.
`vector_math` est devenu direct dans
`055826e26ff0773081d1eaa325bee750ca1e29c8`, également sans mise à jour du lock.
Le lock n'a pas de `flutter_test`, manque ses transitives et conserve des versions
incompatibles avec les pins exacts de `packages/flutter_test/pubspec.yaml` du SDK
3.24.5 (notamment test_api 0.7.2, matcher 0.12.16+1, async 2.11.0).
Le SDK est bien celui prévu : pas de raison de changer 3.24.5 ni de supprimer les tests.

La correction est **uniquement la résolution manquante du lock** : `flutter pub get`
avec le pubspec inchangé et le lock existant, SDK exact 3.24.5 ; pas `pub upgrade`,
pas de suppression du lock, pas de changement des pins Git ou du SDK.
Le petit changement de guillemets de desktop_multi_window est la sérialisation Pub,
pas une modification de ref. Cargo.lock, pubspec.yaml et workflow sont inchangés.
`--enforce-lockfile` et le contrôle git diff restent actifs. Bridge 3.22.3 et son
extended_text 13 temporaire restent isolés ; seul le code généré traverse les jobs,
jamais le pubspec/lock. Windows conserve extended_text 14.0.0.

## Diff sémantique revu

16 entrées : 9 rétrogradations imposées par flutter_test, 6 ajouts (dont flutter_test),
1 correction de catégorie direct/transitive sans changement de version.
Tous les autres paquets, tous les Git resolved-ref et les contraintes sdks sont identiques.

| Paquet | Avant | Après |
|---|---|---|
| `async` | 2.13.0 | 2.11.0 |
| `boolean_selector` | 2.1.2 | 2.1.1 |
| `fake_async` | absent | 1.3.1 |
| `flutter_test` | absent | 0.0.0 |
| `leak_tracker` | absent | 10.0.5 |
| `leak_tracker_flutter_testing` | absent | 3.0.5 |
| `leak_tracker_testing` | absent | 3.0.1 |
| `matcher` | 0.12.17 | 0.12.16+1 |
| `source_span` | 1.10.1 | 1.10.0 |
| `stack_trace` | 1.12.1 | 1.11.1 |
| `stream_channel` | 2.1.4 | 2.1.2 |
| `string_scanner` | 1.4.1 | 1.2.0 |
| `term_glyph` | 1.2.2 | 1.2.1 |
| `test_api` | 0.7.6 | 0.7.2 |
| `vector_math` | 2.1.4 | 2.1.4 |
| `vm_service` | absent | 14.2.5 |

## Provenance et reproduction réelle

SDK Linux isolé en scratch obtenu par clone Git officiel tag 3.24.5,
commit `dec2ee5c1f98f8e84a7d5380c05eb8a3d0a81668`, engine
`a18df97ca57a249df5d8d68cd0820600223ce262`, Dart **3.5.4**.
Les URLs archive/métadonnées Google essayées ont retourné 404 ; le bootstrap depuis
le clone officiel a réellement téléchargé le SDK Dart et les artefacts nécessaires.
Ce tag Git et bootstrap donnent une provenance vérifiable, pas une garantie hermétique.

Copie isolée du dossier flutter, sans fichiers générés du bridge :

1. `flutter pub get --enforce-lockfile` avant : **65**, mêmes 16 changements que CI.
2. Test `tests/test_komiremote_flutter_lock.py` ajouté avant correction : **FAIL**, 65 != 0.
3. `flutter pub get` une fois sur la copie, conservation du lock existant : **0**,
   `Changed 16 dependencies!` ; comparaison sémantique exacte ci-dessus.
4. `flutter pub get --enforce-lockfile` après : **0**, `Got dependencies!`.
5. Test de régression sur le lock corrigé : **PASS**, pubspec/lock inchangés octet par octet.
6. Suite Python KomiRemote avec intégration activée : **21 tests OK**.
7. Vrai `flutter test --no-pub test/input_modifier_utils_test.dart` sur copie :
   **36 tests OK**. Ni compilation Windows, ni génération du bridge revendiquée.

Commande de régression (SDK installé séparément) :

```bash
KOMIREMOTE_FLUTTER=/chemin/flutter-3.24.5/bin/flutter \
  python3 -m unittest discover -s tests -p 'test_komiremote*.py' -v
```

Le test est explicitement ignoré sans variable SDK ; un skip n'est pas une preuve de résolution.
Logs bruts locaux : `/home/hermes/.hermes/cache/scratch/komiremote-lock-evidence/`
(`red.log`, `resolve.log`, `green.log`, `test-red.log`, `test-green.log`,
`flutter-test.log`, `logs.zip`, `run.json`, `jobs.json`, `changes.json`).
Téléchargement des logs API : 302 intercepté, blob récupéré sans Authorization ;
aucune URL signée ni secret conservé dans ce document.

SHA256 :

- `original.lock` : `bca6dcec402c3fe2d251d76132aa4f4aa1d97c56ff2477d6f4be2ebe1f48f586`
- `repro/pubspec.lock` : `e4f87b5c7a0d9217ad0589dc9e60fb39b277f8bd1744ab663f06902e852c1008`
- `logs.zip` : `50fbae467edada6f0e08ccd23037271aa7c045c5e1639bd16bc57a0146b77f79`

## Extraits CI authentiques annotés

Source : fichier `windows/15_Build Windows runner and assemble complete unsigned bundle.txt`
du ZIP API. Les lignes ci-dessous sont les lignes pertinentes originales ; les autres
paquets inchangés ont été omis. Elles établissent le delta SDK, puis le refus strict.

```text
2026-10-18T01:30:43.7661611Z < async 2.11.0 (was 2.13.0) (2.13.1 available)
2026-10-18T01:30:43.7663477Z < boolean_selector 2.1.1 (was 2.1.2) (2.1.2 available)
2026-10-18T01:30:43.7682739Z + fake_async 1.3.1 (1.3.3 available)
2026-10-18T01:30:43.7696625Z + flutter_test 0.0.0 from sdk flutter
2026-10-18T01:30:43.7708067Z + leak_tracker 10.0.5 (11.0.2 available)
2026-10-18T01:30:43.7708420Z + leak_tracker_flutter_testing 3.0.5 (3.0.10 available)
2026-10-18T01:30:43.7708963Z + leak_tracker_testing 3.0.1 (3.0.2 available)
2026-10-18T01:30:43.7709928Z < matcher 0.12.16+1 (was 0.12.17) (0.12.20 available)
2026-10-18T01:30:43.7719630Z < source_span 1.10.0 (was 1.10.1) (1.10.2 available)
2026-10-18T01:30:43.7720449Z < stack_trace 1.11.1 (was 1.12.1) (1.12.2 available)
2026-10-18T01:30:43.7720738Z < stream_channel 2.1.2 (was 2.1.4) (2.1.4 available)
2026-10-18T01:30:43.7721288Z < string_scanner 1.2.0 (was 1.4.1) (1.4.1 available)
2026-10-18T01:30:43.7721877Z < term_glyph 1.2.1 (was 1.2.2) (1.2.2 available)
2026-10-18T01:30:43.7722304Z < test_api 0.7.2 (was 0.7.6) (0.7.15 available)
2026-10-18T01:30:43.7726160Z   vector_math 2.1.4 (from transitive dependency to direct dependency) (2.4.3 available)
2026-10-18T01:30:43.7728031Z + vm_service 14.2.5 (15.4.0 available)
2026-10-18T01:30:43.7743480Z Would change 16 dependencies.
2026-10-18T01:30:43.7768860Z Unable to satisfy `pubspec.yaml` using `pubspec.lock`.
2026-10-18T01:30:43.8787307Z ##[error]Process completed with exit code 65.
```

## Limites

Cette correction établit la cohérence de résolution au SDK Windows exact, testée sous
Linux. La relance manuelle doit encore exercer Windows avec son moteur patché, Cargo,
les plugins, CMake et le packaging. Aucun EXE livré/validé, aucune preuve CRT/ABI/runtime,
consentement, chiffrement ou session distante. Aucun merge main/release/tag ni accès infra.
