# Bieżący plan prac

Stan na 2026-10-22. Ten plik opisuje aktywny etap i decyzje potrzebne do jego kontynuacji; architektura jest w `MAPA-KODU.md`, trwałe pułapki w `PULAPKI.md`.

## Cel projektu

Przeniesienie QA Receiver Center z aplikacji desktopowej (Tauri) na aplikację webową. Serwer działa na nowym PC z Windows 11 w LAN biura (stały adres IP), użytkownicy pracują w przeglądarce, aktualizacje wdraża tylko właściciel. Kod powstaje na PC deweloperskim; na serwer trafia gotowa paczka.

Projekt startuje jako kopia repo desktopowego z historią (`756d7c9`). Repo desktopowe (`../qa-receiver-center`, remote `desktop`) zostaje do poprawek w trakcie wdrożenia; po przełączeniu jest zamrożone. Poprawki z desktopu przenoś przez `git fetch desktop` + `git cherry-pick`.

## Architektura docelowa (uzgodniona)

- Rdzeń Rust z `src-tauri/src/lib.rs` jako biblioteka bez Tauri; serwer HTTP (`axum`) wystawia komendy jako `POST /api/cmd/<nazwa>` z tym samym JSON, serwuje zbudowany React, `/health`, `/api/version`.
- UI: adapter `src/api.ts` zamiast `invoke`; usunięcie strażników `isTauri()`.
- Registry .NET na tym samym serwerze, nasłuch tylko `127.0.0.1:8787`.
- Konta zakładane przez właściciela, role admin/operator/viewer, hasła jako hash w SQLite Registry (migracja 009), cookie sesji `HttpOnly`/`SameSite=Strict`. Stałe `admin123` znikają z kodu.
- Stan wspólny: dane z `%APPDATA%` przechodzą do `data\` serwera; grupy Alpha/Beta wspólne dla wszystkich (zaakceptowane); joby firmware widoczne dla wszystkich, z właścicielem.
- Pakiety firmware (ok. 20 MB) przez upload, Clone File przez pobranie w przeglądarce, „Otwórz odbiornik” przez `window.open` (PC użytkowników widzą odbiorniki w sieci).
- Serwer: `C:\TSQATools\QAReceiverCenter\{releases, current, config, data, logs}`; `config` i `data` poza wersją.

## Etapy

0. **Przygotowanie** (aktywny): repo i dokumenty gotowe; zgłoszenie do IT w `docs/ZGLOSZENIE-IT.md` czeka na wysłanie.
1. Szkielet serwera: rdzeń jako biblioteka, `axum`, `/api/cmd`, adapter UI. Wynik: aplikacja w przeglądarce na `localhost`.
2. Zamiana funkcji desktopowych: upload pakietów, pobieranie Clone File, otwieranie odbiornika, dźwięk, katalog `data\`.
3. Konta i bezpieczeństwo: migracja 009, logowanie, role, zarządzanie kontami, CSP, limit prób, audyt z loginem.
4. Wielu użytkowników: właściciel jobu, downgrade i stop dla właściciela/admina, tryb serwisowy, lista aktywnych jobów.
5. Pakowanie: `Build-WebRelease.ps1` (zip + SHA-256), `Install-Server.ps1` (pierwsza instalacja), `Install-Update.ps1` (blokada przy aktywnych jobach, backup, health check, automatyczny rollback); komunikat „nowa wersja, odśwież”.
6. Testy: kontrole automatyczne, integracja Registry, test lokalny, instalacja na serwerze, pełna aktualizacja na żywym odbiorniku.
7. Pilotaż (1-2 osoby), kopia `receivers.db` ze starego serwera, przełączenie wszystkich, zamrożenie desktopu.

## Otwarte kwestie

- Od IT: konto domenowe usługi (ADO + `\\det-fileserver`), certyfikat z firmowego CA, nazwa DNS, firewall 443. Do czasu certyfikatu pilotaż może iść po HTTP w LAN; pełne wdrożenie tylko HTTPS.
- Etap 7: po przeniesieniu danych desktop nie może dalej pisać do starego Registry (rozjazd historii).
- Repo w ADO dla wersji web: opcjonalnie później.

## Uwaga o wersjonowaniu

W tym repo dokumenty AI (`AGENTS.md`, `CLAUDE.md`, `.ignore`, `docs/*`, `.github/copilot-instructions.md`, `scripts/Check-RepositoryDocs.ps1`) są wersjonowane normalnie (`origin` = prywatne GitHub). Lokalny pozostaje tylko `.claude/settings.local.json` (ignorowany przez `*.local.json`).
