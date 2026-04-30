# ------------------------------------------------------------------------------
# Variables configurables
# ------------------------------------------------------------------------------

BINARY      := taskforge
CONFIG      := config/tasks.toml
LOGS_DIR    := ./logs
CARGO       := cargo
RELEASE_BIN := ./target/release/$(BINARY)
DEBUG_BIN   := ./target/debug/$(BINARY)

# Couleurs terminal
BOLD  := \033[1m
GREEN := \033[0;32m
CYAN  := \033[0;36m
RESET := \033[0m

# ------------------------------------------------------------------------------
# Cibles par défaut
# ------------------------------------------------------------------------------

.DEFAULT_GOAL := help
.PHONY: all build dev run run-release run-daemon-release \
        test test-unit test-integration test-verbose test-module \
        cli-list cli-status cli-run cli-enable cli-disable \
        fmt lint check fix clean setup logs-watch help

all: build

# ------------------------------------------------------------------------------
# Compilation
# ------------------------------------------------------------------------------

## Compiler en mode release (optimisé, pour production)
build:
	@echo "$(BOLD)$(GREEN)» Compilation release...$(RESET)"
	$(CARGO) build --release
	@echo "$(GREEN)✓ Binaire disponible : $(RELEASE_BIN)$(RESET)"

## Compiler en mode debug (rapide, pour développement)
dev:
	@echo "$(BOLD)$(CYAN)» Compilation debug...$(RESET)"
	$(CARGO) build
	@echo "$(CYAN)✓ Binaire disponible : $(DEBUG_BIN)$(RESET)"

## Vérification rapide sans produire de binaire
check:
	$(CARGO) check

# ------------------------------------------------------------------------------
# Lancement du daemon
# ------------------------------------------------------------------------------

## Lancer le daemon en mode debug (config par défaut : config/tasks.toml)
run: dev setup
	@echo "$(BOLD)$(CYAN)» Lancement du daemon TaskForge (debug)...$(RESET)"
	@echo "   Config  : $(CONFIG)"
	@echo "   Logs    : $(LOGS_DIR)"
	@echo "   Ctrl+C pour arrêter\n"
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(CARGO) run -- --config $(CONFIG) run-daemon

## Lancer le daemon en mode release
run-release: build setup
	@echo "$(BOLD)$(GREEN)» Lancement du daemon TaskForge (release)...$(RESET)"
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(RELEASE_BIN) --config $(CONFIG) run-daemon

## Lancer avec un fichier de config alternatif
##   Usage : make run-with CONFIG=/etc/taskforge/prod.toml
run-with: dev setup
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(CARGO) run -- --config $(CONFIG) run-daemon

# ------------------------------------------------------------------------------
# Commandes CLI
# (à utiliser dans un second terminal pendant que le daemon tourne)
# ------------------------------------------------------------------------------

## Lister toutes les tâches avec leur prochaine occurrence
cli-list: dev
	@echo "$(CYAN)» Tâches enregistrées :$(RESET)"
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(DEBUG_BIN) --config $(CONFIG) list

## Afficher le rapport de santé (taux de succès + dernière exécution)
cli-status: dev
	@echo "$(CYAN)» Rapport de santé :$(RESET)"
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(DEBUG_BIN) --config $(CONFIG) status

## Forcer l'exécution immédiate d'une tâche
##   Usage : make cli-run TASK=uptime-check
cli-run: dev
	@test -n "$(TASK)" || (echo "Usage: make cli-run TASK=<id>"; exit 1)
	@echo "$(CYAN)» Exécution forcée de '$(TASK)'...$(RESET)"
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(DEBUG_BIN) --config $(CONFIG) run $(TASK)

## Activer une tâche
##   Usage : make cli-enable TASK=rapport-charge
cli-enable: dev
	@test -n "$(TASK)" || (echo "Usage: make cli-enable TASK=<id>"; exit 1)
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(DEBUG_BIN) --config $(CONFIG) enable $(TASK)

## Désactiver une tâche
##   Usage : make cli-disable TASK=rapport-charge
cli-disable: dev
	@test -n "$(TASK)" || (echo "Usage: make cli-disable TASK=<id>"; exit 1)
	TASKFORGE_LOGS_DIR=$(LOGS_DIR) $(DEBUG_BIN) --config $(CONFIG) disable $(TASK)

# ------------------------------------------------------------------------------
# Tests
# ------------------------------------------------------------------------------

## Lancer tous les tests (unitaires + intégration)
test:
	@echo "$(BOLD)» Tous les tests...$(RESET)"
	$(CARGO) test

## Tests unitaires uniquement (rapides, sans I/O disque ni shell)
test-unit:
	@echo "$(BOLD)» Tests unitaires...$(RESET)"
	$(CARGO) test --lib

## Tests d'intégration uniquement (exécutent des commandes shell réelles)
test-integration:
	@echo "$(BOLD)» Tests d'intégration...$(RESET)"
	$(CARGO) test --test integration_tests

## Tests avec output complet (stdout/stderr visible pour chaque test)
test-verbose:
	@echo "$(BOLD)» Tests verbose...$(RESET)"
	$(CARGO) test -- --nocapture

## Tester un module ou un test spécifique par nom
##   Usage : make test-module MOD=retry
##           make test-module MOD=cli
##           make test-module MOD=test_backoff_doubles
test-module:
	@test -n "$(MOD)" || (echo "Usage: make test-module MOD=<nom>"; exit 1)
	$(CARGO) test $(MOD)

# ------------------------------------------------------------------------------
# Qualité de code
# ------------------------------------------------------------------------------

## Formater tout le code source
fmt:
	@echo "» Formatage du code..."
	$(CARGO) fmt
	@echo "✓ Formatage terminé"

## Linter — erreurs de style et patterns suspects
lint:
	@echo "» Analyse statique (clippy)..."
	$(CARGO) clippy -- -D warnings

## Corriger automatiquement les imports inutilisés et warnings simples
fix:
	$(CARGO) fix --lib -p $(BINARY) --allow-dirty
	$(CARGO) fix --bin "$(BINARY)" -p $(BINARY) --allow-dirty

# ------------------------------------------------------------------------------
# Utilitaires
# ------------------------------------------------------------------------------

## Créer les répertoires nécessaires (config + logs)
setup:
	@mkdir -p $(LOGS_DIR)
	@mkdir -p config
	@if [ ! -f $(CONFIG) ]; then \
		echo "$(CYAN)» Aucune config trouvée — création d'un exemple dans $(CONFIG)$(RESET)"; \
		printf '[[task]]\nid = "uptime-check"\nname = "Vérification uptime"\ncommand = "uptime"\nschedule = "@every 1m"\ntimeout_seconds = 5\nmax_retries = 0\nenabled = true\n\n[[task]]\nid = "date-log"\nname = "Log date courante"\ncommand = "date"\nschedule = "@every 1m"\ntimeout_seconds = 5\nmax_retries = 0\nenabled = true\n' > $(CONFIG); \
		echo "$(GREEN)✓ Config exemple créée : $(CONFIG)$(RESET)"; \
	fi

## Observer les logs JSONL en temps réel
logs-watch:
	@echo "$(CYAN)» Suivi des logs : $(LOGS_DIR)/taskforge.jsonl$(RESET)"
	@echo "   (Ctrl+C pour arrêter)\n"
	@if command -v jq > /dev/null 2>&1; then \
		tail -f $(LOGS_DIR)/taskforge.jsonl | jq '{ tache: .task_id, succes: .success, exit: .exit_code, debut: .start_time }'; \
	else \
		tail -f $(LOGS_DIR)/taskforge.jsonl; \
	fi

## Supprimer les artefacts de compilation et les logs
clean:
	@echo "» Nettoyage..."
	$(CARGO) clean
	rm -rf $(LOGS_DIR)
	@echo "✓ Nettoyage terminé"

## Supprimer uniquement les logs (conserve les binaires)
clean-logs:
	rm -rf $(LOGS_DIR)
	@echo "✓ Logs supprimés"

# ------------------------------------------------------------------------------
# Aide
# ------------------------------------------------------------------------------

## Afficher cette aide
help:
	@echo ""
	@echo "$(BOLD)TaskForge — Planificateur de tâches système$(RESET)"
	@echo ""
	@echo "$(BOLD)COMPILATION$(RESET)"
	@echo "  make build              Compiler en mode release (production)"
	@echo "  make dev                Compiler en mode debug (développement)"
	@echo "  make check              Vérification rapide sans binaire"
	@echo ""
	@echo "$(BOLD)DAEMON$(RESET)"
	@echo "  make run                Lancer le daemon en mode debug"
	@echo "  make run-release        Lancer le daemon en mode release"
	@echo "  make run-with CONFIG=.. Lancer avec un fichier de config alternatif"
	@echo ""
	@echo "$(BOLD)CLI (second terminal pendant que le daemon tourne)$(RESET)"
	@echo "  make cli-list           Lister toutes les tâches + prochaine occurrence"
	@echo "  make cli-status         Rapport de santé (taux de succès, dernière exécution)"
	@echo "  make cli-run TASK=<id>  Forcer l'exécution immédiate d'une tâche"
	@echo "  make cli-enable  TASK=  Activer une tâche"
	@echo "  make cli-disable TASK=  Désactiver une tâche"
	@echo ""
	@echo "$(BOLD)TESTS$(RESET)"
	@echo "  make test               Tous les tests"
	@echo "  make test-unit          Tests unitaires uniquement (rapides)"
	@echo "  make test-integration   Tests d'intégration uniquement"
	@echo "  make test-verbose       Tests avec output complet"
	@echo "  make test-module MOD=   Tester un module ou un test par nom"
	@echo ""
	@echo "$(BOLD)QUALITÉ$(RESET)"
	@echo "  make fmt                Formater le code source"
	@echo "  make lint               Analyse statique (clippy)"
	@echo "  make fix                Corriger les warnings automatiquement"
	@echo ""
	@echo "$(BOLD)UTILITAIRES$(RESET)"
	@echo "  make setup              Créer les répertoires et config exemple"
	@echo "  make logs-watch         Suivre les logs en temps réel (jq si disponible)"
	@echo "  make clean              Supprimer artefacts + logs"
	@echo "  make clean-logs         Supprimer uniquement les logs"
	@echo ""
	@echo "$(BOLD)VARIABLES$(RESET)"
	@echo "  CONFIG=<path>           Fichier de config TOML  (défaut: config/tasks.toml)"
	@echo "  LOGS_DIR=<path>         Répertoire des logs     (défaut: ./logs)"
	@echo "  TASK=<id>               ID de tâche pour cli-run/enable/disable"
	@echo "  MOD=<nom>               Filtre pour test-module"
	@echo ""
	@echo "$(BOLD)EXEMPLE DE SESSION$(RESET)"
	@echo "  $$ make run                          # Terminal 1 — daemon"
	@echo "  $$ make cli-list                     # Terminal 2 — voir les tâches"
	@echo "  $$ make cli-run TASK=uptime-check    # Terminal 2 — exécution forcée"
	@echo "  $$ make logs-watch                   # Terminal 2 — observer les logs"
	@echo "  $$ make cli-status                   # Terminal 2 — rapport de santé"
	@echo ""