# Essai Windows x64 manuel — sécurité, recette et limites

## État réel

Le workflow `.github/workflows/komiremote-windows-test.yml` est **préparé localement,
non exécuté**. Aucun EXE, MSI, artefact Actions ou résultat Windows n'a été produit
ou validé dans cette tranche. Les contrôles locaux portent sur le YAML et sur des
fixtures ZIP/payload explicitement factices. Exception ciblée : l'archive officielle
NASM 3.01 a été téléchargée comme donnée, vérifiée contre le SHA512 du vcpkg épinglé
et extraite en scratch ; son EXE n'a jamais été exécuté.

**Sortie limitée à un bundle ZIP non autonome**, pas d'auto-extractible. La fermeture
DLL/CRT de WindowInjection et le runtime Windows ne sont pas validés. Le nom
`NOT-STANDALONE` et l'avertissement embarqué sont obligatoires. Les findings de
`/home/hermes/projects/komiremote-ci-review.md` restent historiques ; ce document
indique la réponse locale, sans lever les blocages humains ni autoriser un run.

Ce fichier est indépendant des workflows upstream : aucun `workflow_call` vers
`flutter-build.yml`, `bridge.yml` ou le workflow tiers. Leurs recettes ont été
recopiées de manière ciblée pour éviter leurs effets de publication, leurs autres
architectures, les signatures et leurs caches partagés.

## Contrat de sécurité

- Seul événement : `workflow_dispatch`, aucun push, tag, PR, cron ou appel réutilisable.
- Permission maximale déclarée : `contents: read`, aucune élévation par job.
- Aucun secret de dépôt, PAT, certificat ou service de signature référencé ;
  `persist-credentials: false` sur les deux checkouts. GitHub fournit néanmoins
  ses jetons éphémères de checkout/artifacts en interne : « sans secrets » ne
  signifie pas qu'Actions fonctionne sans authentification interne.
- Actions externes épinglées par SHA complet ; sources vcpkg et WindowInjection
  épinglées. Pas de cache Rust/Flutter/vcpkg partagé ni export de runtime token.
- Pas de commit, push, release, signature, MSI, installateur exécuté, lancement du
  client compilé, service lancé/installé, accès VPS/Proxmox ou secrets locaux.
  `git restore` et la modification temporaire du manifeste Dart ont lieu uniquement
  dans les checkouts jetables du runner. Aucun packer ni patch du manifest DPI.
- Les compilateurs, scripts de build des dépendances et actions exécutent du code :
  ceci n'est pas un sandbox hostile. Utiliser uniquement les runners GitHub hébergés
  nommés, jamais un runner de production/self-hosted avec identifiants.
- Les uploads sont **uniquement des artefacts Actions**, pas des GitHub Releases :
  bridge 3 jours, sortie Windows 7 jours, `if: success()`, fichier absent = erreur.
  Le bridge peut exister même si Windows échoue ; aucun artefact Windows n'est
  uploadé après échec. Aucun upload de logs supplémentaire après échec ; consulter
  alors les logs Actions, soumis à la rétention du dépôt.
- Délais : validation 5 min, bridge 60 min, Windows 180 min. Pas d'annulation d'un
  essai en cours lors d'un second dispatch sur la même ref.

## Prérequis humains avant un éventuel lancement autorisé

1. Revoir le diff/commit exact et les workflows upstream conservés (risques ci-dessous).
   Rien dans cette tranche n'autorise un push, merge, modification de paramètres
   GitHub ou un lancement effectif. Ne pas utiliser un push de tag pour tester.
2. Obtenir par revue indépendante les SHA256 approuvés des **quatre** archives
   listées ci-dessous. Les valeurs ne sont pas préremplies ni inventées. Un digest
   calculé sur un téléchargement du même run ou sur `sha256sums` du même tag mutable
   ne constitue pas une approbation indépendante. Conserver l'origine et la revue
   des digests hors de la simple saisie Actions.
   **Blocage B1 maintenu** : selon la revue, engine (asset `210168939`) et USB
   (`180784412`) ont un digest API `null`. Printer (`276730335`) et adapter
   (`328545268`) ont des digests officiels, pas une approbation indépendante.
   Aucun des quatre inputs n'est prérempli. Une décision humaine documentée sur
   les quatre assets et une méthode acceptable pour les deux absents restent requises.
   Le préflight 64-hex ne prouve jamais cette décision.
3. Après autorisation et mise à disposition du workflow sur la branche par défaut
   (condition GitHub pour la disponibilité de `workflow_dispatch`), choisir
   explicitement une ref revue. Contrôler son SHA dans le run. Ne pas lancer
   `Flutter Tag Build`, `Flutter Nightly Build`, `playground` ni les workflows stock.
4. Saisir les quatre digests de 64 caractères hexadécimaux. Les champs entrent via
   `env` et ne sont jamais interpolés comme code shell. La validation précède tous
   les jobs de compilation ; la vérification SHA précède toute extraction.
5. Les artefacts sont des **tests non signés**, potentiellement téléchargeables par
   les lecteurs du dépôt public. Ne pas les présenter comme distribution validée,
   portable sans persistance démontrée, ou binaire signé. Ne pas l'exécuter sur une
   machine de production ; réserver toute validation runtime à un environnement
   Windows isolé avec revue du consentement/services.

| Champ | URL fixe, identique à la recette upstream |
|---|---|
| `engine_sha256` | `https://github.com/rustdesk/engine/releases/download/main/windows-x64-release.zip` |
| `usbmmidd_sha256` | `https://github.com/rustdesk-org/rdev/releases/download/usbmmidd_v2/usbmmidd_v2.zip` |
| `printer_sha256` | `https://github.com/rustdesk/hbb_common/releases/download/driver/rustdesk_printer_driver_v4-1.4.zip` |
| `adapter_sha256` | `https://github.com/rustdesk/hbb_common/releases/download/driver/printer_driver_adapter.zip` |

Ces tags/assets restent mutables ; tout changement de contenu provoquera un échec
avec les digests approuvés. Si l'ancien asset disparaît, le run échouera : pas de
fallback vers `latest`, autre moteur, checksum auto-adopté ou suppression de pilote.
Le téléchargeur vérifie aussi les chemins ZIP, noms réservés Windows, doublons
insensibles à la casse et symlinks avant extraction. Il ne lance aucun fichier
extrait ; les outils/moteurs utilisés ensuite pour compiler restent du code de
confiance à auditer. Aucun client ou installateur du payload n'est exécuté.

## Séquence et versions

1. **Bridge Linux séparé**, `ubuntu-22.04`, sous-module récursif au gitlink,
   Rust **1.75.0**, Flutter **3.22.3**, cargo-expand **1.0.95** (`--locked`),
   flutter_rust_bridge_codegen **1.80.1**, feature `uuid` (`--locked`). Packages
   APT repris de `bridge.yml` : ca-certificates, clang, cmake, curl, gcc, git, g++,
   libclang-dev, libgtk-3-dev, llvm-dev, nasm, ninja-build, pkg-config, wget.
   La substitution `extended_text: 14.0.0` → `13.0.0` est isolée ; `pubspec.yaml`
   et son lock sont restaurés après génération. Seuls les six fichiers upstream
   (deux Rust, Dart/freezed, headers macOS/iOS) sont transmis à Windows.
2. **Windows x64**, `windows-2022`/VS 2022/SDK de l'image ; Flutter **3.24.5**,
   LLVM **15.0.6**, Rust **1.75.0**/`x86_64-pc-windows-msvc`/rustfmt,
   vcpkg **9e593bb18ea69cc5095e012465dcd675a822ed0d**,
   triplet **x64-windows-static**, variable upstream `VCPKG_CMAKE_VERSION=4.3.0`.
   vcpkg est cloné sans credentials persistés et sans cache ; bootstrap/métriques
   désactivées, packages du manifeste/overlays installés normalement.
   `CMake 4.3.0` dans l'environnement n'est pas une preuve du CMake effectif :
   `tools.txt` enregistre les versions disponibles/résolues, pas tous les compilateurs
   effectivement employés par Flutter/vcpkg. CMake/MSBuild restent ceux de l'image.
   **NASM 3.01** est provisionné explicitement par `provision_nasm.py` avant vcpkg :
   ZIP officiel win64, SHA512 vérifié avant extraction, `nasm.exe -v` vérifié par
   chemin absolu, répertoire ajouté à `GITHUB_PATH`. Le step suivant enregistre
   `command -v nasm` et `nasm -v`, puis vérifie par `samefile` que le PATH résout
   bien le fichier validé, avant le build natif long. Tout échec est bloquant.
   `nasm.json` conserve URL/hash/version/chemin/provenance. Le pin vient de
   [la recette NASM du vcpkg épinglé](https://github.com/microsoft/vcpkg/blob/9e593bb18ea69cc5095e012465dcd675a822ed0d/scripts/cmake/vcpkg_find_acquire_program(NASM).cmake) :
   `771c238ddb17c98d5736ccaba4ade1d1601d896f09e588489cb43a4f6381bc0ae14d1869f5316fe94f847f54867e65cf12665529b1e7ad88e5e7d3e162719a4f`.
   Source/version/hash et octets de l'archive ont été comparés réellement sous Linux ;
   l'exécution de NASM et l'export du PATH restent à exercer sur Windows.
   Ce pin d'outil n'est pas une approbation des quatre payloads natifs.
3. Téléchargement des quatre assets approuvés ; remplacement du moteur release
   x64 dans le SDK, puis patch SDK upstream
   `flutter_3.24.4_dropdown_menu_enableFilter.diff` avec contrôle `git apply --check`.
4. Construction `WindowInjection.dll` par MSBuild Release/x64/Windows10 depuis
   **ecd8d6a139eee76845ea66423fb739af450fda90**. Pas de téléchargement de DLL
   non épinglée ni appel au workflow tiers upstream. Le projet requiert v142 ; sa
   disponibilité réelle reste à prouver sur le runner. Aucune réorientation v143
   ou surcharge `/MT` non validée n'est appliquée. Voir blocage CRT ci-dessous.
5. `flutter pub get --enforce-lockfile` puis commande officielle
   `python build.py --portable --flutter --skip-portable-pack --hwcodec --vram`.
   Le script upstream appelle Cargo `--locked` et Flutter Release. Toute modification
   de `Cargo.lock`, `flutter/pubspec.lock` ou `flutter/pubspec.yaml` échoue après build.
6. Assemblage vérifié en présence, pas en exécution : EXE/DLL Rust, moteur,
   affichage virtuel, `data/icudtl.dat`, `data/app.so`, `AssetManifest.bin`, icône
   Flutter ; chaque fichier du vrai `flutter/build/windows/x64/install_manifest.txt`
   doit être non vide dans le bundle, dont les plugins/CRT installés par CMake.
   Un manifeste incomplet, sans plugin ou hors `runner/Release` échoue.
   Les INF **consommés par le runtime** sont requis : `usbmmidd_v2/usbmmIdd.inf`
   (casse Windows) et `drivers/RustDeskPrinterDriver/RustDeskPrinterDriver.inf`.
   Leurs CatalogFile et SourceDisksFiles doivent référencer des fichiers présents.
   Le validateur est volontairement conservateur, pas un moteur SetupAPI :
   formes indirectes/localisées non reconnues, layout modifié ou dépendance après
   retrait Win32/helpers = échec à revoir, jamais un fallback silencieux. Aucun
   layout des quatre ZIP natifs n'a été validé sur leurs octets ici (B1 non levé).
7. Renommage **au packaging seulement** de `rustdesk.exe` en `KomiRemote.exe` ;
   CMake/build.py, `librustdesk.dll`, exports FFI et noms drivers inchangés.
   Le futur wrapper upstream déduirait `komiremote` du stem (comparaison Windows
   insensible à la casse), distinct de `%LOCALAPPDATA%\rustdesk`. Mais aucun wrapper
   n'est construit maintenant : pas d'extraction/suppression automatique de ce
   répertoire par le packaging. Le ZIP contient un dossier `KomiRemote/`, avec
   `KomiRemote.exe`, le payload entier, `LICENCE` et `NOT-STANDALONE.txt`.
   Le renommage est testé en fixture, pas par un démarrage réel Windows.
8. `inspect_imports.ps1` trouve dumpbin x64 de VS via vswhere et conserve `/HEADERS`,
   `/DEPENDENTS`, `/IMPORTS` pour les EXE/DLL/SYS, dont WindowInjection. Tout échec
   du lecteur empêche l'upload. Le chemin du lecteur et le manifeste CMake sont
   archivés. **Ce rapport n'est pas un contrôle de fermeture DLL**, ni une preuve
   d'architecture/ABI compatible ou de chargement des DLL dynamiques.
9. L'upload contient `KomiRemote-bundle-x64-unsigned-NOT-STANDALONE.zip`, rapports PE,
   `LICENCE`, inventaire Python/outils/vcpkg, `nasm.json`, commit/gitlink/image/run,
   URLs/SHA256 vérifiés des quatre archives, `sha256.json` des autres fichiers.

### Blocage CRT / critères avant tout portable autonome

Le runner Flutter `/MT` ne rend pas statique WindowInjection, compilée séparément
avec v142 et runtime implicite du projet épinglé. Faute de compilation et VM Windows,
aucune solution CRT n'est déclarée sûre : ni copie arbitraire de DLL redistribuables,
ni changement de toolset/RuntimeLibrary, ni dépendance supposée satisfaite par l'OS.
Les rapports d'imports sont seulement préparés, **non produits ici**. Il faut ensuite
revoir les imports normaux/différés et chargements dynamiques du résultat exact,
choisir une stratégie CRT/licences acceptable (redistribuable externe ou app-local
revu, ou build statique réellement testé), puis prouver chargement/privacy sur VM
propre sans environnement VS et coexistence avec RustDesk. Le bundle n'inclut pas
les CRT tierces par supposition ; il peut nécessiter un redistribuable externe.
Un simple build réussi ne lève aucun de ces critères.

**Aucun auto-extractible autonome, MSI, signature ni exécution du client.** Le flag
upstream `--portable --skip-portable-pack` prépare le runner sans appeler le packer.
`Runner.res` n'est plus copié : cette copie ne prouvait pas les ressources du wrapper.

## Reproductibilité : amélioration, pas garantie binaire

- Pins upstream conservés, actions SHA, aucun cache partagé, `Cargo.lock` et
  lock Dart contrôlés, quatre digests approuvés requis et provenance enregistrée.
- Images GitHub, paquets APT/SDK/MSVC/CMake et téléchargements transitifs
  vcpkg ne sont pas tous immuables. La bootstrap vcpkg et les actions requièrent le
  réseau ; l'approvisionnement de leurs outils n'est pas une chaîne hermétique.
- La génération bridge change temporairement la résolution Dart (upstream) ; le
  lock généré n'est pas réutilisé par Windows. Un conflit réel Rust/MSRV/codegen
  ou lock Dart est un **échec à diagnostiquer**, pas une permission de mettre à jour.
- Le packer/brotli ne sont plus utilisés. Le setup Python et les distributions
  Flutter/LLVM héritent de leurs téléchargements. NASM possède un SHA512 épinglé,
  ce qui vérifie l'identité des octets, pas un audit de l'outil.
- Le build génère des dates/ressources ; l'ordre des fichiers, les timestamps du ZIP
  et les versions d'outils peuvent varier. Aucun bit-à-bit garanti.
- Compatibilité serveur/clé, chiffrement, consentement, absence de service runtime,
  SmartScreen, tray et install/uninstall sont **non testés**. Les composants pilotes
  et licences tierces restent à auditer avant toute distribution externe.

## Risques des workflows upstream conservés, à remonter avant tout push

Aucun de ces fichiers n'est modifié par cette tranche. L'autorisation read-only de
notre nouveau workflow **ne s'applique pas aux autres workflows**.

| Fichier et lignes de référence | Déclencheur / effet vérifié statiquement |
|---|---|
| `flutter-tag.yml:3-18` | Dispatch ou **push de tags** correspondant aux quatre motifs de versions (`v[0-9]+.[0-9]+.[0-9]+`, sans v, et variantes suffixées `-[0-9]+`). Appelle `flutter-build.yml`, hérite des secrets, passe `upload-artifact: true` et le tag poussé. |
| `flutter-build.yml:57-83,349-361,363-396,454-…` | Workflow réutilisable, pas directement push. Le flag trompeur `upload-artifact` commande aussi **GitHub Releases** via `softprops/action-gh-release`, MSI et signature si secrets configurés. `generate-sbom` demande explicitement `contents: write` (`:60-61`) ; d'autres publications dépendent des permissions effectives héritées. Ne pas supposer que l'absence de secrets utilisateur empêche une release via `GITHUB_TOKEN`. |
| `fdroid.yml:3-10,33-39` | Même famille de **push de tags** ou dispatch ; publie `rustdesk-version.txt` dans la prerelease `fdroid-version`. Pas de permission explicite : dépend des permissions effectives Actions ; risque réel à examiner, pas succès garanti. |
| `flutter-nightly.yml:3-15` | **Cron quotidien 00:00 UTC** ou dispatch, appelle le même build avec héritage de secrets et publication tag `nightly`. Pas déclenché par un push ordinaire ; son activation dépend de l'état Actions et de la branche par défaut. |
| `ci.yml:10-25`, `flutter-ci.yml:3-24` | Push **uniquement master**, filtres de chemins, ou PR (seuls docs/** et README.md ignorés pour PR), ou dispatch. `flutter-ci` passe `upload-artifact: false` ; pas une publication normale par ce chemin, mais coûteux et jobs upstream toujours invoqués, dont un job demandant contents:write. Une mise à jour de PR touchant `.github`, scripts ou tests peut donc déclencher ces CI stock ; vérifier les réglages avant un futur push. |
| `update-webpki-roots.yml:13-16,28-30,59-74` | Cron lundi **03:00 UTC** ou dispatch ; demande contents/PR write, crée un commit et **force-push** `auto-update-webpki-roots`, puis ouvre une PR. Pas un trigger push ni une release, mais effet automatique incompatible avec une politique globale « aucun commit/push automatique ». |
| `playground.yml` | Manuel, contient des publications/signatures ; pas un workflow de test sûr à utiliser à la place de celui-ci. |

Aucun workflow de publication directe sur un push ordinaire de la branche
`work/komiremote-branding` ou `main` n'a été repéré dans cet inventaire local.
**Ce constat n'exclut ni les événements PR, ni les tags, ni les crons, ni des réglages
GitHub externes** : l'état d'activation, les droits par défaut et règles distantes
n'ont pas été consultés/modifiés. Le parent doit faire décider séparément d'une
neutralisation ciblée avant tout futur push/merge ; ne pas modifier upstream ici.

## Vérifications locales reproductibles

Depuis la racine du dépôt, avec Python et PyYAML (version locale contrôlée : 6.0.3) :

```bash
python3 -m unittest discover -s tests -p 'test_komiremote*.py' -v
# Après installation séparée d'actionlint 1.7.7 :
actionlint -shellcheck='' .github/workflows/komiremote-windows-test.yml
git diff --check
```

Résultats de la correction : voir [VALIDATION.md](VALIDATION.md), section revue CI.
Cycles RED→GREEN observés pour SHA512/NASM, payload incomplet, nom d'EXE distinct,
INF/manifeste CMake, repli ZIP et contenu du ZIP. Aucune commande de build Windows,
génération bridge ou dispatch CI n'a été exécutée ; PowerShell/dumpbin non exercés.

Les tests lisent le YAML avec `yaml.BaseLoader` pour éviter le piège YAML 1.1 qui
interprète `on` comme booléen. Ils vérifient événement/droits/actions/pins/artefacts,
l'isolation du bridge et les inputs ; les fixtures testent les SHA erronés, les
chemins ZIP dangereux et l'assemblage des composants. `actionlint` contrôle la
syntaxe Actions et ses expressions ; ShellCheck n'est pas inclus dans cette
commande. Aucun de ces contrôles ne remplace une compilation Windows effective.
