# Validation de la tranche

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
