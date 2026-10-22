# Mapa kodu

## Krótki opis

QA Receiver Center to wewnętrzna aplikacja Windows do obsługi odbiorników Trimble QA. Interfejs React działa w Tauri, a kod Rust wykonuje operacje na odbiornikach, lokalnym systemie plików i usługach wewnętrznych. Osobna usługa ASP.NET Core udostępnia współdzielone dane przez SQLite i HTTP na porcie 8787.

Stan na 2026-10-22: mapa opisuje kod desktopowy przejęty z repo `qa-receiver-center`. Architektura docelowa (serwer web) jest w `PLAN-BIEZACY.md`; mapę aktualizuj w miarę realizacji etapów 1-5.

## Technologie

- React 19 + TypeScript + Vite
- Tauri 2 + Rust
- ASP.NET Core na .NET 10 Windows + SQLite
- Testy czystej logiki TypeScript przez `node --test`
- Kontrole: ESLint, TypeScript/Vite build, Cargo, `dotnet build`

## Entry pointy

| Warstwa | Zacznij od | Rola |
|---|---|---|
| UI | `src/main.tsx` | Montuje React, motyw Modus i ekran startowy. |
| UI | `src/App.tsx` | Główny ekran i workflow aplikacji. Zawiera kilka spójnych, ale nadal współlokalnych domen UI. |
| Tauri | `src-tauri/src/main.rs` | Cienki launcher wywołujący `app_lib::run()`. |
| Tauri | `src-tauri/src/lib.rs` | Rejestruje komendy IPC i implementuje operacje aplikacji. |
| Orkiestracja | `src-tauri/src/firmware_orchestrator.rs` | Rezerwacje, limity współbieżności i stan zadań firmware. |
| Registry | `services/receiver-registry/Program.cs` | Bootstrap usługi ASP.NET Core i konfiguracja hosta Windows. |
| Registry API | `services/receiver-registry/Api/RegistryEndpoints.cs` | Endpointy HTTP usługi. |
| Registry DB | `services/receiver-registry/Data/RegistryDatabase.cs` | SQLite, migracje i operacje danych. |

## Zadanie -> gdzie zacząć

| Rodzaj zadania | Zacznij od | Następne miejsca tylko jeśli potrzebne |
|---|---|---|
| Zmiana ekranu lub workflow UI | `src/App.tsx` | `src/IpManagementPage.tsx`, komponenty alertów, odpowiadający moduł `src/firmware*.ts` |
| Zmiana wyglądu / CSS | `rg -n "\.klasa" src/styles` | tabela plików w „Przewodnik po dużych plikach”; edytuj ostatnie wystąpienie |
| Formatowanie wartości prezentowanych w UI | `src/displayFormatters.ts` | test `tests/display-formatters.test.ts`, potem call sites w `src/App.tsx` |
| Reguła blokująca start firmware | `src/firmwareStartBlockers.ts` | odpowiadający test w `tests/`, potem call site w `src/App.tsx` |
| Wybór odbiorników lub limity pracy | `src/firmwareSelection.ts`, `src/firmwareConcurrency.ts` | odpowiadające testy w `tests/` |
| Status, skanowanie lub komunikacja z odbiornikiem | komenda w `src-tauri/src/lib.rs` | typy wywołania w `src/App.tsx`, `src-tauri/src/receiver_password.rs` |
| Aktualizacja firmware | `src-tauri/src/firmware_orchestrator.rs` | komendy firmware w `src-tauri/src/lib.rs`, workflow firmware w `src/App.tsx` |
| Lista release Alpha/Beta | `src/firmwareReleaseSelection.ts` | odświeżanie w `src/App.tsx`, parser katalogu i buildów w `src-tauri/src/lib.rs` |
| Hasła po aktualizacji | `src-tauri/src/receiver_password.rs` | `src-tauri/src/password_recovery_notices.rs`, alerty w `src/PasswordRecoveryAlerts.tsx` |
| Dostęp do odbiornika bez Security Mode | `src-tauri/src/receiver_password.rs` | `check_initial_firmware_password` i workflow w `src-tauri/src/lib.rs` |
| Usuwanie z grup firmware Alpha/Beta | `remove_firmware_config_receiver` w `src-tauri/src/lib.rs` | `firmware_receivers_for_group`, lokalny `firmware-configs.json`, modal w `src/App.tsx` |
| Zarządzanie IP | `src/IpManagementPage.tsx` | komendy IPC w `src-tauri/src/lib.rs`, endpointy i `IpPingService.cs` |
| Historia firmware i Clone Files | `src/App.tsx` (widok historii) | `src-tauri/src/lib.rs`, `RegistryEndpoints.cs`, `RegistryDatabase.cs` |
| Współdzielone ustawienia | `src/App.tsx` (ustawienia bezpieczeństwa/monitoringu) | `src-tauri/src/lib.rs`, `RegistryEndpoints.cs` |
| Dane registry lub migracja SQLite | `services/receiver-registry/Data/RegistryDatabase.cs` | `services/receiver-registry/Migrations/`, modele i endpointy |
| Instalacja lub aktualizacja usługi | `services/receiver-registry/scripts/` | `services/receiver-registry/README.md`, `appsettings*.json` |

## Przewodnik po dużych plikach

Numery linii są przybliżone (stan 2026-10-22); potwierdź symbolem: `rg -n "fn nazwa" src-tauri/src/lib.rs`.

**`src-tauri/src/lib.rs` (~8,1 tys.)**

| Linie | Zawartość |
|---|---|
| 1-56 | importy i stałe (limity zadań, adresy usług, stałe postępu) |
| 90-686 | struktury DTO: odbiornik, firmware, job, historia, IP, Registry |
| 687-1330 | komendy IPC: odbiorniki, konfiguracja, katalog release, joby pojedyncze, historia, Clone File |
| 1333-1600 | przywracanie haseł po aktualizacji |
| 1602-2200 | joby grupowe (`start_firmware_update_job`, stop, downgrade, workery) |
| 2224-2345 | `firmware-configs.json` i członkostwo grup |
| 2346-3018 | modele, walidacja nazw, parser katalogu release/buildów (Alpha/Beta) |
| 3019-3830 | plan, dry-run i workflow pojedynczej aktualizacji (`run_single_local_firmware_update_job_inner` ok. 500 linii) |
| 3826-4870 | rozwiązanie odbiorników, preflight, hasło początkowe, upload pakietu, Clone File |
| 4869-5420 | monitoring instalacji na żywo, etapy i procent postępu |
| 5420-5670 | stan jobów i wpisy historii |
| 5673-6620 | klient Registry, IP, ustawienia bezpieczeństwa aktualizacji, cache historii |
| 6621-6920 | odczyt listy odbiorników (ADO) i skan statusu |
| 6921-7148 | parsowanie XML i formatowanie |
| ok. 7149 | `run()` z rejestracją komend (`generate_handler!`); ok. 7217 testy |

**`src/App.tsx` (~6,8 tys.)**

| Linie | Zawartość |
|---|---|
| 74-370 | typy i stałe nawigacji |
| 413-585 | `UpdateReceiverSummary`, `FirmwareDeviceCard` |
| 586-1263 | `App`: nawigacja, dashboard, skan, stan globalny |
| 1264-5334 | `FirmwareModulePage`: Single, Group, Multiple, Alpha, Beta, modale, polling jobów |
| 5335-5682 | `LoginScreen` (`LOGIN_ENABLED=false`), boot, offline, akcje i wiersz odbiornika |
| 5683-6197 | `HistoryPage` |
| 6198-6577 | ustawienia Update Safety i monitoringu IP |
| 6578-6848 | modal danych logowania, kafelki, helpery |

**Style (`src/App.css` + `src/styles/`)**: `src/App.css` zawiera tylko listę `@import` w kolejności kaskady (późniejszy plik nadpisuje wcześniejszy; nie zmieniaj kolejności). Podział z 2026-10-22 jest mechaniczny; zbudowany CSS jest bajt w bajt identyczny z wersją sprzed podziału. Zmieniając styl: `rg -n "\.klasa" src/styles`, edytuj ostatnie wystąpienie.

| Plik w `src/styles/` | Zawartość (nazwa wg dominującej domeny) |
|---|---|
| `01-base-login-navigation` | logowanie, boot, pasek nawigacji |
| `02-controls-buttons` | przyciski i kontrolki |
| `03-dashboard-receivers` | dashboard, karty i akcje odbiorników, status |
| `04-credentials-dialogs` | dane logowania, potwierdzenia usuwania |
| `05-ip-management` | zarządzanie IP, tabela, żądania |
| `06-history` | historia: kalendarz, wpisy, Clone File |
| `07-single-update` | aktualizacja pojedyncza, kroki workflow, dropzone |
| `08-preflight-multiple` | preflight, wybór odbiorników, Multiple |
| `09-responsive-base` | keyframes, media i container queries bazy |
| `10-modus-controls` | kontrolki Modus (dawny blok od ok. 5865) |
| `11-firmware-responsive-rebuild` | przebudowa responsywna Firmware i History |
| `12-receiver-cards-canonical` | kanoniczne karty Single i Group |
| `13-firmware-workspace-final` | „final” workspace firmware (komentarz: trzymaj późno) |
| `14-receiver-card-guardrails` | zabezpieczenia kart, ujednolicony workspace |
| `15-firmware-package-refinements` | pakiety lokalne, karta workspace |
| `16-release-catalog-refinements` | katalog release i karty |
| `17-group-workspace` | Group: kompaktowa lista Modus |
| `18-density-laptop` | gęstość dla laptopów i niskich okien |
| `19-update-safety-settings` | ustawienia Update Safety, monitoring IP |
| `20-final-polish` | końcowe poprawki kart i workspace, hasła |

Rozmiary: 170-1000 linii na plik. `src/index.css` (141 linii) to osobne style globalne.

**`services/receiver-registry`**: `Api/RegistryEndpoints.cs` grupuje trasy: `/health`, `/api/receivers`, `/groups`, `/settings`, `/firmware-history` (+ `clone-file`), IP admin (`/ip-inventory`, `/ip-requests`), IP guest (portal), `/audit`, `/maintenance/backup`. `Data/RegistryDatabase.cs`: odbiorniki 97-260, IP 268-740, grupy 744-850, audyt/ustawienia 851-975, historia firmware i Clone File 979-1400, status/backup 1404+, reszta to migracje i helpery SQLite. Walidacja wejścia: `Infrastructure/RequestValidator.cs`.

## Baza danych Registry (SQLite)

- Plik `receivers.db` na serwerze (`C:\TSQATools\QAReceiverRegistry\data`), tylko usługa go otwiera; klienci używają HTTP :8787. WAL, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout` z konfiguracji.
- Migracje: `Migrations/NNN_*.sql` stosowane rosnąco w transakcji przy starcie (`InitializeAsync`), ewidencja w `_schema_migrations`. Obecnie 001-008 (schemat 8); `005_seed_ip_inventory.sql` to ok. 2,8 tys. linii danych, nie czytaj.
- Tabele: `receivers`, `receiver_groups`, `receiver_group_memberships`, `clone_files`, `audit_log`, `app_settings`, `firmware_update_history`, `ip_inventory`, `ip_requests`, `ip_request_events`.
- Współbieżność: optymistyczna blokada `row_version` (UPDATE ... WHERE row_version = $x, konflikt = błąd walidacji); każda zmiana zapisuje wpis w `audit_log`.
- Clone Files: pliki na dysku w `clone-files`, limit 20 MB, SHA-256; historia firmware ma upsert idempotentny po `jobId`.
- Backup: weryfikowany przy starcie i co 24 h (`BackupWorker`), zachowuje 14 ostatnich; ręcznie `POST /api/v1/maintenance/backup`.
- Po stronie klienta (Rust): lokalny cache historii i ustawień w `%APPDATA%\QA Receiver Center` jako zapas, gdy usługa jest niedostępna; hasła nie trafiają do bazy.

## Narzędzia nawigacji

Wtyczki LSP (TypeScript, Rust, C#) są zainstalowane na poziomie użytkownika; używaj ich do definicji i referencji symboli zamiast czytania monolitów. Drugi przegląd przed wydaniem: `/codex:review --base main --background`.

## Główne domeny

- **Dashboard i odbiorniki:** pobieranie konfiguracji, skanowanie statusu i prezentacja metryk.
- **Firmware:** wybór katalogu lub plików lokalnych, weryfikacja pakietów, aktualizacje pojedyncze, grupowe i wielokrotne.
- **IP management:** inwentaryzacja adresów, przydziały, pingowanie i ustawienia monitoringu.
- **Historia i ustawienia:** synchronizacja historii, Clone Files oraz współdzielone ustawienia z kontrolą wersji.
- **Registry:** niezależna usługa HTTP z SQLite, audytem, backupami i migracjami.

## Najważniejsze przepływy

1. `src/main.tsx` montuje `App`; UI wywołuje komendy Tauri przez IPC.
2. `src/App.tsx` rozpoczyna skanowanie lub workflow firmware; `src-tauri/src/lib.rs` wykonuje operację i korzysta z usług wewnętrznych albo cache.
3. Aktualizacje firmware są rezerwowane przez `firmware_orchestrator.rs`, a UI odpytuje status zadania.
4. Katalog release jest pobierany z serwera przy wejściu w ekran wyboru i ręcznym odświeżeniu. Wspólny release jest dostępny w Alpha i Beta; po jego wyborze parser Rust filtruje buildy po nazwie kanału.
5. Dane współdzielone przechodzą z Rust do Registry API na porcie 8787; Registry zapisuje je w SQLite i wykonuje migracje.
6. Lista odbiorników i grup pochodzi z `qa-receivers.json` w Azure DevOps, a hasła nie są zapisywane w Registry.

## Komendy kontroli

Uruchamiaj z katalogu głównego repozytorium:

```powershell
npm.cmd run lint
npm.cmd run build
npm.cmd run test:logic
Set-Location src-tauri; cargo test; cargo check
Set-Location ..; dotnet build services/receiver-registry/QAReceiverRegistry.csproj --no-restore
```

Integrację Registry uruchamiaj przed publikacją pakietu:

```powershell
.\services\receiver-registry\scripts\Run-IntegrationTests.ps1
```

<!-- check-path: src/main.tsx -->
<!-- check-path: src/displayFormatters.ts -->
<!-- check-path: src/App.tsx -->
<!-- check-path: src-tauri/src/lib.rs -->
<!-- check-path: src-tauri/src/firmware_orchestrator.rs -->
<!-- check-path: services/receiver-registry/Program.cs -->
<!-- check-path: services/receiver-registry/Data/RegistryDatabase.cs -->
<!-- check-path: docs/PULAPKI.md -->
<!-- check-path: docs/AI-SESSION-HANDOFF.md -->
<!-- check-script: lint -->
<!-- check-script: build -->
<!-- check-script: test:logic -->

## Zasady struktury

- Najpierw używaj tej mapy, potem wyszukuj symbole i referencje w wskazanym obszarze.
- Nie dziel pliku według liczby linii; dziel dopiero, gdy granica domeny zmniejsza koszt typowego zadania.
- Zachowuj publiczne komendy IPC i kontrakty API, jeśli zmiana wewnętrzna nie wymaga ich modyfikacji.
- Aktualizuj mapę tylko po zmianie entry pointu, domeny, przepływu, głównego folderu lub komendy kontroli.
- Duże pliki opisane w `PULAPKI.md` nie są automatycznie kandydatami do podziału.
- Bieżący zakres i decyzje prowadź w `PLAN-BIEZACY.md`; nie powielaj ich w mapie.
