# Validation de la tranche

## Correction de la revue CI — état local actuel

- Branche `work/komiremote-branding`, rapport complet
  `/home/hermes/projects/komiremote-ci-review.md` lu avant modifications.
  Sources workflow/helpers/packer/FFI/CMake et routes INF relues ; sources publiques
  NASM/vcpkg et WindowInjection consultées au SHA exact (pas de `latest`).
- **B1 maintenu** : quatre SHA256 approuvés exigés avant téléchargement/extraction.
  Engine/USB sans digest officiel dans l'instantané de revue : décision humaine
  documentée indispensable. Les digests officiels printer/adapter ne sont pas une
  approbation indépendante. Aucun input par défaut, checksum inventé ou dérogation.
- **B2 corrigé dans la recette, non exécuté sous Windows** : NASM 3.01 provisionné
  explicitement, SHA512 issu de vcpkg épinglé, extraction après vérification,
  version contrôlée par chemin absolu, export `GITHUB_PATH`, vérification `samefile`
  du NASM résolu puis préflight avant vcpkg. Le snippet de résolution est exercé
  sous Linux avec chemins correct/incorrect factices, sans exécuter NASM.
  Réellement vérifié sur Linux : recette publique version/hash concordants,
  téléchargement ZIP NASM officiel, SHA512 concordant, présence `nasm-3.01/nasm.exe`
  et en-tête de fichier `MZ`. Aucun EXE NASM exécuté, scratch supprimé après contrôle.
- **R2 renforcé** : ICU/AOT/manifest/assets non vides, sorties réelles du manifeste
  CMake dont plugins/CRT installés, INF exacts runtime et fichiers CatalogFile /
  SourceDisksFiles avant/après retrait USB helpers. Tests négatifs retirant les
  vrais chemins requis et test anti-traversée INF. Le parseur INF conservateur
  échoue sur layouts inconnus ; ce n'est pas une validation SetupAPI/catalogue signé.
- **R3 isolé au packaging** : runner assemblé `KomiRemote.exe`, DLL/FFI inchangés,
  dossier ZIP `KomiRemote/`. Test réel du snippet ZIP en fixture et relecture des
  membres ; aucun wrapper auto-extractible ni suppression de localappdata par le
  packaging. Le stem serait `komiremote`, pas `rustdesk`, pour un futur wrapper revu.
  Coexistence et démarrage Windows encore non testés.
- **R1 non résolu sans Windows** : aucun `/MT`, retargeting ou CRT redistribué par
  supposition. v142 et CRT de WindowInjection à vérifier sur le résultat réel.
  Script de collecte dumpbin préparé (headers, imports normaux/différés), sans
  exécution du payload ; pas de preuve de fermeture DLL. Le workflow est limité
  à `KomiRemote-bundle-x64-unsigned-NOT-STANDALONE.zip`, avertissement inclus.
- Cycles **RED→GREEN observés** avant corrections : support SHA512/NASM manquant,
  payload incomplet accepté, runner non renommé, validateurs INF/CMake absents,
  contrat ZIP non autonome et dossier ZIP non isolé. Les tests n'ont pas été
  présentés comme un démarrage Windows ou une attestation de sécurité.

### Vérifications relancées réellement

- `PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest discover -s tests -p 'test_komiremote*.py' -v` :
  **18 tests réussis**, fixtures de données uniquement ; le snippet de création
  ZIP du workflow est réellement exécuté et ses noms internes contrôlés.
- `actionlint 1.7.7 -shellcheck='' .github/workflows/komiremote-windows-test.yml` :
  succès sans diagnostic ; ShellCheck non exécuté.
- YAML `BaseLoader`, **11 blocs bash** vérifiés par `bash -n`, **5 snippets Python**
  et helpers Python vérifiés par AST. Le script PowerShell n'a pas été exécuté
  ou parsé par PowerShell (absent ici) ; seules assertions source statiques.
- `rustc 1.75.0 --edition=2021 --test tests/komiremote_defaults.rs` : **3 tests réussis** ;
  `rustfmt --check` des deux sources de defaults et `git diff --check` : succès.
  Outils scratch existants utilisés, aucun profil/toolchain global modifié.
- Aucun build Cargo/Flutter/Windows, client/driver/installateur exécuté, dispatch,
  push/merge/release, accès aux secrets, VPS ou Proxmox. Les deux assets sans
  digest n'ont pas été téléchargés pour créer un faux feu vert.

### Blocages de sortie

Avant CI : décision humaine B1, autorisation distincte, ref exacte revue et examen
séparé des triggers upstream. Sur Windows : preuve NASM/PATH/v142, builds effectifs,
validation des layouts natifs approuvés et du script dumpbin. Avant autonomie ou
distribution : stratégie CRT/licences revue et fermeture/chargement DLL, VM propre
sans VS, privacy/pilotes, profil isolé puis coexistence RustDesk, consentement,
services, réseau/chiffrement et signatures. **Aucun portable autonome validé.**
Les sections suivantes sont l'historique de la première tranche, pas des preuves
supplémentaires de cette correction CI.

## Exécuté réellement sur Linux

- Lecture complète du cahier des charges, LISEZMOI du pack et audit préalable.
  Étude des tests Rust existants de `src/lang.rs` et du test Flutter
  `flutter/test/server_settings_dialog_test.dart` avant les changements.
- Cycle RED→GREEN : test Rust du module initialement vide échouant sur
  `RustDesk != KomiRemote`, puis passage ; test de préservation des defaults
  échouant sur relay écrasé, puis passage après utilisation d'`entry().or_insert_with`.
- **3 tests Rust comportementaux réussis**, compilés avec rustc **1.75.0** :
  défauts exacts, conservation des entrées existantes, appels répétés/concurrents
  sans réinitialisation des choix ultérieurs (y compris langue vide et thème clair).
  Ils exécutent le même module `src/komi_config.rs` que le client, sans mock,
  sans compilation du reste de RustDesk ni accès disque/réseau par le module.
- **5 tests Python stdlib réussis**, gardes statiques (pas des tests runtime) :
  ordre core/FFI/loader, priorité des options dans le sous-module épinglé,
  provenance SHA-256 des 15 ressources, métadonnées/noms internes, attribution
  et lien source À propos. Les gardes nouveaux ont échoué avant intégration
  (hooks absents, métadonnées upstream, ressources absentes/différentes), puis passé.
  Le garde de priorité des préférences est une propriété upstream déjà existante.
- Décodage Pillow des quatre ICO intégrés, **sept tailles chacun** :
  16, 24, 32, 48, 64, 128 et 256 px ; PNG vérifiés (logos 1600×311,
  icône runtime 32×32). Table de provenance vérifiée sur les fichiers originaux.
- Inspection visuelle d'une planche synthétique des PNG originaux 16/24/32 px sur
  fonds clair/sombre, également agrandis ×3 sans interpolation. Forme terminal et
  badge vert visibles ; le détail intérieur est peu lisible à 16 px, mieux à
  24/32 px. **Ce n'est pas une capture Windows**, ni une validation du tray réel.
- `git diff --check`, contrôles de périmètre et scan local heuristique des lignes
  ajoutées : vérifications finales avant commit. Aucun secret connu introduit ;
  la clé serveur documentée est publique. Un scan heuristique ne démontre pas
  l'absence exhaustive de vulnérabilités.

## Commandes reproductibles depuis la racine

Python 3.11+ (`tomllib`), sous-module initialisé ; rustc 1.75.0 ou compatible :

```bash
python -m unittest discover -s tests -p test_komiremote.py -v
# Choisir un dossier temporaire utilisateur autorisé, hors dépôt.
rustc --edition=2021 --test tests/komiremote_defaults.rs -o "$TMPDIR/komiremote-defaults-test"
"$TMPDIR/komiremote-defaults-test"
rustfmt --edition 2021 --check src/komi_config.rs tests/komiremote_defaults.rs
git diff --check
```

Le toolchain utilisé ici réside hors dépôt dans
`/home/hermes/.hermes/cache/scratch/komiremote-toolchain/` (aucun PATH/profil global
modifié). Les PNG Flutter sont ignorés par `.gitignore` upstream et sont donc
ajoutés explicitement au commit ; la règle upstream n'a pas été changée.

## Tentative bloquée et non exécuté

`cargo test --locked --offline --lib komi` a réellement été tenté et a échoué
**avant compilation** : checkout Git `tungstenite` au SHA
`efce47e5364f00a63302cf07ab1fb13ded478914` absent du cache et mode hors ligne.
Ce n'est ni un test RustDesk réussi ni une erreur de compilation démontrée du patch.
Pas de téléchargement de toutes les dépendances natives pour simuler un build Windows.

Non exécutés : suite complète Rust, analyse/tests Flutter, application réelle,
compilation MSVC, EXE, portable, MSI, signature, installation/désinstallation,
service/tray après reboot, profils persistés réels, sessions réseau/chiffrement,
compatibilité serveur, consentement, pilotes et fonctions de bureau distant.
Aucune adresse distante n'a été sondée. Aucun mot de passe permanent ajouté.
La suite statique confirme le code des priorités existantes ; elle ne remplace pas
un test de sauvegarde/relecture sur profil Windows réel.

## Prochain jalon, sans publication automatique

Runner Windows isolé autorisé : build de référence puis fork, profil vierge et
profil Komi existant (fr/dark, langue système, autre langue, thème clair/système,
serveur personnalisé), coexistence avec RustDesk, GUI/tray/service cohérents,
puis validation réseau autorisée et revue de packaging/juridique. Aucun onboarding
n'est ajouté à cette tranche ; les garanties de consentement upstream ne sont pas
étendues par ces tests. Revue indépendante souhaitée avant intégration/push.
