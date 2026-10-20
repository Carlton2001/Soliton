# RAP - Rust Audio Player

## Présentation
RAP est un lecteur de musique haute performance écrit en **Rust**, conçu pour gérer de grandes collections de fichiers audio (FLAC, MP3, WAV, etc) avec une interface fluide et réactive. Le projet met l'accent sur la richesse des métadonnées et la rapidité de navigation via une base de données locale.

## Stack Technique

*   **Langage :** Rust
*   **Interface Graphique (GUI) :** [GPUI](https://github.com/zed-industries/zed), le framework UI de Zed. Version envisagée : `0.2.2`, à valider lors de l'initialisation.
*   **Moteur Audio :** Codecs via `symphonia` et sortie via `cpal`.
*   **Base de Données :** `SQLite` (via `rusqlite`) pour la persistance de la bibliothèque.
*   **Gestion des Métadonnées :** `lofty` pour l'extraction de tags ID3/Vorbis et des pochettes d'albums.

Les versions exactes restent à valider. Une fois le workspace initialisé, les manifestes `Cargo.toml` font référence pour les dépendances déclarées.

## Références

- [Spécifications de RAP](RAP_SPECIFICATIONS.md) : périmètre fonctionnel, contraintes et phases de développement. Ce fichier `AGENTS.md` porte les consignes de travail ; éviter de dupliquer les spécifications. Signaler les contradictions avant d'implémenter le comportement concerné.
- Lire intégralement les spécifications avant l'initialisation du projet, puis consulter les sections pertinentes pour chaque changement. Présenter les choix ayant un impact important sur l'architecture avant implémentation.
- **Référence GPUI :** examiner le projet voisin `../Oxyde-GPUI` pour les composants, conventions visuelles et usages GPUI pertinents, notamment les closures `cx.listener`. Vérifier leur compatibilité avec la version retenue dans RAP. Réutiliser sélectivement après examen du code, en respectant licences et attributions, sans dépendance directe à l'application Oxyde.

## Règles permanentes

- Garder `cargo check` sans avertissement du projet. Utiliser `rg`, jamais `grep`.
- Garder les calculs coûteux et les opérations de fichiers hors du thread UI.
- Isoler les intégrations propres au système derrière des interfaces dédiées, avec implémentations et dépendances conditionnées par cible. Ne pas exposer de types Windows dans le code commun.
- Privilégier les interactions natives et le clic gauche pour les actions principales.
- Utiliser les SVG Lucide officiels de `lucide-icons/lucide/icons/<nom>.svg`, sans modifier leur tracé. Si la source est inaccessible, signaler le blocage.
- Ne créer ni modifier un changement de curseur sans demande explicite de l’utilisateur. Les nouvelles interactions gardent le curseur par défaut.
- Écrire les messages de commit en anglais. **Chaque push vers GitHub exige une demande explicite pour cette opération**, y compris branches et tags, quel que soit l’outil. Une demande de modification, commit ou fusion ne l’autorise pas ; une autorisation antérieure ne vaut pas pour le push suivant.
- Prévoir les textes d'interface en français et en anglais dès leur introduction.

## Invariants de RAP

- Ne jamais modifier, déplacer, renommer ou supprimer les fichiers de la bibliothèque musicale, leurs tags ou leurs pochettes intégrées. Stocker bases, caches et fichiers temporaires hors des dossiers sources.
- Une source inaccessible ne constitue pas une preuve de suppression de ses fichiers : conserver ses entrées et signaler leur indisponibilité.
- Garder le moteur audio indépendant de GPUI. Dans le callback audio, éviter allocations, entrées/sorties, journalisation et attentes sur des verrous bloquants.
- Prévoir l'annulation des indexations et une concurrence bornée pour les lectures et traitements coûteux.

## Organisation du code

- Définir les chemins et les frontières des crates lors de l'initialisation. Séparer le domaine, le moteur audio, la persistance, l'indexation et l'interface ; garder les règles métier indépendantes des composants GPUI.
- Une responsabilité identifiable par module. Les façades déclarent et assemblent ; elles n’accumulent pas les parcours et rendus.
- Utiliser 200–600 lignes de production comme repère, sans minimum à atteindre ; au-delà de 800, examiner le découpage avant une extension significative et justifier le maintien. Ne pas découper artificiellement ni compter les tests comme production.
- Extraire les sections cohérentes ; partager un composant seulement si plusieurs usages partagent présentation ou comportement. Préserver visibilité minimale, identifiants GPUI et cycles de vie. Limiter la refactorisation au changement.
- Ranger les nouveaux tests unitaires dans le `tests/` du module, sans changer leur parent logique Rust ni élargir les visibilités. Préserver scénarios, attributs et découverte lors d’un déplacement.

## Validation

- Après modification de Rust, exécuter `cargo fmt --check`, `cargo check` et les tests pertinents. Indiquer les vérifications non exécutées et pourquoi.
- Tester en priorité les comportements critiques : métadonnées, regroupement des albums et éditions, indexation incrémentale, disponibilité des sources et états du moteur audio.
- Windows est la cible de validation initiale. Préserver l'isolation du code spécifique aux plateformes ; ne pas présenter Linux comme validé sans vérification.
