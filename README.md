DarkClient

DarkClient to framework napisany w Rust, który komunikuje się z uruchomioną instancją Minecrafta przez JNI. Projekt skupia się na wydajności, modularnej architekturze i analizie działania JVM w czasie rzeczywistym.

Nie jest to kolejny projekt z przeładowanym README. Celem jest dostarczenie solidnej bazy kodu, którą można rozwijać, testować i dostosowywać do własnych potrzeb.

Założenia

- Rust jako główny język projektu
- Bezpośrednia komunikacja z JVM przez JNI
- Modułowa architektura
- Dynamiczne ładowanie funkcjonalności
- Rozbudowany system konfiguracji
- Wsparcie dla wielu profili użytkownika
- Nowoczesny interfejs oparty o egui

Struktura projektu

DarkClient/
├── protocol/
├── injector/
├── agent_loader/
├── client/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── state.rs
│   │   ├── config.rs
│   │   ├── mapping/
│   │   ├── graphic/
│   │   ├── module/
│   │   └── net/
├── mapping_derive/
├── xtask/
├── mappings.json
└── conversion.py

Główne elementy

Injector

Odpowiada za wykrywanie procesów Java, wybór instancji Minecrafta oraz uruchomienie procesu ładowania.

Agent Loader

Warstwa pośrednia działająca wewnątrz JVM. Zarządza komunikacją, stanem klienta oraz przeładowywaniem komponentów.

Client

Główna biblioteka projektu. Zawiera:

- system modułów
- konfigurację
- obsługę mapowań
- interfejs użytkownika
- warstwę sieciową
- rejestrację funkcjonalności

Mapping System

Mapowania JVM są oddzielone od logiki projektu, co upraszcza aktualizacje pomiędzy wersjami Minecrafta.

Wymagania

- Rust 1.95+
- JDK 21+
- Minecraft Java Edition

Kompilacja

Sklonuj repozytorium:

git clone https://github.com/meklasdev/uddcmc.git
cd uddcmc

Wygeneruj mapowania:

python conversion.py

Zbuduj projekt:

cargo build --release

Uruchomienie

1. Włącz Minecraft.
2. Uruchom injector.
3. Wybierz proces.
4. Rozpocznij ładowanie.

./target/release/injector

Tworzenie modułów

Nowe funkcjonalności dodaje się przez implementację traitu "Module".

Po zarejestrowaniu modułu w "register_modules()" zostanie on automatycznie uwzględniony przez system konfiguracji i interfejs użytkownika.

Roadmap

- ukończenie edytora HUD
- system automatycznych aktualizacji
- API Lua
- rozszerzalny system skryptów
- dodatkowe narzędzia developerskie

Licencja

Projekt udostępniany jest na licencji GPL-3.0. Szczegóły znajdują się w pliku LICENSE.