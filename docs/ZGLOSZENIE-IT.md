# Zgłoszenie do IT: serwer QA Receiver Center Web

Szkic do wysłania przez właściciela projektu. Uzupełnij pola w `<...>` przed wysłaniem.

---

**Temat:** Serwer wewnętrznej aplikacji QA Receiver Center: konto usługi, certyfikat, DNS, firewall

Dzień dobry,

przygotowuję wewnętrzną aplikację webową QA Receiver Center (obsługa odbiorników QA w laboratorium). Będzie działać jako usługa Windows na komputerze `<nazwa komputera>` (Windows 11, stały adres `<adres IP>`), dostępna wyłącznie z sieci biura przez przeglądarkę. Proszę o:

1. **Konto domenowe usługi** (np. `svc-qarc`), które:
   - ma odczyt repozytorium Azure DevOps `https://det-ados/tfs/Terrasat/QA/_git/...` (plik `qa-receivers.json`),
   - ma odczyt udziału `\\det-fileserver\TeamBuilds\QA\rx-monitoring-dhs`,
   - może działać jako usługa (`Log on as a service`) na `<nazwa komputera>`,
   - ma hasło bez wygasania albo wskazany sposób rotacji.
2. **Certyfikat TLS z firmowego urzędu certyfikacji** dla nazwy `<qarc.domena>` (oraz nazwy komputera), z kluczem prywatnym w magazynie `LocalMachine\My` na `<nazwa komputera>`, tak aby komputery w domenie ufały mu bez ostrzeżeń.
3. **Wpis DNS** `<qarc.domena>` -> `<adres IP>`.
4. **Regułę firewall** na `<nazwa komputera>`: przychodzący TCP 443 z sieci biura (profil Domain).
5. Potwierdzenie, że komputer może działać bez przerwy (zasilanie, uśpienie, okna aktualizacji Windows).

Aplikacja ma własne konta użytkowników; nie potrzebuje uprawnień administratora domeny ani dostępu z internetu.

Pozdrawiam,
`<podpis>`
