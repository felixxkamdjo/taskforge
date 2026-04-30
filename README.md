# TaskForge

> Planificateur de tâches système écrit en Rust, conçu pour serveurs Linux.

TaskForge est un démon de planification inspiré de `cron`, enrichi de fonctionnalités modernes :
retry avec backoff exponentiel, persistance de l'historique d'exécution, rapport de santé par tâche,
et une CLI complète pour interagir avec le planificateur en temps réel.

---

## Table des matières

- [Fonctionnalités](#fonctionnalités)
- [Prérequis](#prérequis)
- [Installation](#installation)
- [Configuration](#configuration)
- [Utilisation](#utilisation)
- [Simulation sur poste de développement](#simulation-sur-poste-de-développement)
- [Variables d'environnement](#variables-denvironnement)
- [Architecture](#architecture)
- [Tests](#tests)
- [Commandes Make](#commandes-make)

---

## Fonctionnalités

- **Expressions cron complètes** — syntaxe `* * * * *` avec support des plages, listes, steps
- **Macros de planification** — `@daily`, `@hourly`, `@weekly`, `@monthly`, `@yearly`, `@every Nm/Nh`
- **Exécution concurrente** — chaque tâche due tourne dans sa propre tâche Tokio
- **Retry exponentiel** — backoff `5s → 10s → 20s … → 60s` plafonné, configurable par tâche
- **Timeout par tâche** — processus tué (SIGKILL) si dépassement, tracé dans les logs
- **Persistance JSONL** — chaque exécution est enregistrée avec timestamps, exit code, stdout/stderr
- **Rapport de santé** — taux de succès et dernière exécution par tâche, consultable à tout moment
- **CLI interactive** — list, status, enable, disable, run — sans redémarrer le daemon
- **Activation/désactivation à chaud** — via `taskforge enable/disable <id>`

---

## Prérequis

| Outil | Version minimale | Vérification |
|-------|-----------------|--------------|
| Rust  | 1.75            | `rustc --version` |
| Cargo | 1.75            | `cargo --version` |
| Make  | 3.81            | `make --version` |
| Linux | kernel 4.x+     | `uname -r` |

> TaskForge utilise `sh -c` pour exécuter les commandes. Tout shell POSIX est supporté.

---

## Installation

```bash
# 1. Cloner le dépôt
git clone https://github.com/youruser/taskforge
cd taskforge

# 2. Compiler en mode release
make build

# 3. Vérifier que le binaire est disponible
./target/release/taskforge --version
```

---

## Configuration

Les tâches sont définies dans un fichier TOML. Par défaut, TaskForge cherche `config/tasks.toml`.

```bash
mkdir -p config
```

### Exemple complet — `config/tasks.toml`

```toml
# Sauvegarde base de données — tous les jours à 2h du matin
[[task]]
id = "backup-bdd"
name = "Sauvegarde base de données"
command = "pg_dump mydb > /backup/db_$(date +%Y%m%d).sql"
schedule = "0 2 * * *"
timeout_seconds = 120
max_retries = 3
enabled = true

# Nettoyage fichiers temporaires — chaque nuit à minuit
[[task]]
id = "cleanup-tmp"
name = "Nettoyage fichiers temporaires"
command = "find /tmp -mtime +7 -delete"
schedule = "@daily"
timeout_seconds = 30
max_retries = 0
enabled = true

# Vérification uptime — toutes les minutes
[[task]]
id = "uptime-check"
name = "Vérification uptime"
command = "uptime >> /var/log/uptime.log"
schedule = "@every 1m"
timeout_seconds = 5
max_retries = 0
enabled = true

# Rapport de charge — toutes les 30 minutes
[[task]]
id = "rapport-charge"
name = "Rapport de charge système"
command = "top -bn1 | head -5 >> /var/log/charge.log"
schedule = "*/30 * * * *"
timeout_seconds = 10
max_retries = 1
enabled = true

# Archivage logs — tous les dimanches à 23h
[[task]]
id = "archivage-logs"
name = "Archivage des logs"
command = "tar -czf /archive/logs_$(date +%Y%W).tar.gz /var/log/app/"
schedule = "0 23 * * 0"
timeout_seconds = 60
max_retries = 2
enabled = true

# Tâche désactivée — ne sera jamais exécutée
[[task]]
id = "maintenance"
name = "Mode maintenance"
command = "echo 'maintenance en cours'"
schedule = "@every 5m"
timeout_seconds = 10
max_retries = 0
enabled = false
```

### Champs disponibles

| Champ | Type | Défaut | Description |
|-------|------|--------|-------------|
| `id` | string | — | Identifiant unique de la tâche **(obligatoire)** |
| `name` | string | — | Nom lisible **(obligatoire)** |
| `command` | string | — | Commande shell à exécuter **(obligatoire)** |
| `schedule` | string | — | Expression cron ou macro **(obligatoire)** |
| `timeout_seconds` | u32 | `60` | Durée max avant SIGKILL |
| `max_retries` | u32 | `0` | Nombre de retries en cas d'échec |
| `enabled` | bool | `true` | Active ou désactive la tâche |

### Syntaxe de planification

#### Expressions cron — `minute heure jour mois jour_semaine`

```
┌───────────── minute        (0–59)
│ ┌─────────── heure         (0–23)
│ │ ┌───────── jour du mois  (1–31)
│ │ │ ┌─────── mois          (1–12)
│ │ │ │ ┌───── jour semaine  (0–7, 0 et 7 = dimanche)
│ │ │ │ │
* * * * *
```

| Exemple | Description |
|---------|-------------|
| `* * * * *` | Chaque minute |
| `0 8 * * *` | Chaque jour à 8h00 |
| `30 8 * * 1` | Chaque lundi à 8h30 |
| `0 0 1 * *` | 1er de chaque mois à minuit |
| `*/15 * * * *` | Toutes les 15 minutes |
| `0 9-18 * * 1-5` | Toutes les heures de 9h à 18h, du lundi au vendredi |
| `0 8,12,18 * * *` | À 8h, 12h et 18h chaque jour |

#### Macros

| Macro | Équivalent cron | Description |
|-------|----------------|-------------|
| `@hourly` | `0 * * * *` | Début de chaque heure |
| `@daily` / `@midnight` | `0 0 * * *` | Chaque jour à minuit |
| `@weekly` | `0 0 * * 1` | Chaque lundi à minuit |
| `@monthly` | `0 0 1 * *` | 1er du mois à minuit |
| `@yearly` / `@annually` | `0 0 1 1 *` | 1er janvier à minuit |
| `@every 5m` | — | Toutes les 5 minutes |
| `@every 2h` | — | Toutes les 2 heures |

---

## Utilisation

### Mode daemon (planificateur actif)

```bash
# Démarrage standard
make run

# Avec un fichier de config personnalisé
TASKFORGE_LOGS_DIR=./logs ./target/debug/taskforge --config /etc/taskforge/prod.toml run-daemon
```

Au démarrage, le daemon affiche :

```
╔══════════════════════════════════════════╗
║         TaskForge — Daemon actif         ║
╚══════════════════════════════════════════╝
 8 tâche(s) chargée(s) — 7 active(s)

 Prochaines exécutions :
   uptime-check         → 2026-04-30 07:31:00 UTC
   rapport-charge       → 2026-04-30 08:00:00 UTC
   backup-bdd           → 2026-05-01 02:00:00 UTC
   ...

[taskforge] Superviseur démarré — tick toutes les 60s
```

Puis, à chaque exécution de tâche :

```
[07:31:00] ✓ 'uptime-check' (exit: Some(0))
[08:00:00] ✓ 'rapport-charge' (exit: Some(0))
[08:00:01] ✗ 'backup-bdd' (exit: Some(1))
```

### Commandes CLI

> Ces commandes s'utilisent **dans un second terminal** pendant que le daemon tourne.

```bash
# Lister toutes les tâches avec leur prochaine occurrence
taskforge list

# Rapport de santé (taux de succès + dernière exécution)
taskforge status

# Forcer l'exécution immédiate d'une tâche (bloquant, avec output)
taskforge run uptime-check

# Désactiver une tâche sans redémarrer le daemon
taskforge disable rapport-charge

# Réactiver une tâche
taskforge enable rapport-charge

# Utiliser un fichier de config alternatif
taskforge --config /etc/taskforge/prod.toml list
```

#### Exemple de sortie — `taskforge list`

```
ID                   | Nom                       | Planification          | Prochaine exécution            | État
---------------------+---------------------------+------------------------+--------------------------------+---------
backup-bdd           | Sauvegarde base de données| 0 2 * * *              | 2026-05-01 02:00:00 UTC        | ✓ actif
cleanup-tmp          | Nettoyage fichiers temp.  | @daily                 | 2026-05-01 00:00:00 UTC        | ✓ actif
uptime-check         | Vérification uptime       | @every 1m              | 2026-04-30 07:32:00 UTC        | ✓ actif
maintenance          | Mode maintenance          | @every 5m              | —                              | ✗ inactif

8 tâche(s) — 7 active(s)
```

#### Exemple de sortie — `taskforge status`

```
=== RAPPORT DE SANTÉ TASKFORGE ===
ID Tâche        | État       | Succès (%) | Dernière Exécution
----------------+------------+------------+-------------------------
backup-bdd      | Actif      |      100.0 | 2026-04-30 02:00:01 (success)
uptime-check    | Actif      |       95.0 | 2026-04-30 07:31:00 (success)
rapport-charge  | Désactivé  |        0.0 | Jamais exécutée

Total tâches actives : 7 / 8
```

#### Exemple de sortie — `taskforge run uptime-check`

```
[taskforge] Exécution forcée de 'uptime-check' : uptime >> /var/log/uptime.log
[taskforge] ✓ succès (exit: Some(0))

[historique] 'uptime-check' — 42 exécution(s) | taux de succès : 98%
```

---

## Simulation sur poste de développement

Ton poste Linux est un serveur à part entière pour TaskForge. Voici comment simuler
un environnement de production en quelques minutes.

### Étape 1 — Préparer une config de simulation

```bash
mkdir -p config logs

cat > config/tasks.toml << 'EOF'
[[task]]
id = "uptime-check"
name = "Vérification uptime"
command = "uptime"
schedule = "@every 1m"
timeout_seconds = 5
max_retries = 0
enabled = true

[[task]]
id = "date-log"
name = "Log date courante"
command = "date"
schedule = "@every 1m"
timeout_seconds = 5
max_retries = 0
enabled = true

[[task]]
id = "disk-check"
name = "Vérification disque"
command = "df -h /"
schedule = "@every 2m"
timeout_seconds = 5
max_retries = 1
enabled = true

[[task]]
id = "failing-task"
name = "Tâche qui échoue"
command = "exit 1"
schedule = "@every 1m"
timeout_seconds = 5
max_retries = 2
enabled = true
EOF
```

### Étape 2 — Ouvrir deux terminaux

**Terminal 1 — le daemon**

```bash
make run
# ou directement :
# TASKFORGE_LOGS_DIR=./logs cargo run -- --config config/tasks.toml run-daemon
```

**Terminal 2 — interaction CLI**

```bash
export TASKFORGE_LOGS_DIR=./logs

# Voir l'état initial
./target/debug/taskforge --config config/tasks.toml list

# Après 1-2 minutes, consulter le rapport de santé
./target/debug/taskforge --config config/tasks.toml status

# Forcer une exécution immédiate
./target/debug/taskforge --config config/tasks.toml run disk-check

# Désactiver la tâche qui échoue
./target/debug/taskforge --config config/tasks.toml disable failing-task

# Observer que le rapport de santé se met à jour
./target/debug/taskforge --config config/tasks.toml status

# Réactiver
./target/debug/taskforge --config config/tasks.toml enable failing-task
```

### Étape 3 — Observer les logs JSONL

```bash
# Suivre les exécutions en temps réel
tail -f logs/taskforge.jsonl

# Avec jq pour une lecture formatée (si installé)
tail -f logs/taskforge.jsonl | jq '{ tache: .task_id, succes: .success, exit: .exit_code, debut: .start_time }'
```

Chaque entrée ressemble à :

```json
{
  "task_id": "uptime-check",
  "start_time": "2026-04-30T07:31:00Z",
  "end_time": "2026-04-30T07:31:00.123Z",
  "exit_code": 0,
  "stdout": " 7:31:00 up 3 days,  2:15,  1 user,  load average: 0.12, 0.08, 0.05\n",
  "stderr": "",
  "success": true
}
```

---

## Variables d'environnement

| Variable | Défaut | Description |
|----------|--------|-------------|
| `TASKFORGE_LOGS_DIR` | `./logs` | Répertoire de persistance de l'historique JSONL |

```bash
# Exemple pour pointer vers /var/log/taskforge en production
export TASKFORGE_LOGS_DIR=/var/log/taskforge
make run-release
```

---

## Architecture

```
taskforge/
├── src/
│   ├── main.rs                  # Point d'entrée — CLI + lancement daemon
│   ├── types.rs                 # Types partagés : Task, ExecutionRecord, Schedule, CronField
│   │
│   ├── parser/                  # P1 — Parsing des planifications
│   │   ├── expression.rs        #   Expressions cron 5 champs
│   │   └── macros.rs            #   Macros @daily, @every…
│   │
│   ├── registry.rs              # P2 — Registre des tâches en mémoire
│   │
│   ├── engine/                  # P3 — Moteur d'exécution
│   │   ├── executor.rs          #   Exécution shell + timeout + capture I/O
│   │   ├── retry.rs             #   Politique de retry (backoff exponentiel)
│   │   └── supervisor.rs        #   Boucle principale, dispatch concurrent Tokio
│   │
│   ├── schedule.rs              # P4 — Calcul de la prochaine occurrence
│   │
│   ├── history/                 # P4 — Persistance et requêtes historique
│   │   ├── store.rs             #   Écriture JSONL (save_record)
│   │   └── query.rs             #   Lecture : last_execution, success_rate, total_executions
│   │
│   └── cli/                     # P5 — Interface en ligne de commande
│       ├── commands.rs          #   Sous-commandes clap
│       └── health.rs            #   Rapport de santé tabulaire
│
├── tests/
│   └── integration_tests.rs     # Tests d'intégration P1→P5
│
├── config/
│   └── tasks.toml               # Configuration des tâches (à créer)
│
├── logs/                        # Historique JSONL (créé automatiquement)
│
├── Makefile
└── README.md
```

### Flux de données

```
config/tasks.toml
       │
       ▼ load_config() [P1]
  Vec<Task>
       │
       ▼ TaskRegistry::from_tasks() [P2]
  TaskRegistry (Arc<Mutex<_>>)
       │
       ▼ due_tasks() toutes les 60s [P2+P4]
  Vec<Task> dues
       │
       ▼ execute_with_retry() [P3]
  ExecutionRecord
       │
       ├──▶ mpsc::Sender  ──▶  save_record() [P4-store]  ──▶  logs/taskforge.jsonl
       │
       └──▶ last_execution() / success_rate() [P4-query]  ──▶  taskforge status [P5]
```

---

## Tests

```bash
# Tous les tests (unitaires + intégration)
make test

# Tests unitaires uniquement (rapides, sans I/O)
make test-unit

# Tests d'intégration uniquement (exécutent des commandes shell réelles)
make test-integration

# Avec output complet (voir stdout/stderr de chaque test)
make test-verbose

# Un module spécifique
cargo test cli
cargo test retry
cargo test registry
cargo test schedule
```

### Couverture des tests

| Module | Tests unitaires | Tests d'intégration |
|--------|----------------|---------------------|
| P1 — parser | ✓ expression, macros, erreurs | ✓ config → registry |
| P2 — registry | ✓ CRUD, due_tasks, upcoming | ✓ config → due → exécution |
| P3 — engine | ✓ succès, échec, timeout, retry | ✓ pipeline complet via canal |
| P4 — history | ✓ store, query, rotation | ✓ persistance + taux de succès |
| P5 — cli | ✓ parsing clap, dispatch | ✓ config → registry → CLI ops |

---

## Commandes Make

```bash
make help    # liste toutes les cibles disponibles
```

---

## Licence

MIT