# Handoff dla sesji AI

QA Receiver Center Web: migracja aplikacji Windows (React/Vite + Tauri/Rust + usługa ASP.NET Core Registry) do wersji webowej na serwerze w LAN. Do czasu etapu 1 kod jest jeszcze desktopowy; cel i etapy w `PLAN-BIEZACY.md`. Zasady pracy (w tym budżet kontekstu) są w `../AGENTS.md`. Zacznij od [PLAN-BIEZACY.md](PLAN-BIEZACY.md) i [MAPA-KODU.md](MAPA-KODU.md); wpis w [PULAPKI.md](PULAPKI.md) czytaj tylko dla swojego zadania.

## Szybki start

1. `git status --short --branch`; nie nadpisuj cudzych zmian.
2. Znajdź obszar w mapie (tabela „Zadanie -> gdzie zacząć”), potem `rg -n` na symbol i wycinek pliku.
3. Po zmianie uruchom tylko kontrolę właściwą dla obszaru. Nie ma skryptu npm `docs:check`; dokumenty sprawdza lokalny `powershell -ExecutionPolicy Bypass -File scripts/Check-RepositoryDocs.ps1`.
4. Plan aktualizuj po zmianie zakresu lub statusu; mapę tylko po zmianie architektury lub komendy kontroli.

## Podstawowe kontrole

```powershell
npm.cmd run lint
npm.cmd run build
npm.cmd run test:logic
Set-Location src-tauri; cargo test --lib; cargo check
Set-Location ..; dotnet build services/receiver-registry/QAReceiverRegistry.csproj --no-restore
```

Stan bazowy 2026-10-22: wszystkie powyższe przechodzą (18 testów TS, 75 testów Rust + 2 pominięte, 0 ostrzeżeń .NET). `Run-IntegrationTests.ps1` to kontrola operacyjna Registry (wymaga środowiska), nie zamiennik builda.

## Gdzie szukać

- UI i workflow: `src/App.tsx` (monolit; `FirmwareModulePage` zajmuje ok. 4 tys. linii, patrz mapa)
- Style: `src/styles/NN-*.css` (20 plików) importowane w kolejności przez `src/App.css`; `rg -n "\.klasa" src/styles` i edytuj ostatnie wystąpienie
- Czyste reguły firmware: `src/firmware*.ts` + `tests/`
- IPC i operacje: `src-tauri/src/lib.rs`; orkiestracja: `firmware_orchestrator.rs`; hasła: `receiver_password.rs`
- Katalog release Alpha/Beta: `src/firmwareReleaseSelection.ts`, parser w `lib.rs`
- Członkostwo Alpha/Beta z `qa-receivers.json`; wykluczenia lokalne w `firmware-configs.json`
- Kontrola dostępu przed firmware: `UserSecurity=FALSE` i `SecMode=Off` pomijają logowanie
- IP: `src/IpManagementPage.tsx` + endpointy Registry; Registry: `services/receiver-registry/`

## Środowisko i pamięć

- Vault Obsidian (drugi mózg): `C:\Users\pnorek\Documents\PATRYK_QA`; wejście: `AGENTS.md`, `wiki/hot.md`. Połączenie z Claude Code przez wtyczkę `claude-code-ide` (komenda `/ide`).
- Zdalne repo: `origin` = prywatne GitHub `qa-receiver-center-web`; `desktop` = lokalne repo desktopowe (źródło cherry-pick poprawek).
- Dokumenty AI są wersjonowane; lokalny tylko `.claude/settings.local.json`.

<!-- check-doc: docs/MAPA-KODU.md -->
<!-- check-doc: docs/PULAPKI.md -->
<!-- check-doc: docs/PLAN-BIEZACY.md -->
<!-- check-path: src/App.tsx -->
<!-- check-path: src-tauri/src/lib.rs -->
