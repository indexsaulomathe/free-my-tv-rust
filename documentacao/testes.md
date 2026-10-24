# Testes

```bash
tests/run.sh            # tudo: Python, web e Rust
tests/run.sh python     # só um conjunto: python, web ou rust
```

Requisitos: Python 3 com `pytest`, Node 18+ e Rust (cargo).

## `tests/python/` (pytest, 60 testes)

| Arquivo | O que cobre |
|---|---|
| `test_webapp.py` | `config.properties` (padrões, comentários, aspas, validação), `--generic`/`--config`, logos das moedas, catálogos, arquivos do app, preview |
| `test_packages.py` | Formato `ar` e `tar.gz` do `.ipk`, membros, `control`, `packageinfo.json`, ordem das pastas; `.wgt`; assinatura sem o CLI `tizen` |
| `test_tools.py` | `homebrew_repo` (JSON, SHA-256, uso errado), `make_zones` (graus, banco de fusos de exemplo, formato do `zones.js` versionado), `translate` (argumentos, placeholders, padrões de data, plurais, catálogos do repositório) |
| `test_webos_install.py` | Instalador LG sem TV: caminho da chave, opções do SSH, comando Luna, reuso da chave, Key Server fora do ar, IP inválido. Documenta o risco da host key não verificada |
| `test_rust_parity.py` | O Rust gera os mesmos arquivos que o Python (`.ipk` genérico e com config, `.wgt`, `apps.json`, `zones.js`). Pulado se o binário não estiver compilado |

Os testes nunca escrevem no repositório: os caminhos de saída são trocados por pastas temporárias.

## `tests/web/` (node:test, 23 testes)

`harness.js` carrega `web/*.js` num contexto `vm` do Node que imita o navegador da TV (`window`, `localStorage`,
`navigator`, `document`, fuso horário) e um `XMLHttpRequest` falso que não acessa a rede.

| Arquivo | O que cobre |
|---|---|
| `strings.test.js` | `tr`, `trf`, `plural` (inglês e russo), idioma automático, `formatDate`, lista de idiomas |
| `data.test.js` | `Place` (cidade pelo fuso, apelidos, Fahrenheit, moeda, `config.properties` vencendo), `Prefs` (salvar, storage corrompido), formatação (`compact`, `money`, `change`, `degrees`, `condition`, `symbol`, `isNight`, `staleness`, `image`), requisições só HTTPS e sem TMDB sem chave, falha de rede, `TV` no modo navegador |

## `rust/` (cargo test, 17 testes)

Testes unitários em cada módulo do `freemytv-core`: JSON idêntico ao Python, config, extração i18n, `ar`/`tar.gz`,
SHA-256 e fusos.

## `tests/bench/bench.py`

Benchmark Python × Rust. Veja os resultados em [rust.md](rust.md#benchmark-fase-1).

## O que não tem teste automatizado

- **App Android (Java):** precisa do Android SDK (`android.jar`), que não está neste ambiente. A lógica pura vai
  ganhar testes quando for convertida (issue 15).
- **`home.js` (interface web):** depende do DOM. Fica para a fase 3.
- **Instalação real na TV:** depende da TV na rede local. Teste manual: `python3 webos/install.py <IP>`.
